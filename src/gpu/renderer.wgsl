struct Camera {
    origin: vec4f,
    pixel00: vec4f,
    pixel_delta_u: vec4f,
    pixel_delta_v: vec4f,
    u: vec4f,
    v: vec4f,
    background: vec4f,
    params_f: vec4f,
    params_u: vec4u,
};

struct Sphere {
    center_radius: vec4f,
    material_index: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct Material {
    albedo_fuzz: vec4f,
    kind: u32,
    ref_idx: f32,
    _pad: vec2u,
};

struct Ray {
    origin: vec3f,
    direction: vec3f,
};

struct Hit {
    hit: bool,
    t: f32,
    p: vec3f,
    normal: vec3f,
    front_face: bool,
    mat_index: u32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> spheres: array<Sphere>;
@group(0) @binding(2) var<storage, read> materials: array<Material>;
@group(0) @binding(3) var<storage, read_write> accum: array<vec4f>;

const PI: f32 = 3.14159265359;
const DEBUG_GRADIENT: bool = false;
const DEBUG_PARAM: bool = false;

fn rng_hash(x: u32) -> u32 {
    var v = x;
    v ^= v >> 16u;
    v *= 0x7feb352du;
    v ^= v >> 15u;
    v *= 0x846ca68bu;
    v ^= v >> 16u;
    return v;
}

fn rng_init(x: u32, y: u32, seed: u32) -> u32 {
    return rng_hash(x * 1973u + y * 9277u + seed * 26699u + 9119u);
}

fn rng_next(state: ptr<function, u32>) -> u32 {
    var x = *state;
    x ^= x << 13u;
    x ^= x >> 17u;
    x ^= x << 5u;
    *state = x;
    return x;
}

fn rand_f(state: ptr<function, u32>) -> f32 {
    return f32(rng_next(state)) * (1.0 / 4294967296.0);
}

fn random_unit_vector(state: ptr<function, u32>) -> vec3f {
    let a = rand_f(state) * 2.0 * PI;
    let z = rand_f(state) * 2.0 - 1.0;
    let r = sqrt(max(0.0, 1.0 - z * z));
    return vec3f(r * cos(a), r * sin(a), z);
}

fn random_in_unit_disk(state: ptr<function, u32>) -> vec3f {
    let r = sqrt(rand_f(state));
    let theta = rand_f(state) * 2.0 * PI;
    return vec3f(r * cos(theta), r * sin(theta), 0.0);
}

fn reflect(v: vec3f, n: vec3f) -> vec3f {
    return v - 2.0 * dot(v, n) * n;
}

fn refract(uv: vec3f, n: vec3f, etai_over_etat: f32) -> vec3f {
    let cos_theta = min(dot(-uv, n), 1.0);
    let r_out_perp = etai_over_etat * (uv + cos_theta * n);
    let r_out_parallel = -sqrt(abs(1.0 - dot(r_out_perp, r_out_perp))) * n;
    return r_out_perp + r_out_parallel;
}

fn reflectance(cosine: f32, ref_idx: f32) -> f32 {
    var r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow(1.0 - cosine, 5.0);
}

fn hit_spheres(ray: Ray, t_min: f32, t_max: f32) -> Hit {
    var record: Hit;
    record.hit = false;
    var closest = t_max;

    var i: u32 = 0u;
    loop {
        if (i >= camera.params_u.z) {
            break;
        }

        let sphere = spheres[i];
        let center = sphere.center_radius.xyz;
        let radius = sphere.center_radius.w;
        let oc = center - ray.origin;
        let a = dot(ray.direction, ray.direction);
        let h = dot(ray.direction, oc);
        let c = dot(oc, oc) - radius * radius;
        let discriminant = h * h - a * c;

        if (discriminant > 0.0) {
            let sqrtd = sqrt(discriminant);
            var root = (h - sqrtd) / a;
            if (root < t_min || root > closest) {
                root = (h + sqrtd) / a;
            }

            if (root >= t_min && root <= closest) {
                closest = root;
                let p = ray.origin + root * ray.direction;
                let outward = (p - center) / radius;
                let front = dot(ray.direction, outward) < 0.0;
                let normal = select(-outward, outward, front);

                record.hit = true;
                record.t = root;
                record.p = p;
                record.normal = normal;
                record.front_face = front;
                record.mat_index = sphere.material_index;
            }
        }

        i += 1u;
    }

    return record;
}

fn get_ray(u: f32, v: f32, state: ptr<function, u32>) -> Ray {
    let pixel = camera.pixel00.xyz + u * camera.pixel_delta_u.xyz + v * camera.pixel_delta_v.xyz;
    var origin = camera.origin.xyz;

    let lens_radius = camera.params_f.x;
    if (lens_radius > 0.0) {
        let rd = lens_radius * random_in_unit_disk(state);
        let offset = camera.u.xyz * rd.x + camera.v.xyz * rd.y;
        origin = origin + offset;
    }

    let direction = pixel - origin;
    var ray: Ray;
    ray.origin = origin;
    ray.direction = direction;
    return ray;
}

fn ray_color(ray_in: Ray, state: ptr<function, u32>) -> vec3f {
    var ray = ray_in;
    var attenuation = vec3f(1.0, 1.0, 1.0);

    var depth: u32 = 0u;
    loop {
        if (depth >= camera.params_u.x) {
            return vec3f(0.0, 0.0, 0.0);
        }

        let hit = hit_spheres(ray, 0.001, 1e9);
        if (hit.hit) {
            let mat = materials[hit.mat_index];
            if (mat.kind == 0u) {
                var scatter_dir = hit.normal + random_unit_vector(state);
                if (length(scatter_dir) < 1e-8) {
                    scatter_dir = hit.normal;
                }
                ray.origin = hit.p;
                ray.direction = scatter_dir;
                attenuation *= mat.albedo_fuzz.xyz;
            } else if (mat.kind == 1u) {
                let reflected = reflect(normalize(ray.direction), hit.normal);
                let fuzz = mat.albedo_fuzz.w;
                let scattered = reflected + fuzz * random_unit_vector(state);
                if (dot(scattered, hit.normal) <= 0.0) {
                    return vec3f(0.0, 0.0, 0.0);
                }
                ray.origin = hit.p;
                ray.direction = scattered;
                attenuation *= mat.albedo_fuzz.xyz;
            } else {
                let ref_idx = mat.ref_idx;
                let etai_over_etat = select(ref_idx, 1.0 / ref_idx, hit.front_face);
                let unit_dir = normalize(ray.direction);
                let cos_theta = min(dot(-unit_dir, hit.normal), 1.0);
                let sin_theta = sqrt(max(0.0, 1.0 - cos_theta * cos_theta));
                let cannot_refract = etai_over_etat * sin_theta > 1.0;

                var direction: vec3f;
                if (cannot_refract || reflectance(cos_theta, etai_over_etat) > rand_f(state)) {
                    direction = reflect(unit_dir, hit.normal);
                } else {
                    direction = refract(unit_dir, hit.normal, etai_over_etat);
                }

                ray.origin = hit.p;
                ray.direction = direction;
            }

            if (depth >= 5u) {
                let p = clamp(max(attenuation.x, max(attenuation.y, attenuation.z)), 0.05, 0.95);
                if (rand_f(state) > p) {
                    return vec3f(0.0, 0.0, 0.0);
                }
                attenuation = attenuation / p;
            }
        } else {
            if (camera.params_u.w == 1u) {
                return attenuation * camera.background.xyz;
            }
            let unit_dir = normalize(ray.direction);
            let t = 0.5 * (unit_dir.y + 1.0);
            let background = (1.0 - t) * vec3f(1.0, 1.0, 1.0) + t * vec3f(0.5, 0.7, 1.0);
            return attenuation * background;
        }

        depth += 1u;
    }

    return vec3f(0.0, 0.0, 0.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    let width = u32(camera.params_f.y);
    let height = u32(camera.params_f.z);
    if (x >= width || y >= height) {
        return;
    }

    if (DEBUG_GRADIENT) {
        var rng = rng_init(x, y, camera.params_u.y);
        let u = f32(x) + 0.5;
        let v = f32(y) + 0.5;
        let ray = get_ray(u, v, &rng);
        let unit_dir = normalize(ray.direction);
        let t = 0.5 * (unit_dir.y + 1.0);
        let color = (1.0 - t) * vec3f(1.0, 1.0, 1.0) + t * vec3f(0.5, 0.7, 1.0);
        let idx = y * width + x;
        accum[idx] = vec4f(color, 1.0);
        return;
    }

    if (DEBUG_PARAM) {
        let width_f = camera.params_f.y;
        let height_f = camera.params_f.z;
        let color = vec3f(clamp(width_f / 1024.0, 0.0, 1.0), clamp(height_f / 1024.0, 0.0, 1.0), 0.0);
        let idx = y * width + x;
        accum[idx] = vec4f(color, 1.0);
        return;
    }

    var rng = rng_init(x, y, camera.params_u.y);
    var color = vec3f(0.0, 0.0, 0.0);

    var s: u32 = 0u;
    loop {
        if (s >= u32(camera.params_f.w)) {
            break;
        }
        let u = f32(x) + rand_f(&rng);
        let v = f32(y) + rand_f(&rng);
        let ray = get_ray(u, v, &rng);
        color += ray_color(ray, &rng);
        s += 1u;
    }

    let idx = y * width + x;
    let prev = accum[idx];
    accum[idx] = vec4f(prev.xyz + color, prev.w + f32(camera.params_f.w));
}
