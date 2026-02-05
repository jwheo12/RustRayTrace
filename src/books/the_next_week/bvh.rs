use std::cmp::Ordering;
use std::sync::Arc;

use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable};
use super::hittable_list::HittableList;
use super::interval::Interval;
use super::ray::Ray;

pub struct BvhNode {
    left: Arc<dyn Hittable + Send + Sync>,
    right: Arc<dyn Hittable + Send + Sync>,
    bbox: Aabb,
}

impl BvhNode {
    pub fn new(list: HittableList) -> Self {
        let mut objects = list.objects;
        Self::build(&mut objects)
    }

    fn build(objects: &mut [Arc<dyn Hittable + Send + Sync>]) -> Self {
        let mut bbox = Aabb::EMPTY;
        for object in objects.iter() {
            bbox = Aabb::from_boxes(bbox, object.bounding_box());
        }

        let axis = bbox.longest_axis();
        let comparator = |a: &Arc<dyn Hittable + Send + Sync>, b: &Arc<dyn Hittable + Send + Sync>| {
            let a_axis = a.bounding_box().axis_interval(axis);
            let b_axis = b.bounding_box().axis_interval(axis);
            a_axis.min.partial_cmp(&b_axis.min).unwrap_or(Ordering::Equal)
        };

        let object_span = objects.len();

        let (left, right) = if object_span == 1 {
            (objects[0].clone(), objects[0].clone())
        } else if object_span == 2 {
            (objects[0].clone(), objects[1].clone())
        } else {
            objects.sort_by(comparator);
            let mid = object_span / 2;
            let left: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[..mid]));
            let right: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[mid..]));
            (left, right)
        };

        Self { left, right, bbox }
    }
}

impl Hittable for BvhNode {
    fn hit(&self, r: &Ray, ray_t: Interval) -> Option<HitRecord> {
        if !self.bbox.hit(r, ray_t) {
            return None;
        }

        let hit_left = self.left.hit(r, ray_t);
        let hit_right = match hit_left {
            Some(ref rec) => self.right.hit(r, Interval::new(ray_t.min, rec.t)),
            None => self.right.hit(r, ray_t),
        };

        hit_right.or(hit_left)
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
