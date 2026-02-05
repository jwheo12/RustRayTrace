use std::sync::Arc;

use super::aabb::Aabb;
use super::bvh::BvhNode;
use super::constant_medium::ConstantMedium;
use super::hittable_list::HittableList;
use super::interval::Interval;
use super::material::MaterialRef;
use super::quad::Quad;
use super::ray::Ray;
use super::rtweekend::{degrees_to_radians, INFINITY};
use super::sphere::Sphere;
use super::vec3::{dot, Point3, Vec3};

pub struct HitRecord {
    pub p: Point3,
    pub normal: Vec3,
    pub mat: MaterialRef,
    pub t: f64,
    pub u: f64,
    pub v: f64,
    pub front_face: bool,
}

impl HitRecord {
    pub fn new(
        p: Point3,
        t: f64,
        r: &Ray,
        outward_normal: Vec3,
        mat: MaterialRef,
        u: f64,
        v: f64,
    ) -> Self {
        let mut rec = Self {
            p,
            normal: outward_normal,
            mat,
            t,
            u,
            v,
            front_face: false,
        };
        rec.set_face_normal(r, outward_normal);
        rec
    }

    pub fn set_face_normal(&mut self, r: &Ray, outward_normal: Vec3) {
        self.front_face = dot(r.direction(), outward_normal) < 0.0;
        self.normal = if self.front_face { outward_normal } else { -outward_normal };
    }
}

pub trait Hittable: Send + Sync {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord>;
    fn bounding_box(&self) -> Aabb;

    fn pdf_value(&self, _origin: Point3, _direction: Vec3) -> f64 {
        0.0
    }

    fn random(&self, _origin: Point3) -> Vec3 {
        Vec3::new(1.0, 0.0, 0.0)
    }
}

pub type HittableRef = Arc<HittableObject>;

pub fn make_ref<T: Into<HittableObject>>(object: T) -> HittableRef {
    Arc::new(object.into())
}

pub struct Translate {
    object: HittableRef,
    offset: Vec3,
    bbox: Aabb,
}

impl Translate {
    pub fn new(object: HittableRef, offset: Vec3) -> Self {
        let bbox = object.bounding_box() + offset;
        Self { object, offset, bbox }
    }
}

impl Hittable for Translate {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let offset_r = Ray::new_with_time(r.origin() - self.offset, r.direction(), r.time());

        let mut rec = self.object.hit(&offset_r, ray_t)?;
        rec.p += self.offset;
        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}

pub struct RotateY {
    object: HittableRef,
    sin_theta: f64,
    cos_theta: f64,
    bbox: Aabb,
}

impl RotateY {
    pub fn new(object: HittableRef, angle: f64) -> Self {
        let radians = degrees_to_radians(angle);
        let sin_theta = radians.sin();
        let cos_theta = radians.cos();
        let bbox = object.bounding_box();

        let mut min = Point3::new(INFINITY, INFINITY, INFINITY);
        let mut max = Point3::new(-INFINITY, -INFINITY, -INFINITY);

        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    let x = if i == 1 { bbox.x.max } else { bbox.x.min };
                    let y = if j == 1 { bbox.y.max } else { bbox.y.min };
                    let z = if k == 1 { bbox.z.max } else { bbox.z.min };

                    let newx = cos_theta * x + sin_theta * z;
                    let newz = -sin_theta * x + cos_theta * z;

                    let tester = Vec3::new(newx, y, newz);

                    for c in 0..3 {
                        min[c] = min[c].min(tester[c]);
                        max[c] = max[c].max(tester[c]);
                    }
                }
            }
        }

        let bbox = Aabb::from_points(min, max);
        Self { object, sin_theta, cos_theta, bbox }
    }
}

impl Hittable for RotateY {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let origin = Point3::new(
            (self.cos_theta * r.origin().x()) - (self.sin_theta * r.origin().z()),
            r.origin().y(),
            (self.sin_theta * r.origin().x()) + (self.cos_theta * r.origin().z()),
        );

        let direction = Vec3::new(
            (self.cos_theta * r.direction().x()) - (self.sin_theta * r.direction().z()),
            r.direction().y(),
            (self.sin_theta * r.direction().x()) + (self.cos_theta * r.direction().z()),
        );

