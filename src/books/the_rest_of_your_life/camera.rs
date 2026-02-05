use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rayon::prelude::*;

use super::color::write_color;
use super::hittable::{Hittable, HittableRef};
use super::interval::Interval;
use super::pdf::{HittablePdf, MixturePdf, Pdf};
use super::ray::Ray;
use super::rtweekend::{degrees_to_radians, random_double, INFINITY};
use super::vec3::{
    cross, random_in_unit_disk, unit_vector, Color, Point3, Vec3,
};

pub struct Camera {
    pub aspect_ratio: f64,
    pub image_width: i32,
    pub samples_per_pixel: i32,
    pub max_depth: i32,
    pub background: Color,

    pub vfov: f64,
    pub lookfrom: Point3,
    pub lookat: Point3,
    pub vup: Vec3,

    pub defocus_angle: f64,
    pub focus_dist: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            aspect_ratio: 1.0,
            image_width: 100,
            samples_per_pixel: 10,
            max_depth: 10,
            background: Color::new(0.0, 0.0, 0.0),
            vfov: 90.0,
            lookfrom: Point3::new(0.0, 0.0, 0.0),
            lookat: Point3::new(0.0, 0.0, -1.0),
            vup: Vec3::new(0.0, 1.0, 0.0),
            defocus_angle: 0.0,
            focus_dist: 10.0,
        }
    }
}

struct CameraInternals {
    image_height: i32,
    pixel_samples_scale: f64,
    sqrt_spp: i32,
    recip_sqrt_spp: f64,
    center: Point3,
    pixel00_loc: Point3,
    pixel_delta_u: Vec3,
    pixel_delta_v: Vec3,
    defocus_disk_u: Vec3,
    defocus_disk_v: Vec3,
}

impl Camera {
    pub fn render<H: Hittable>(&self, world: &H, lights: HittableRef) {
        let data = self.initialize();

        let image_height = data.image_height as usize;
        let image_width = self.image_width as usize;
        let remaining = AtomicUsize::new(image_height);

        let rows: Vec<Vec<u8>> = (0..image_height)
            .into_par_iter()
            .map(|j| {
                let mut row = Vec::with_capacity(image_width * 12);
                let j_i32 = j as i32;
                for i in 0..self.image_width {
                    let mut pixel_color = Color::new(0.0, 0.0, 0.0);
                    for s_j in 0..data.sqrt_spp {
                        for s_i in 0..data.sqrt_spp {
                            let r = self.get_ray(i, j_i32, s_i, s_j, &data);
                            pixel_color += self.ray_color(r, self.max_depth, world, lights.clone());
                        }
                    }
                    write_color(&mut row, data.pixel_samples_scale * pixel_color);
                }

                let left = remaining.fetch_sub(1, Ordering::Relaxed) - 1;
                if left % 10 == 0 || left == 0 {
                    eprint!("\rScanlines remaining: {} ", left);
                    io::stderr().flush().ok();
                }

                row
            })
            .collect();

        let stdout = io::stdout();
        let mut out = BufWriter::new(stdout.lock());
        writeln!(out, "P3\n{} {}\n255", self.image_width, data.image_height)
            .expect("failed to write header");

        for row in rows {
            out.write_all(&row).expect("failed to write pixel data");
        }

        eprintln!("\rDone.                 ");
    }

    fn initialize(&self) -> CameraInternals {
        let mut image_height = (self.image_width as f64 / self.aspect_ratio) as i32;
        if image_height < 1 {
            image_height = 1;
        }

        let sqrt_spp = (self.samples_per_pixel as f64).sqrt() as i32;
        let pixel_samples_scale = 1.0 / (sqrt_spp * sqrt_spp) as f64;
        let recip_sqrt_spp = 1.0 / sqrt_spp as f64;

        let center = self.lookfrom;

        let theta = degrees_to_radians(self.vfov);
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h * self.focus_dist;
        let viewport_width = viewport_height * (self.image_width as f64 / image_height as f64);

        let w = unit_vector(self.lookfrom - self.lookat);
        let u = unit_vector(cross(self.vup, w));
        let v = cross(w, u);

        let viewport_u = viewport_width * u;
        let viewport_v = viewport_height * -v;

        let pixel_delta_u = viewport_u / self.image_width as f64;
        let pixel_delta_v = viewport_v / image_height as f64;

        let viewport_upper_left = center - (self.focus_dist * w) - viewport_u / 2.0 - viewport_v / 2.0;
        let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

        let defocus_radius = self.focus_dist * (degrees_to_radians(self.defocus_angle / 2.0)).tan();
        let defocus_disk_u = u * defocus_radius;
        let defocus_disk_v = v * defocus_radius;

        CameraInternals {
            image_height,
            pixel_samples_scale,
            sqrt_spp,
            recip_sqrt_spp,
            center,
            pixel00_loc,
            pixel_delta_u,
            pixel_delta_v,
            defocus_disk_u,
            defocus_disk_v,
        }
    }

