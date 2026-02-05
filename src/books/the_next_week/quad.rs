use std::sync::Arc;

use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable};
use super::hittable_list::HittableList;
use super::interval::Interval;
use super::material::Material;
use super::ray::Ray;
use super::vec3::{cross, dot, unit_vector, Point3, Vec3};

pub struct Quad {
    q: Point3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    mat: Arc<dyn Material + Send + Sync>,
    bbox: Aabb,
    normal: Vec3,
    d: f64,
}

impl Quad {
    pub fn new(q: Point3, u: Vec3, v: Vec3, mat: Arc<dyn Material + Send + Sync>) -> Self {
        let n = cross(u, v);
        let normal = unit_vector(n);
        let d = dot(normal, q);
        let w = n / dot(n, n);

        let mut quad = Self {
            q,
            u,
            v,
            w,
            mat,
            bbox: Aabb::EMPTY,
            normal,
            d,
        };
        quad.set_bounding_box();
        quad
    }

    fn set_bounding_box(&mut self) {
        let bbox_diagonal1 = Aabb::from_points(self.q, self.q + self.u + self.v);
        let bbox_diagonal2 = Aabb::from_points(self.q + self.u, self.q + self.v);
        self.bbox = Aabb::from_boxes(bbox_diagonal1, bbox_diagonal2);
    }

    fn is_interior(&self, a: f64, b: f64, rec: &mut HitRecord) -> bool {
        let unit_interval = Interval::new(0.0, 1.0);
        if !unit_interval.contains(a) || !unit_interval.contains(b) {
            return false;
        }

        rec.u = a;
        rec.v = b;
        true
    }
}

impl Hittable for Quad {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let denom = dot(self.normal, r.direction());

        if denom.abs() < 1e-8 {
            return None;
        }

        let t = (self.d - dot(self.normal, r.origin())) / denom;
        if !ray_t.contains(t) {
            return None;
        }

        let intersection = r.at(t);
        let planar_hitpt_vector = intersection - self.q;
        let alpha = dot(self.w, cross(planar_hitpt_vector, self.v));
        let beta = dot(self.w, cross(self.u, planar_hitpt_vector));

        let mut rec = HitRecord::new(intersection, t, r, self.normal, self.mat.clone(), 0.0, 0.0);
        if !self.is_interior(alpha, beta, &mut rec) {
            return None;
        }

        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}

pub fn make_box(
    a: Point3,
    b: Point3,
    mat: Arc<dyn Material + Send + Sync>,
) -> Arc<dyn Hittable + Send + Sync> {
    let mut sides = HittableList::new();

    let min = Point3::new(a.x().min(b.x()), a.y().min(b.y()), a.z().min(b.z()));
    let max = Point3::new(a.x().max(b.x()), a.y().max(b.y()), a.z().max(b.z()));

    let dx = Vec3::new(max.x() - min.x(), 0.0, 0.0);
    let dy = Vec3::new(0.0, max.y() - min.y(), 0.0);
    let dz = Vec3::new(0.0, 0.0, max.z() - min.z());

    sides.add(Arc::new(Quad::new(Point3::new(min.x(), min.y(), max.z()), dx, dy, mat.clone())));
    sides.add(Arc::new(Quad::new(Point3::new(max.x(), min.y(), max.z()), -dz, dy, mat.clone())));
    sides.add(Arc::new(Quad::new(Point3::new(max.x(), min.y(), min.z()), -dx, dy, mat.clone())));
    sides.add(Arc::new(Quad::new(Point3::new(min.x(), min.y(), min.z()), dz, dy, mat.clone())));
    sides.add(Arc::new(Quad::new(Point3::new(min.x(), max.y(), max.z()), dx, -dz, mat.clone())));
    sides.add(Arc::new(Quad::new(Point3::new(min.x(), min.y(), min.z()), dx, dz, mat)));

    Arc::new(sides)
}
