use std::ops::Add;

use super::interval::Interval;
use super::ray::Ray;
use super::vec3::{Point3, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub x: Interval,
    pub y: Interval,
    pub z: Interval,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb { x: Interval::EMPTY, y: Interval::EMPTY, z: Interval::EMPTY };
    #[allow(dead_code)]
    pub const UNIVERSE: Aabb = Aabb {
        x: Interval::UNIVERSE,
        y: Interval::UNIVERSE,
        z: Interval::UNIVERSE,
    };

    pub fn new(x: Interval, y: Interval, z: Interval) -> Self {
        let mut bbox = Self { x, y, z };
        bbox.pad_to_minimums();
        bbox
    }

    pub fn from_points(a: Point3, b: Point3) -> Self {
        let x = if a[0] <= b[0] { Interval::new(a[0], b[0]) } else { Interval::new(b[0], a[0]) };
        let y = if a[1] <= b[1] { Interval::new(a[1], b[1]) } else { Interval::new(b[1], a[1]) };
        let z = if a[2] <= b[2] { Interval::new(a[2], b[2]) } else { Interval::new(b[2], a[2]) };
        Self::new(x, y, z)
    }

    pub fn from_boxes(box0: Aabb, box1: Aabb) -> Self {
        Self::new(
            Interval::from_intervals(box0.x, box1.x),
            Interval::from_intervals(box0.y, box1.y),
            Interval::from_intervals(box0.z, box1.z),
        )
    }

    pub fn axis_interval(&self, n: usize) -> Interval {
        match n {
            1 => self.y,
            2 => self.z,
            _ => self.x,
        }
    }

    pub fn hit(&self, r: &Ray, mut ray_t: Interval) -> bool {
        let ray_orig = r.origin();
        let ray_dir = r.direction();

        for axis in 0..3 {
            let ax = self.axis_interval(axis);
            let adinv = 1.0 / ray_dir[axis];

            let t0 = (ax.min - ray_orig[axis]) * adinv;
            let t1 = (ax.max - ray_orig[axis]) * adinv;

            if t0 < t1 {
                if t0 > ray_t.min {
                    ray_t.min = t0;
                }
                if t1 < ray_t.max {
                    ray_t.max = t1;
                }
            } else {
                if t1 > ray_t.min {
                    ray_t.min = t1;
                }
                if t0 < ray_t.max {
                    ray_t.max = t0;
                }
            }

            if ray_t.max <= ray_t.min {
                return false;
            }
        }
        true
    }

    pub fn longest_axis(&self) -> usize {
        if self.x.size() > self.y.size() {
            if self.x.size() > self.z.size() {
                0
            } else {
                2
            }
        } else if self.y.size() > self.z.size() {
            1
        } else {
            2
        }
    }

    fn pad_to_minimums(&mut self) {
        let delta = 0.0001;
        if self.x.size() < delta {
            self.x = self.x.expand(delta);
        }
        if self.y.size() < delta {
            self.y = self.y.expand(delta);
        }
        if self.z.size() < delta {
            self.z = self.z.expand(delta);
        }
    }
}

impl Add<Vec3> for Aabb {
    type Output = Aabb;

    fn add(self, offset: Vec3) -> Self::Output {
        Aabb::new(self.x + offset.x(), self.y + offset.y(), self.z + offset.z())
    }
}
