#[cfg(feature = "cuda")]
mod imp {
    use crate::gpu::{build_in_one_weekend_scene, CameraUniform, MaterialGpu, SphereGpu};
    use crate::render_io::write_ppm_from_accum;
    use cudarc::driver::{CudaDevice, DeviceRepr, LaunchAsync};
    use cudarc::nvrtc::compile_ptx;

    const CUDA_SPP_PER_PASS: u32 = 256;

    unsafe impl DeviceRepr for CameraUniform {}
    unsafe impl DeviceRepr for SphereGpu {}
    unsafe impl DeviceRepr for MaterialGpu {}

    const CUDA_SOURCE: &str = r#"
extern "C" {
struct float3 { float x; float y; float z; };
struct float4 { float x; float y; float z; float w; };
struct uint4 { unsigned int x; unsigned int y; unsigned int z; unsigned int w; };

struct Camera {
    float4 origin;
    float4 pixel00;
    float4 pixel_delta_u;
    float4 pixel_delta_v;
    float4 u;
    float4 v;
    float4 background;
    float4 params_f;
    uint4 params_u;
};

struct Sphere {
    float4 center_radius;
    unsigned int material_index;
    unsigned int _pad0;
    unsigned int _pad1;
    unsigned int _pad2;
};

struct Material {
    float4 albedo_fuzz;
    unsigned int kind;
    float ref_idx;
    unsigned int _pad0;
    unsigned int _pad1;
};

struct Ray {
    float3 origin;
    float3 direction;
};

struct Hit {
    int hit;
    float t;
    float3 p;
    float3 normal;
    int front_face;
    unsigned int mat_index;
};

__device__ __forceinline__ float3 make_float3(float x, float y, float z) {
    float3 v; v.x = x; v.y = y; v.z = z; return v;
}

__device__ __forceinline__ float4 make_float4(float x, float y, float z, float w) {
    float4 v; v.x = x; v.y = y; v.z = z; v.w = w; return v;
}

__device__ __forceinline__ float3 xyz(const float4& v) {
    return make_float3(v.x, v.y, v.z);
}

__device__ __forceinline__ float3 add3(float3 a, float3 b) {
    return make_float3(a.x + b.x, a.y + b.y, a.z + b.z);
}

__device__ __forceinline__ float3 sub3(float3 a, float3 b) {
    return make_float3(a.x - b.x, a.y - b.y, a.z - b.z);
}

__device__ __forceinline__ float3 mul3(float3 a, float b) {
    return make_float3(a.x * b, a.y * b, a.z * b);
}

__device__ __forceinline__ float dot3(float3 a, float3 b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

__device__ __forceinline__ float3 normalize3(float3 v) {
    float inv = rsqrtf(dot3(v, v));
    return mul3(v, inv);
}

__device__ __forceinline__ float3 cross3(float3 a, float3 b) {
    return make_float3(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x
    );
}

__device__ __forceinline__ float3 reflect3(float3 v, float3 n) {
    return sub3(v, mul3(n, 2.0f * dot3(v, n)));
}

__device__ __forceinline__ float3 refract3(float3 uv, float3 n, float etai_over_etat) {
    float cos_theta = fminf(dot3(make_float3(-uv.x, -uv.y, -uv.z), n), 1.0f);
    float3 r_out_perp = mul3(add3(uv, mul3(n, cos_theta)), etai_over_etat);
    float k = 1.0f - dot3(r_out_perp, r_out_perp);
    float3 r_out_parallel = mul3(n, -sqrtf(fabsf(k)));
    return add3(r_out_perp, r_out_parallel);
}

__device__ __forceinline__ float reflectance(float cosine, float ref_idx) {
    float r0 = (1.0f - ref_idx) / (1.0f + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0f - r0) * powf(1.0f - cosine, 5.0f);
}

__device__ __forceinline__ unsigned int rng_hash(unsigned int x) {
    unsigned int v = x;
    v ^= v >> 16;
    v *= 0x7feb352d;
    v ^= v >> 15;
    v *= 0x846ca68b;
    v ^= v >> 16;
    return v;
}

__device__ __forceinline__ unsigned int rng_init(unsigned int x, unsigned int y, unsigned int seed) {
    return rng_hash(x * 1973u + y * 9277u + seed * 26699u + 9119u);
}

__device__ __forceinline__ unsigned int rng_next(unsigned int& state) {
    unsigned int x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    state = x;
    return x;
}

__device__ __forceinline__ float rand_f(unsigned int& state) {
    return (float)rng_next(state) * (1.0f / 4294967296.0f);
}

__device__ __forceinline__ float3 random_unit_vector(unsigned int& state) {
    const float PI = 3.14159265359f;
    float a = rand_f(state) * 2.0f * PI;
    float z = rand_f(state) * 2.0f - 1.0f;
    float r = sqrtf(fmaxf(0.0f, 1.0f - z * z));
    return make_float3(r * cosf(a), r * sinf(a), z);
}

__device__ __forceinline__ float3 random_in_unit_disk(unsigned int& state) {
    const float PI = 3.14159265359f;
    float r = sqrtf(rand_f(state));
    float theta = rand_f(state) * 2.0f * PI;
    return make_float3(r * cosf(theta), r * sinf(theta), 0.0f);
}

__device__ __forceinline__ Ray get_ray(const Camera& camera, float u, float v, unsigned int& state) {
    float3 pixel = add3(
        add3(xyz(camera.pixel00), mul3(xyz(camera.pixel_delta_u), u)),
        mul3(xyz(camera.pixel_delta_v), v)
    );
    float3 origin = xyz(camera.origin);
    float lens_radius = camera.params_f.x;
    if (lens_radius > 0.0f) {
        float3 rd = mul3(random_in_unit_disk(state), lens_radius);
        float3 offset = add3(mul3(xyz(camera.u), rd.x), mul3(xyz(camera.v), rd.y));
        origin = add3(origin, offset);
    }
    Ray ray;
    ray.origin = origin;
    ray.direction = sub3(pixel, origin);
    return ray;
}

__device__ __forceinline__ Hit hit_spheres(const Ray& ray, const Sphere* spheres, unsigned int sphere_count, float t_min, float t_max) {
    Hit record;
    record.hit = 0;
    float closest = t_max;

    for (unsigned int i = 0; i < sphere_count; ++i) {
        Sphere sphere = spheres[i];
        float3 center = xyz(sphere.center_radius);
        float radius = sphere.center_radius.w;
        float3 oc = sub3(center, ray.origin);
        float a = dot3(ray.direction, ray.direction);
        float h = dot3(ray.direction, oc);
        float c = dot3(oc, oc) - radius * radius;
        float discriminant = h * h - a * c;
        if (discriminant > 0.0f) {
            float sqrtd = sqrtf(discriminant);
            float root = (h - sqrtd) / a;
            if (root < t_min || root > closest) {
                root = (h + sqrtd) / a;
            }
            if (root >= t_min && root <= closest) {
                closest = root;
                float3 p = add3(ray.origin, mul3(ray.direction, root));
                float3 outward = mul3(sub3(p, center), 1.0f / radius);
                int front = dot3(ray.direction, outward) < 0.0f;
                float3 normal = front ? outward : mul3(outward, -1.0f);

                record.hit = 1;
                record.t = root;
                record.p = p;
                record.normal = normal;
                record.front_face = front;
                record.mat_index = sphere.material_index;
            }
        }
    }
    return record;
}

__device__ __forceinline__ float3 ray_color(const Ray& ray_in, const Camera& camera, const Sphere* spheres, unsigned int sphere_count, const Material* materials, unsigned int& state) {
    Ray ray = ray_in;
    float3 attenuation = make_float3(1.0f, 1.0f, 1.0f);
    unsigned int max_depth = camera.params_u.x;

    for (unsigned int depth = 0; depth < max_depth; ++depth) {
        Hit hit = hit_spheres(ray, spheres, sphere_count, 0.001f, 1e9f);
        if (hit.hit) {
            Material mat = materials[hit.mat_index];
            if (mat.kind == 0u) {
                float3 scatter_dir = add3(hit.normal, random_unit_vector(state));
                if (dot3(scatter_dir, scatter_dir) < 1e-8f) {
                    scatter_dir = hit.normal;
                }
                ray.origin = hit.p;
                ray.direction = scatter_dir;
                attenuation = make_float3(
                    attenuation.x * mat.albedo_fuzz.x,
                    attenuation.y * mat.albedo_fuzz.y,
                    attenuation.z * mat.albedo_fuzz.z
                );
            } else if (mat.kind == 1u) {
                float3 reflected = reflect3(normalize3(ray.direction), hit.normal);
                float fuzz = mat.albedo_fuzz.w;
                float3 scattered = add3(reflected, mul3(random_unit_vector(state), fuzz));
                if (dot3(scattered, hit.normal) <= 0.0f) {
                    return make_float3(0.0f, 0.0f, 0.0f);
                }
                ray.origin = hit.p;
                ray.direction = scattered;
                attenuation = make_float3(
                    attenuation.x * mat.albedo_fuzz.x,
                    attenuation.y * mat.albedo_fuzz.y,
                    attenuation.z * mat.albedo_fuzz.z
                );
            } else {
                float ref_idx = mat.ref_idx;
                float etai_over_etat = hit.front_face ? (1.0f / ref_idx) : ref_idx;
                float3 unit_dir = normalize3(ray.direction);
                float cos_theta = fminf(dot3(make_float3(-unit_dir.x, -unit_dir.y, -unit_dir.z), hit.normal), 1.0f);
                float sin_theta = sqrtf(fmaxf(0.0f, 1.0f - cos_theta * cos_theta));
                int cannot_refract = etai_over_etat * sin_theta > 1.0f;

                float3 direction;
                if (cannot_refract || reflectance(cos_theta, etai_over_etat) > rand_f(state)) {
                    direction = reflect3(unit_dir, hit.normal);
                } else {
                    direction = refract3(unit_dir, hit.normal, etai_over_etat);
                }
                ray.origin = hit.p;
                ray.direction = direction;
            }

            if (depth >= 5u) {
                float p = fmaxf(attenuation.x, fmaxf(attenuation.y, attenuation.z));
                p = fminf(fmaxf(p, 0.05f), 0.95f);
                if (rand_f(state) > p) {
                    return make_float3(0.0f, 0.0f, 0.0f);
                }
                attenuation = mul3(attenuation, 1.0f / p);
            }
        } else {
            if (camera.params_u.w == 1u) {
                float3 bg = xyz(camera.background);
                return make_float3(
                    attenuation.x * bg.x,
                    attenuation.y * bg.y,
                    attenuation.z * bg.z
                );
            }
            float3 unit_dir = normalize3(ray.direction);
            float t = 0.5f * (unit_dir.y + 1.0f);
            float3 background = add3(
                mul3(make_float3(1.0f, 1.0f, 1.0f), (1.0f - t)),
                mul3(make_float3(0.5f, 0.7f, 1.0f), t)
            );
            return make_float3(
                attenuation.x * background.x,
                attenuation.y * background.y,
                attenuation.z * background.z
            );
        }
    }
    return make_float3(0.0f, 0.0f, 0.0f);
}

__global__ void render(
    Camera camera,
    const Sphere* spheres,
    unsigned int sphere_count,
    const Material* materials,
    float4* accum,
    unsigned int seed,
    unsigned int spp,
    unsigned int width,
    unsigned int height
) {
    unsigned int x = (unsigned int)(blockIdx.x * blockDim.x + threadIdx.x);
    unsigned int y = (unsigned int)(blockIdx.y * blockDim.y + threadIdx.y);
    if (x >= width || y >= height) {
        return;
    }

    unsigned int rng = rng_init(x, y, seed);
    float3 color = make_float3(0.0f, 0.0f, 0.0f);
    for (unsigned int s = 0; s < spp; ++s) {
        float u = (float)x + rand_f(rng);
        float v = (float)y + rand_f(rng);
        Ray ray = get_ray(camera, u, v, rng);
        float3 c = ray_color(ray, camera, spheres, sphere_count, materials, rng);
        color = add3(color, c);
    }

    unsigned int idx = y * width + x;
    float4 prev = accum[idx];
    accum[idx] = make_float4(prev.x + color.x, prev.y + color.y, prev.z + color.z, prev.w + (float)spp);
}
} // extern "C"
"#;

    pub fn render_in_one_weekend() -> Result<(), String> {
        let (camera, spheres, materials) = build_in_one_weekend_scene();
        render(camera, &spheres, &materials)
    }

    fn render(camera: CameraUniform, spheres: &[SphereGpu], materials: &[MaterialGpu]) -> Result<(), String> {
        let dev = CudaDevice::new(0).map_err(|e| format!("cuda init failed: {e:?}"))?;
        let ptx = compile_ptx(CUDA_SOURCE, &["--std=c++14"]).map_err(|e| format!("nvrtc failed: {e:?}"))?;
        let module = dev
            .load_ptx(ptx, "pathtracer", &["render"])
            .map_err(|e| format!("load_ptx failed: {e:?}"))?;
        let func = module.get_func("render").map_err(|e| format!("get_func failed: {e:?}"))?;
        let stream = dev.default_stream();

        let width = camera.params_f[1] as u32;
        let height = camera.params_f[2] as u32;
        let pixel_count = width as usize * height as usize;
        let total_spp = camera.params_f[3].max(1.0) as u32;
        let spp_per_pass = CUDA_SPP_PER_PASS.min(total_spp);
        let pass_count = (total_spp + spp_per_pass - 1) / spp_per_pass;
        let base_seed = camera.params_u[1];

        let d_spheres = dev.htod_sync_copy(spheres).map_err(|e| format!("copy spheres failed: {e:?}"))?;
        let d_materials = dev.htod_sync_copy(materials).map_err(|e| format!("copy materials failed: {e:?}"))?;
        let mut d_accum = dev.alloc_zeros::<f32>(pixel_count * 4).map_err(|e| format!("alloc accum failed: {e:?}"))?;

        let block_x = 8u32;
        let block_y = 8u32;
        let grid_x = (width + block_x - 1) / block_x;
        let grid_y = (height + block_y - 1) / block_y;

        for pass_index in 0..pass_count {
            let remaining = total_spp - pass_index * spp_per_pass;
            let pass_spp = remaining.min(spp_per_pass);
            let seed = base_seed ^ ((pass_index as u32).wrapping_mul(0x9E3779B9));

            unsafe {
                cudarc::driver::launch!(
                    func<<<(grid_x, grid_y, 1), (block_x, block_y, 1), 0, stream>>>(
                        camera,
                        &d_spheres,
                        spheres.len() as u32,
                        &d_materials,
                        &mut d_accum,
                        seed,
                        pass_spp,
                        width,
                        height
                    )
                )
                .map_err(|e| format!("kernel launch failed: {e:?}"))?;
            }
            stream.synchronize().map_err(|e| format!("cuda sync failed: {e:?}"))?;
            let done = pass_index + 1;
            let pct = (done as f64 / pass_count as f64) * 100.0;
            eprint!("\rCUDA progress: {}/{} ({:.1}%)", done, pass_count, pct);
            if done == pass_count {
                eprintln!();
            }
        }

        let mut accum = vec![0.0f32; pixel_count * 4];
        dev.dtoh_sync_copy_into(&d_accum, &mut accum)
            .map_err(|e| format!("copy back failed: {e:?}"))?;

        write_ppm_from_accum(width as usize, height as usize, &accum, total_spp)
    }
}

#[cfg(feature = "cuda")]
pub fn render_in_one_weekend() -> Result<(), String> {
    imp::render_in_one_weekend()
}

#[cfg(not(feature = "cuda"))]
pub fn render_in_one_weekend() -> Result<(), String> {
    Err("CUDA backend not enabled. Rebuild with --features cuda (or cuda-12000, cuda-11080, etc).".to_string())
}
