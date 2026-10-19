//! Tray icon pixels: a status light in the overall colour.

use padbridge_model::Light;

pub const SIZE: u32 = 32;

pub fn rgb(light: Light) -> [u8; 3] {
    match light {
        Light::Green => [46, 204, 113],
        Light::Amber => [243, 166, 35],
        Light::Red => [231, 76, 60],
        Light::Off => [140, 140, 140],
    }
}

/// RGBA pixels of a filled disc with a dark rim, edges anti-aliased.
pub fn light_rgba(light: Light) -> Vec<u8> {
    let [r, g, b] = rgb(light);
    let c = (SIZE as f32 - 1.0) / 2.0;
    let outer = SIZE as f32 / 2.0 - 1.0;
    let rim = outer - 2.5;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let alpha = (outer + 0.5 - d).clamp(0.0, 1.0);
            let colour = if d > rim { [30, 30, 30] } else { [r, g, b] };
            pixels.extend_from_slice(&[colour[0], colour[1], colour[2], (alpha * 255.0) as u8]);
        }
    }
    pixels
}

pub fn tooltip(light: Light) -> &'static str {
    match light {
        Light::Green => "Smashcraft: controller ready",
        Light::Amber => "Smashcraft: controller waiting",
        Light::Red => "Smashcraft: controller needs attention",
        Light::Off => "Smashcraft: controller support is off",
    }
}
