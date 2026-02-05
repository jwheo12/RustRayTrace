use std::io::{self, BufWriter, Write};

pub fn write_ppm_from_accum(width: usize, height: usize, accum: &[f32], samples_per_pixel: u32) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    writeln!(out, "P3\n{} {}\n255", width, height).map_err(|e| e.to_string())?;

    let scale = if samples_per_pixel > 0 { 1.0 / samples_per_pixel as f32 } else { 0.0 };
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            let mut r = accum[idx] * scale;
            let mut g = accum[idx + 1] * scale;
            let mut b = accum[idx + 2] * scale;

            if !r.is_finite() { r = 0.0; }
            if !g.is_finite() { g = 0.0; }
            if !b.is_finite() { b = 0.0; }

            r = r.max(0.0).sqrt();
            g = g.max(0.0).sqrt();
            b = b.max(0.0).sqrt();

            let ir = (r.clamp(0.0, 0.999) * 256.0) as u8;
            let ig = (g.clamp(0.0, 0.999) * 256.0) as u8;
            let ib = (b.clamp(0.0, 0.999) * 256.0) as u8;
            writeln!(out, "{} {} {}", ir, ig, ib).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
