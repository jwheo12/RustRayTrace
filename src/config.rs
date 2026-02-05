#[derive(Clone, Copy, Debug)]
pub struct RenderOverrides {
    pub aspect_ratio: Option<f64>,
    pub image_width: Option<i32>,
    pub samples_per_pixel: Option<i32>,
    pub max_depth: Option<i32>,
    pub vfov: Option<f64>,
    pub lookfrom: Option<[f64; 3]>,
    pub lookat: Option<[f64; 3]>,
    pub vup: Option<[f64; 3]>,
    pub defocus_angle: Option<f64>,
    pub focus_dist: Option<f64>,
    pub background: Option<[f64; 3]>,
}

impl RenderOverrides {
    #[allow(dead_code)]
    pub const fn none() -> Self {
        Self {
            aspect_ratio: None,
            image_width: None,
            samples_per_pixel: None,
            max_depth: None,
            vfov: None,
            lookfrom: None,
            lookat: None,
            vup: None,
            defocus_angle: None,
            focus_dist: None,
            background: None,
        }
    }
}

// Set any field to `Some(value)` to override all scenes across all books.
// Example:
// pub const OVERRIDES: RenderOverrides = RenderOverrides {
//     aspect_ratio: Some(16.0 / 9.0),
//     image_width: Some(800),
//     samples_per_pixel: Some(200),
//     max_depth: Some(50),
//     vfov: Some(20.0),
//     lookfrom: Some([13.0, 2.0, 3.0]),
//     lookat: Some([0.0, 0.0, 0.0]),
//     vup: Some([0.0, 1.0, 0.0]),
//     defocus_angle: Some(0.6),
//     focus_dist: Some(10.0),
//     background: Some([0.0, 0.0, 0.0]),
// };
pub const OVERRIDES: RenderOverrides = RenderOverrides {
    aspect_ratio: None,
    image_width: Some(2160),
    samples_per_pixel: Some(5000),
    max_depth: Some(100),
    vfov: None,
    lookfrom: None,
    lookat: None,
    vup: None,
    defocus_angle: None,
    focus_dist: None,
    background: None,
};
