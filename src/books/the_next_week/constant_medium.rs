use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable, HittableRef};
use super::interval::Interval;
use super::material::{make_mat, Isotropic, MaterialRef};
use super::ray::Ray;
use super::rtweekend::{random_double, INFINITY};
use super::texture::TextureRef;
use super::vec3::{Color, Vec3};

pub struct ConstantMedium {
    boundary: HittableRef,
    neg_inv_density: f64,
    phase_function: MaterialRef,
}

impl ConstantMedium {
    #[allow(dead_code)]
    pub fn new(
        boundary: HittableRef,
        density: f64,
        tex: TextureRef,
    ) -> Self {
        Self {
            boundary,
            neg_inv_density: -1.0 / density,
            phase_function: make_mat(Isotropic::from_texture(tex)),
        }
    }

    pub fn from_color(boundary: HittableRef, density: f64, albedo: Color) -> Self {
        Self {
            boundary,
            neg_inv_density: -1.0 / density,
            phase_function: make_mat(Isotropic::new(albedo)),
        }
    }
}

impl Hittable for ConstantMedium {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let mut rec1 = self.boundary.hit(r, Interval::UNIVERSE)?;
        let mut rec2 = self.boundary.hit(r, Interval::new(rec1.t + 0.0001, INFINITY))?;

        if rec1.t < ray_t.min {
            rec1.t = ray_t.min;
        }
        if rec2.t > ray_t.max {
            rec2.t = ray_t.max;
        }

        if rec1.t >= rec2.t {
            return None;
        }

        if rec1.t < 0.0 {
            rec1.t = 0.0;
        }

        let ray_length = r.direction().length();
        let distance_inside_boundary = (rec2.t - rec1.t) * ray_length;
        let hit_distance = self.neg_inv_density * random_double().ln();

        if hit_distance > distance_inside_boundary {
            return None;
        }

        let t = rec1.t + hit_distance / ray_length;
        let p = r.at(t);

        Some(HitRecord {
            p,
            normal: Vec3::new(1.0, 0.0, 0.0),
            mat: self.phase_function.clone(),
            t,
            u: 0.0,
            v: 0.0,
            front_face: true,
        })
    }

    fn bounding_box(&self) -> Aabb {
        self.boundary.bounding_box()
    }
}