    fn get_ray(&self, i: i32, j: i32, s_i: i32, s_j: i32, data: &CameraInternals) -> Ray {
        let offset = self.sample_square_stratified(s_i, s_j, data.recip_sqrt_spp);
        let pixel_sample = data.pixel00_loc
            + (i as f64 + offset.x()) * data.pixel_delta_u
            + (j as f64 + offset.y()) * data.pixel_delta_v;

        let ray_origin = if self.defocus_angle <= 0.0 {
            data.center
        } else {
            self.defocus_disk_sample(data)
        };
        let ray_direction = pixel_sample - ray_origin;
        let ray_time = random_double();

        Ray::new_with_time(ray_origin, ray_direction, ray_time)
    }

    fn sample_square_stratified(&self, s_i: i32, s_j: i32, recip_sqrt_spp: f64) -> Vec3 {
        let px = ((s_i as f64 + random_double()) * recip_sqrt_spp) - 0.5;
        let py = ((s_j as f64 + random_double()) * recip_sqrt_spp) - 0.5;
        Vec3::new(px, py, 0.0)
    }

    fn defocus_disk_sample(&self, data: &CameraInternals) -> Point3 {
        let p = random_in_unit_disk();
        data.center + (p[0] * data.defocus_disk_u) + (p[1] * data.defocus_disk_v)
    }

    fn ray_color<H: Hittable>(&self, r: Ray, depth: i32, world: &H, lights: HittableRef) -> Color {
        if depth <= 0 {
            return Color::new(0.0, 0.0, 0.0);
        }

        let Some(rec) = world.hit(&r, Interval::new(0.001, INFINITY)) else {
            return self.background;
        };

        let emitted = rec.mat.emitted(&r, &rec, rec.u, rec.v, rec.p);

        let Some(srec) = rec.mat.scatter(&r, &rec) else {
            return emitted;
        };

        if srec.skip_pdf {
            let bounces = self.max_depth - depth;
            if bounces >= 5 {
                let p = srec
                    .attenuation
                    .x()
                    .max(srec.attenuation.y())
                    .max(srec.attenuation.z())
                    .clamp(0.05, 0.95);
                if random_double() > p {
                    return emitted;
                }
                return emitted
                    + (srec.attenuation
                        * self.ray_color(srec.skip_pdf_ray, depth - 1, world, lights)
                        / p);
            }
            return emitted + srec.attenuation * self.ray_color(srec.skip_pdf_ray, depth - 1, world, lights);
        }

        let Some(pdf_ptr) = srec.pdf_ptr else {
            return emitted;
        };

        let bounces = self.max_depth - depth;
        let rr_prob = if bounces >= 5 {
            srec
                .attenuation
                .x()
                .max(srec.attenuation.y())
                .max(srec.attenuation.z())
                .clamp(0.05, 0.95)
        } else {
            1.0
        };

        if rr_prob < 1.0 && random_double() > rr_prob {
            return emitted;
        }

        let light_pdf: Arc<dyn Pdf + Send + Sync> = Arc::new(HittablePdf::new(lights.clone(), rec.p));
        let mixed_pdf = MixturePdf::new(light_pdf, pdf_ptr);

        let scattered = Ray::new_with_time(rec.p, mixed_pdf.generate(), r.time());
        let pdf_value = mixed_pdf.value(scattered.direction());
        if pdf_value <= 0.0 {
            return emitted;
        }

        let scattering_pdf = rec.mat.scattering_pdf(&r, &rec, &scattered);
        let sample_color = self.ray_color(scattered, depth - 1, world, lights);
        let color_from_scatter =
            srec.attenuation * scattering_pdf * sample_color / (pdf_value * rr_prob);

        emitted + color_from_scatter
    }
}
