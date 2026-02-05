use crate::render_io::write_ppm_from_accum;
use std::time::Instant;
use bytemuck::{Pod, Zeroable};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use wgpu::util::DeviceExt;

use crate::config::OVERRIDES;

const WORKGROUP_SIZE: u32 = 8;
const GPU_SPP_PER_PASS: u32 = 64;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct CameraUniform {
    origin: [f32; 4],
    pixel00: [f32; 4],
    pixel_delta_u: [f32; 4],
    pixel_delta_v: [f32; 4],
    u: [f32; 4],
    v: [f32; 4],
    background: [f32; 4],
    pub(crate) params_f: [f32; 4],
    pub(crate) params_u: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SphereGpu {
    center_radius: [f32; 4],
    material_index: u32,
    _pad: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct MaterialGpu {
    albedo_fuzz: [f32; 4],
    kind: u32,
    ref_idx: f32,
    _pad: [u32; 2],
}

#[derive(Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn unit_vector(v: Vec3) -> Vec3 {
    v / v.length()
}

fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

fn add_material(materials: &mut Vec<MaterialGpu>, kind: u32, albedo: [f32; 3], fuzz: f32, ref_idx: f32) -> u32 {
    let index = materials.len() as u32;
    materials.push(MaterialGpu {
        albedo_fuzz: [albedo[0], albedo[1], albedo[2], fuzz],
        kind,
        ref_idx,
        _pad: [0; 2],
    });
    index
}

pub(crate) fn build_in_one_weekend_scene() -> (CameraUniform, Vec<SphereGpu>, Vec<MaterialGpu>) {
    let mut aspect_ratio = 16.0 / 9.0;
    let mut image_width = 1200;
    let mut samples_per_pixel = 10;
    let mut max_depth = 20;
    let mut vfov = 20.0;
    let mut lookfrom = Vec3::new(13.0, 2.0, 3.0);
    let mut lookat = Vec3::new(0.0, 0.0, 0.0);
    let mut vup = Vec3::new(0.0, 1.0, 0.0);
    let mut defocus_angle = 0.6;
    let mut focus_dist = 10.0;
    let mut background = [0.0, 0.0, 0.0];
    let mut background_mode = 0u32;

    let o = OVERRIDES;
    if let Some(value) = o.aspect_ratio {
        aspect_ratio = value;
    }
    if let Some(value) = o.image_width {
        image_width = value;
    }
    if let Some(value) = o.samples_per_pixel {
        samples_per_pixel = value;
    }
    if let Some(value) = o.max_depth {
        max_depth = value;
    }
    if let Some(value) = o.vfov {
        vfov = value;
    }
    if let Some(value) = o.lookfrom {
        lookfrom = Vec3::new(value[0], value[1], value[2]);
    }
    if let Some(value) = o.lookat {
        lookat = Vec3::new(value[0], value[1], value[2]);
    }
    if let Some(value) = o.vup {
        vup = Vec3::new(value[0], value[1], value[2]);
    }
    if let Some(value) = o.defocus_angle {
        defocus_angle = value;
    }
    if let Some(value) = o.focus_dist {
        focus_dist = value;
    }
    if let Some(value) = o.background {
        background = [value[0] as f32, value[1] as f32, value[2] as f32];
        background_mode = 1;
    }

    let mut image_height = (image_width as f64 / aspect_ratio) as i32;
    if image_height < 1 {
        image_height = 1;
    }

    let theta = degrees_to_radians(vfov);
    let h = (theta / 2.0).tan();
    let viewport_height = 2.0 * h * focus_dist;
    let viewport_width = viewport_height * (image_width as f64 / image_height as f64);

    let w = unit_vector(lookfrom - lookat);
    let u = unit_vector(cross(vup, w));
    let v = cross(w, u);

    let viewport_u = u * viewport_width;
    let viewport_v = v * -viewport_height;

    let pixel_delta_u = viewport_u / image_width as f64;
    let pixel_delta_v = viewport_v / image_height as f64;

    let viewport_upper_left = lookfrom - (w * focus_dist) - viewport_u / 2.0 - viewport_v / 2.0;
    let pixel00 = viewport_upper_left + (pixel_delta_u + pixel_delta_v) * 0.5;

    let defocus_radius = focus_dist * (degrees_to_radians(defocus_angle / 2.0)).tan();

    let mut rng = SmallRng::seed_from_u64(0x5EED_1234);
    let mut materials = Vec::new();
    let mut spheres = Vec::new();

    let ground_mat = add_material(&mut materials, 0, [0.5, 0.5, 0.5], 0.0, 1.0);
    spheres.push(SphereGpu {
        center_radius: [0.0, -1000.0, 0.0, 1000.0],
        material_index: ground_mat,
        _pad: [0; 3],
    });

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat: f32 = rng.r#gen();
            let center = Vec3::new(
                a as f64 + 0.9 * rng.r#gen::<f64>(),
                0.2,
                b as f64 + 0.9 * rng.r#gen::<f64>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                if choose_mat < 0.8 {
                    let albedo = [
                        (rng.r#gen::<f32>() * rng.r#gen::<f32>()),
                        (rng.r#gen::<f32>() * rng.r#gen::<f32>()),
                        (rng.r#gen::<f32>() * rng.r#gen::<f32>()),
                    ];
                    let mat = add_material(&mut materials, 0, albedo, 0.0, 1.0);
                    spheres.push(SphereGpu {
                        center_radius: [center.x as f32, center.y as f32, center.z as f32, 0.2],
                        material_index: mat,
                        _pad: [0; 3],
                    });
                } else if choose_mat < 0.95 {
                    let albedo = [
                        rng.gen_range(0.5..1.0),
                        rng.gen_range(0.5..1.0),
                        rng.gen_range(0.5..1.0),
                    ];
                    let fuzz = rng.r#gen::<f32>() * 0.5;
                    let mat = add_material(&mut materials, 1, albedo, fuzz, 1.0);
                    spheres.push(SphereGpu {
                        center_radius: [center.x as f32, center.y as f32, center.z as f32, 0.2],
                        material_index: mat,
                        _pad: [0; 3],
                    });
                } else {
                    let mat = add_material(&mut materials, 2, [1.0, 1.0, 1.0], 0.0, 1.5);
                    spheres.push(SphereGpu {
                        center_radius: [center.x as f32, center.y as f32, center.z as f32, 0.2],
                        material_index: mat,
                        _pad: [0; 3],
                    });
                }
            }
        }
    }

    let material1 = add_material(&mut materials, 2, [1.0, 1.0, 1.0], 0.0, 1.5);
    spheres.push(SphereGpu {
        center_radius: [0.0, 1.0, 0.0, 1.0],
        material_index: material1,
        _pad: [0; 3],
    });

    let material2 = add_material(&mut materials, 0, [0.4, 0.2, 0.1], 0.0, 1.0);
    spheres.push(SphereGpu {
        center_radius: [-4.0, 1.0, 0.0, 1.0],
        material_index: material2,
        _pad: [0; 3],
    });

    let material3 = add_material(&mut materials, 1, [0.7, 0.6, 0.5], 0.0, 1.0);
    spheres.push(SphereGpu {
        center_radius: [4.0, 1.0, 0.0, 1.0],
        material_index: material3,
        _pad: [0; 3],
    });

    let camera = CameraUniform {
        origin: [lookfrom.x as f32, lookfrom.y as f32, lookfrom.z as f32, 0.0],
        pixel00: [pixel00.x as f32, pixel00.y as f32, pixel00.z as f32, 0.0],
        pixel_delta_u: [pixel_delta_u.x as f32, pixel_delta_u.y as f32, pixel_delta_u.z as f32, 0.0],
        pixel_delta_v: [pixel_delta_v.x as f32, pixel_delta_v.y as f32, pixel_delta_v.z as f32, 0.0],
        u: [u.x as f32, u.y as f32, u.z as f32, 0.0],
        v: [v.x as f32, v.y as f32, v.z as f32, 0.0],
        background: [background[0], background[1], background[2], 0.0],
        params_f: [
            defocus_radius as f32,
            image_width as f32,
            image_height as f32,
            samples_per_pixel as f32,
        ],
        params_u: [
            max_depth as u32,
            rng.r#gen(),
            spheres.len() as u32,
            background_mode,
        ],
    };

    (camera, spheres, materials)
}

pub fn render_in_one_weekend() -> Result<(), String> {
    let (camera, spheres, materials) = build_in_one_weekend_scene();
    pollster::block_on(render(camera, &spheres, &materials))
}

async fn render(mut camera: CameraUniform, spheres: &[SphereGpu], materials: &[MaterialGpu]) -> Result<(), String> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| "No compatible GPU adapter found".to_string())?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("wgpu-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .map_err(|e| format!("request_device failed: {e:?}"))?;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pathtracer"),
        source: wgpu::ShaderSource::Wgsl(include_str!("renderer.wgsl").into()),
    });

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("camera-buffer"),
        contents: bytemuck::bytes_of(&camera),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let spheres_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("spheres-buffer"),
        contents: bytemuck::cast_slice(spheres),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let materials_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("materials-buffer"),
        contents: bytemuck::cast_slice(materials),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let width = camera.params_f[1] as u32;
    let height = camera.params_f[2] as u32;
    let pixel_count = width as u64 * height as u64;
    let accum_size = pixel_count * std::mem::size_of::<[f32; 4]>() as u64;
    let zeroed = vec![0.0f32; pixel_count as usize * 4];
    let accum_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("accum-buffer"),
        contents: bytemuck::cast_slice(&zeroed),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    });

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback-buffer"),
        size: accum_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: spheres_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: materials_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: accum_buffer.as_entire_binding() },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("compute-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    let dispatch_x = (width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
    let dispatch_y = (height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
    let total_spp = camera.params_f[3].max(1.0) as u32;
    let spp_per_pass = GPU_SPP_PER_PASS.min(total_spp);
    let pass_count = (total_spp + spp_per_pass - 1) / spp_per_pass;
    let base_seed = camera.params_u[1];
    let start = Instant::now();
    for pass_index in 0..pass_count {
        let remaining = total_spp - pass_index * spp_per_pass;
        let pass_spp = remaining.min(spp_per_pass);
        camera.params_f[3] = pass_spp as f32;
        camera.params_u[1] = base_seed ^ ((pass_index as u32).wrapping_mul(0x9E3779B9));
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        }
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);

        let done = pass_index + 1;
        let pct = (done as f64 / pass_count as f64) * 100.0;
        let elapsed = start.elapsed().as_secs_f64();
        let avg = elapsed / done as f64;
        let eta = avg * (pass_count - done) as f64;
        eprint!(
            "\rGPU progress: {}/{} ({:.1}%) elapsed {:.1}s eta {:.1}s",
            done, pass_count, pct, elapsed, eta
        );
        if done == pass_count {
            eprintln!();
        }
    }

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("copy-encoder") });
    encoder.copy_buffer_to_buffer(&accum_buffer, 0, &readback_buffer, 0, accum_size);
    queue.submit(Some(encoder.finish()));

    let buffer_slice = readback_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| format!("map_async recv failed: {e:?}"))?
        .map_err(|e| format!("map_async failed: {e:?}"))?;

    let data = buffer_slice.get_mapped_range();
    let accum: &[f32] = bytemuck::cast_slice(&data);

    write_ppm_from_accum(width as usize, height as usize, accum, total_spp)?;

    drop(data);
    readback_buffer.unmap();

    Ok(())
}
