use std::env;
use std::path::{Path, PathBuf};

pub struct RtwImage {
    data: Vec<u8>,
    width: i32,
    height: i32,
}

impl RtwImage {
    pub fn new(image_filename: &str) -> Self {
        let mut image = Self { data: Vec::new(), width: 0, height: 0 };

        if let Ok(imagedir) = env::var("RTW_IMAGES") {
            let candidate = Path::new(&imagedir).join(image_filename);
            if image.load(&candidate) {
                return image;
            }
        }

        if image.load(Path::new(image_filename)) {
            return image;
        }

        let mut prefix = PathBuf::new();
        for _ in 0..7 {
            let candidate = prefix.join("images").join(image_filename);
            if image.load(&candidate) {
                return image;
            }
            prefix.push("..");
        }

        eprintln!("ERROR: Could not load image file '{}'.", image_filename);
        image
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn pixel_data(&self, x: i32, y: i32) -> [u8; 3] {
        if self.data.is_empty() {
            return [255, 0, 255];
        }

        let x = clamp(x, 0, self.width);
        let y = clamp(y, 0, self.height);
        let idx = (y * self.width + x) as usize * 3;
        [self.data[idx], self.data[idx + 1], self.data[idx + 2]]
    }

    fn load(&mut self, filename: &Path) -> bool {
        let Ok(img) = image::open(filename) else {
            return false;
        };

        let rgb = img.to_rgb8();
        self.width = rgb.width() as i32;
        self.height = rgb.height() as i32;
        self.data = rgb.into_raw();
        true
    }
}

fn clamp(x: i32, low: i32, high: i32) -> i32 {
    if x < low {
        low
    } else if x < high {
        x
    } else {
        high - 1
    }
}
