use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable, HittableRef};
use super::interval::Interval;
use super::ray::Ray;

pub struct HittableList {
    pub objects: Vec<HittableRef>,
    bbox: Aabb,
}

impl HittableList {
    pub fn new() -> Self {
        Self { objects: Vec::new(), bbox: Aabb::EMPTY }
    }

    pub fn from(object: HittableRef) -> Self {
        let mut list = Self::new();
        list.add(object);
        list
    }

    pub fn add(&mut self, object: HittableRef) {
        self.objects.push(object);
        let len = self.objects.len();
        if len == 1 {
            self.bbox = self.objects[0].bounding_box();
        } else {
            self.bbox = Aabb::from_boxes(self.bbox, self.objects[len - 1].bounding_box());
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.objects.clear();
        self.bbox = Aabb::EMPTY;
    }
}

impl Hittable for HittableList {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let mut hit_anything: Option<HitRecord> = None;
        let mut closest_so_far = ray_t.max;

        for object in &self.objects {
            if let Some(rec) = object.hit(r, Interval::new(ray_t.min, closest_so_far)) {
                closest_so_far = rec.t;
                hit_anything = Some(rec);
            }
        }

        hit_anything
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
