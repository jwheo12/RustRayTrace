use std::sync::Arc;

use super::aabb::Aabb;
use super::bvh::BvhNode;
use super::hittable_list::HittableList;
use super::interval::Interval;
use super::material::MaterialRef;
use super::ray::Ray;
use super::sphere::Sphere;
use super::vec3::{dot, Point3, Vec3};

pub struct HitRecord {
    pub p: Point3,
    pub normal: Vec3,
    pub mat: MaterialRef,
    pub t: f64,
    pub front_face: bool,
}

impl HitRecord {
    pub fn new(
        p: Point3,
        t: f64,
        r: &Ray,
        outward_normal: Vec3,
        mat: MaterialRef,
    ) -> Self {
        let front_face = dot(r.direction(), outward_normal) < 0.0;
        let normal = if front_face { outward_normal } else { -outward_normal };
        Self { p, normal, mat, t, front_face }
    }
}

pub trait Hittable: Send + Sync {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord>;
    fn bounding_box(&self) -> Aabb;
}

pub type HittableRef = Arc<HittableObject>;

pub fn make_ref<T: Into<HittableObject>>(object: T) -> HittableRef {
    Arc::new(object.into())
}

pub enum HittableObject {
    Sphere(Sphere),
    Bvh(BvhNode),
    List(HittableList),
}

impl From<Sphere> for HittableObject {
    fn from(value: Sphere) -> Self {
        Self::Sphere(value)
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
            HittableObject::Bvh(object) => object.hit(r, ray_t),
            HittableObject::List(object) => object.hit(r, ray_t),
        }
    }

    fn bounding_box(&self) -> Aabb {
        match self {
            HittableObject::Sphere(object) => object.bounding_box(),
            HittableObject::Bvh(object) => object.bounding_box(),
            HittableObject::List(object) => object.bounding_box(),
        }
    }
}
