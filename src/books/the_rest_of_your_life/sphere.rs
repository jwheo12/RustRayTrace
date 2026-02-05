use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable};
use super::interval::Interval;
use super::material::MaterialRef;
use super::onb::Onb;
use super::ray::Ray;
use super::rtweekend::{random_double, INFINITY, PI};
use super::vec3::{dot, Point3, Vec3};

pub struct Sphere {
    center: Ray,
    radius: f64,
    mat: MaterialRef,
    bbox: Aabb,
}

impl Sphere {
    pub fn new(static_center: Point3, radius: f64, mat: MaterialRef) -> Self {
        let rvec = Vec3::new(radius, radius, radius);
        let bbox = Aabb::from_points(static_center - rvec, static_center + rvec);
        Self {
            center: Ray::new(static_center, Vec3::new(0.0, 0.0, 0.0)),
            radius: radius.max(0.0),
            mat,
            bbox,
        }
    }

    pub fn new_moving(
        center1: Point3,
        center2: Point3,
        radius: f64,
        mat: MaterialRef,
    ) -> Self {
        let rvec = Vec3::new(radius, radius, radius);
        let box1 = Aabb::from_points(center1 - rvec, center1 + rvec);
        let box2 = Aabb::from_points(center2 - rvec, center2 + rvec);
        let bbox = Aabb::from_boxes(box1, box2);
        Self {
            center: Ray::new(center1, center2 - center1),
            radius: radius.max(0.0),
            mat,
            bbox,
        }
    }

    fn get_sphere_uv(p: Point3) -> (f64, f64) {
        let theta = (-p.y()).acos();
        let phi = (-p.z()).atan2(p.x()) + PI;
        let u = phi / (2.0 * PI);
        let v = theta / PI;
        (u, v)
    }

    fn random_to_sphere(radius: f64, distance_squared: f64) -> Vec3 {
        let r1 = random_double();
        let r2 = random_double();
        let z = 1.0 + r2 * ((1.0 - radius * radius / distance_squared).sqrt() - 1.0);

        let phi = 2.0 * PI * r1;
        let x = phi.cos() * (1.0 - z * z).sqrt();
        let y = phi.sin() * (1.0 - z * z).sqrt();

        Vec3::new(x, y, z)
    }
}

impl Hittable for Sphere {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let current_center = self.center.at(r.time());
        let oc = current_center - r.origin();
        let a = r.direction().length_squared();
        let h = dot(r.direction(), oc);
        let c = oc.length_squared() - self.radius * self.radius;

        let discriminant = h * h - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();

        let mut root = (h - sqrtd) / a;
        if !ray_t.surrounds(root) {
            root = (h + sqrtd) / a;
            if !ray_t.surrounds(root) {
                return None;
            }
        }

        let p = r.at(root);
        let outward_normal = (p - current_center) / self.radius;
        let (u, v) = Sphere::get_sphere_uv(outward_normal);

        Some(HitRecord::new(p, root, r, outward_normal, self.mat.clone(), u, v))
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }

    fn pdf_value(&self, origin: Point3, direction: Vec3) -> f64 {
        if self
            .hit(&Ray::new(origin, direction), Interval::new(0.001, INFINITY))
            .is_none()
        {
            return 0.0;
        }

        let dist_squared = (self.center.at(0.0) - origin).length_squared();
        let cos_theta_max = (1.0 - self.radius * self.radius / dist_squared).sqrt();
        let solid_angle = 2.0 * PI * (1.0 - cos_theta_max);

        1.0 / solid_angle
    }

    fn random(&self, origin: Point3) -> Vec3 {
        let direction = self.center.at(0.0) - origin;
        let distance_squared = direction.length_squared();
        let uvw = Onb::new(direction);
        uvw.transform(Sphere::random_to_sphere(self.radius, distance_squared))
    }
}
