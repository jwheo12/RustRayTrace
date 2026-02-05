use std::cell::RefCell;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

pub const INFINITY: f64 = f64::INFINITY;
pub const PI: f64 = std::f64::consts::PI;

thread_local! {
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

pub fn random_double() -> f64 {
    // Returns a random real in [0,1).
    RNG.with(|rng| rng.borrow_mut().gen_range(0.0..1.0))
}

pub fn random_double_range(min: f64, max: f64) -> f64 {
    // Returns a random real in [min,max).
    RNG.with(|rng| rng.borrow_mut().gen_range(min..max))
}

pub fn random_int(min: i32, max: i32) -> i32 {
    // Returns a random integer in [min,max].
    (random_double_range(min as f64, (max + 1) as f64)) as i32
}
