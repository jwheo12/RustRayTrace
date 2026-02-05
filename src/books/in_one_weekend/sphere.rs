use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable};
use super::interval::Interval;
use super::material::MaterialRef;
use super::ray::Ray;
use super::vec3::{dot, Point3, Vec3};

pub struct Sphere {
    center: Point3,
    radius: f64,
    mat: MaterialRef,
    bbox: Aabb,
}

impl Sphere {
    pub fn new(center: Point3, radius: f64, mat: MaterialRef) -> Self {
        let r = radius.max(0.0);
        let rvec = Vec3::new(r, r, r);
        let bbox = Aabb::from_points(center - rvec, center + rvec);
        Self { center, radius: r, mat, bbox }
    }
}

impl Hittable for Sphere {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let oc = self.center - r.origin();
        let a = r.direction().length_squared();
        let h = dot(r.direction(), oc);
        let c = oc.length_squared() - self.radius * self.radius;

        let discriminant = h * h - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();

        // Find the nearest root that lies in the acceptable range.
        let mut root = (h - sqrtd) / a;
        if !ray_t.surrounds(root) {
            root = (h + sqrtd) / a;
            if !ray_t.surrounds(root) {
                return None;
            }
        }

        let p = r.at(root);
        let outward_normal = (p - self.center) / self.radius;

        Some(HitRecord::new(p, root, r, outward_normal, self.mat.clone()))
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
