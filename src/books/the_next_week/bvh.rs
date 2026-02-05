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
        let object_span = objects.len();

        let mut bbox = Aabb::EMPTY;
        for object in objects.iter() {
            bbox = Aabb::from_boxes(bbox, object.bounding_box());
        }

        let (left, right) = if object_span == 1 {
            (objects[0].clone(), objects[0].clone())
        } else if object_span == 2 {
            (objects[0].clone(), objects[1].clone())
        } else {
            const NUM_BUCKETS: usize = 12;

            let axis = bbox.longest_axis();
            let comparator = |a: &Arc<dyn Hittable + Send + Sync>, b: &Arc<dyn Hittable + Send + Sync>| {
                let a_axis = a.bounding_box().axis_interval(axis);
                let b_axis = b.bounding_box().axis_interval(axis);
                a_axis.min.partial_cmp(&b_axis.min).unwrap_or(Ordering::Equal)
            };

            let mut centroid_min = f64::INFINITY;
            let mut centroid_max = f64::NEG_INFINITY;
            for object in objects.iter() {
                let b = object.bounding_box();
                let centroid = 0.5 * (b.axis_interval(axis).min + b.axis_interval(axis).max);
                if centroid < centroid_min {
                    centroid_min = centroid;
                }
                if centroid > centroid_max {
                    centroid_max = centroid;
                }
            }

            if (centroid_max - centroid_min).abs() < 1e-12 {
                objects.sort_by(comparator);
                let mid = object_span / 2;
                let left: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[..mid]));
                let right: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[mid..]));
                (left, right)
            } else {
                #[derive(Clone, Copy)]
                struct Bucket {
                    count: usize,
                    bbox: Aabb,
                }

                let mut buckets = vec![Bucket { count: 0, bbox: Aabb::EMPTY }; NUM_BUCKETS];
                for object in objects.iter() {
                    let b = object.bounding_box();
                    let centroid = 0.5 * (b.axis_interval(axis).min + b.axis_interval(axis).max);
                    let mut idx =
                        ((centroid - centroid_min) / (centroid_max - centroid_min) * NUM_BUCKETS as f64)
                            as usize;
                    if idx >= NUM_BUCKETS {
                        idx = NUM_BUCKETS - 1;
                    }
                    buckets[idx].count += 1;
                    buckets[idx].bbox = Aabb::from_boxes(buckets[idx].bbox, b);
                }

                let mut right_bbox = vec![Aabb::EMPTY; NUM_BUCKETS];
                let mut right_count = vec![0usize; NUM_BUCKETS];
                let mut accum_bbox = Aabb::EMPTY;
                let mut accum_count = 0usize;
                for i in (0..NUM_BUCKETS).rev() {
                    accum_count += buckets[i].count;
                    accum_bbox = Aabb::from_boxes(accum_bbox, buckets[i].bbox);
                    right_bbox[i] = accum_bbox;
                    right_count[i] = accum_count;
                }

                let mut left_bbox = Aabb::EMPTY;
                let mut left_count = 0usize;
                let mut best_cost = f64::INFINITY;
                let mut best_split = 0usize;

                for i in 0..(NUM_BUCKETS - 1) {
                    left_count += buckets[i].count;
                    left_bbox = Aabb::from_boxes(left_bbox, buckets[i].bbox);

                    let right_bbox = right_bbox[i + 1];
                    let right_count = right_count[i + 1];

                    if left_count == 0 || right_count == 0 {
                        continue;
                    }

                    let cost = left_bbox.surface_area() * left_count as f64
                        + right_bbox.surface_area() * right_count as f64;
                    if cost < best_cost {
                        best_cost = cost;
                        best_split = i;
                    }
                }

                if !best_cost.is_finite() {
                    objects.sort_by(comparator);
                    let mid = object_span / 2;
                    let left: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[..mid]));
                    let right: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[mid..]));
                    (left, right)
                } else {
                    let mut mid = 0usize;
                    for i in 0..object_span {
                        let b = objects[i].bounding_box();
                        let centroid = 0.5 * (b.axis_interval(axis).min + b.axis_interval(axis).max);
                        let mut idx = ((centroid - centroid_min) / (centroid_max - centroid_min)
                            * NUM_BUCKETS as f64) as usize;
                        if idx >= NUM_BUCKETS {
                            idx = NUM_BUCKETS - 1;
                        }
                        if idx <= best_split {
                            objects.swap(i, mid);
                            mid += 1;
                        }
                    }

                    if mid == 0 || mid == object_span {
                        objects.sort_by(comparator);
                        let mid = object_span / 2;
                        let left: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[..mid]));
                        let right: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[mid..]));
                        (left, right)
                    } else {
                        let left: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[..mid]));
                        let right: Arc<dyn Hittable + Send + Sync> = Arc::new(Self::build(&mut objects[mid..]));
                        (left, right)
                    }
                }
            }
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
