use std::ops::Add;

use super::rtweekend::INFINITY;

#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}

impl Interval {
    #[allow(dead_code)]
    pub const EMPTY: Interval = Interval { min: INFINITY, max: -INFINITY };
    #[allow(dead_code)]
    pub const UNIVERSE: Interval = Interval { min: -INFINITY, max: INFINITY };

    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn from_intervals(a: Interval, b: Interval) -> Self {
        Self {
            min: if a.min <= b.min { a.min } else { b.min },
            max: if a.max >= b.max { a.max } else { b.max },
        }
    }

    pub fn size(self) -> f64 {
        self.max - self.min
    }

    pub fn contains(self, x: f64) -> bool {
        self.min <= x && x <= self.max
    }

    pub fn surrounds(self, x: f64) -> bool {
        self.min < x && x < self.max
    }

    pub fn clamp(self, x: f64) -> f64 {
        if x < self.min {
            self.min
        } else if x > self.max {
            self.max
        } else {
            x
        }
    }

    pub fn expand(self, delta: f64) -> Self {
        let padding = delta / 2.0;
        Interval::new(self.min - padding, self.max + padding)
    }
}

impl Add<f64> for Interval {
    type Output = Interval;

    fn add(self, displacement: f64) -> Self::Output {
        Interval::new(self.min + displacement, self.max + displacement)
    }
}