        let rotated_r = Ray::new_with_time(origin, direction, r.time());

        let mut rec = self.object.hit(&rotated_r, ray_t)?;

        rec.p = Point3::new(
            (self.cos_theta * rec.p.x()) + (self.sin_theta * rec.p.z()),
            rec.p.y(),
            (-self.sin_theta * rec.p.x()) + (self.cos_theta * rec.p.z()),
        );

        rec.normal = Vec3::new(
            (self.cos_theta * rec.normal.x()) + (self.sin_theta * rec.normal.z()),
            rec.normal.y(),
            (-self.sin_theta * rec.normal.x()) + (self.cos_theta * rec.normal.z()),
        );

        Some(rec)
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}

pub enum HittableObject {
    Sphere(Sphere),
    Quad(Quad),
    ConstantMedium(ConstantMedium),
    Translate(Translate),
    RotateY(RotateY),
    Bvh(BvhNode),
    List(HittableList),
}

impl From<Sphere> for HittableObject {
    fn from(value: Sphere) -> Self {
        Self::Sphere(value)
    }
}

impl From<Quad> for HittableObject {
    fn from(value: Quad) -> Self {
        Self::Quad(value)
    }
}

impl From<ConstantMedium> for HittableObject {
    fn from(value: ConstantMedium) -> Self {
        Self::ConstantMedium(value)
    }
}

impl From<Translate> for HittableObject {
    fn from(value: Translate) -> Self {
        Self::Translate(value)
    }
}

impl From<RotateY> for HittableObject {
    fn from(value: RotateY) -> Self {
        Self::RotateY(value)
    }
}

impl From<BvhNode> for HittableObject {
    fn from(value: BvhNode) -> Self {
        Self::Bvh(value)
    }
}

impl From<HittableList> for HittableObject {
    fn from(value: HittableList) -> Self {
        Self::List(value)
    }
}

impl Hittable for HittableObject {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        match self {
            HittableObject::Sphere(object) => object.hit(r, ray_t),
            HittableObject::Quad(object) => object.hit(r, ray_t),
            HittableObject::ConstantMedium(object) => object.hit(r, ray_t),
            HittableObject::Translate(object) => object.hit(r, ray_t),
            HittableObject::RotateY(object) => object.hit(r, ray_t),
            HittableObject::Bvh(object) => object.hit(r, ray_t),
            HittableObject::List(object) => object.hit(r, ray_t),
        }
    }

    fn bounding_box(&self) -> Aabb {
        match self {
            HittableObject::Sphere(object) => object.bounding_box(),
            HittableObject::Quad(object) => object.bounding_box(),
            HittableObject::ConstantMedium(object) => object.bounding_box(),
            HittableObject::Translate(object) => object.bounding_box(),
            HittableObject::RotateY(object) => object.bounding_box(),
            HittableObject::Bvh(object) => object.bounding_box(),
            HittableObject::List(object) => object.bounding_box(),
        }
    }

    fn pdf_value(&self, origin: Point3, direction: Vec3) -> f64 {
        match self {
            HittableObject::Sphere(object) => object.pdf_value(origin, direction),
            HittableObject::Quad(object) => object.pdf_value(origin, direction),
            HittableObject::ConstantMedium(object) => object.pdf_value(origin, direction),
            HittableObject::Translate(object) => object.pdf_value(origin, direction),
            HittableObject::RotateY(object) => object.pdf_value(origin, direction),
            HittableObject::Bvh(object) => object.pdf_value(origin, direction),
            HittableObject::List(object) => object.pdf_value(origin, direction),
        }
    }

    fn random(&self, origin: Point3) -> Vec3 {
        match self {
            HittableObject::Sphere(object) => object.random(origin),
            HittableObject::Quad(object) => object.random(origin),
            HittableObject::ConstantMedium(object) => object.random(origin),
            HittableObject::Translate(object) => object.random(origin),
            HittableObject::RotateY(object) => object.random(origin),
            HittableObject::Bvh(object) => object.random(origin),
            HittableObject::List(object) => object.random(origin),
        }
    }
}
