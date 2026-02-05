use super::aabb::Aabb;
use super::hittable::{HitRecord, Hittable, HittableRef};
use super::interval::Interval;
use super::ray::Ray;
use super::rtweekend::random_int;
use super::vec3::{Point3, Vec3};

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

    fn pdf_value(&self, origin: Point3, direction: Vec3) -> f64 {
        let weight = 1.0 / self.objects.len() as f64;
        let mut sum = 0.0;

        for object in &self.objects {
            sum += weight * object.pdf_value(origin, direction);
        }

        sum
    }

    fn random(&self, origin: Point3) -> Vec3 {
        let int_size = self.objects.len() as i32;
        let index = random_int(0, int_size - 1) as usize;
        self.objects[index].random(origin)
    }
}
