use image::Primitive;

use super::{max, min, PixelBytes, R_CH, G_CH, B_CH};
pub type HSVPixel = [f32; 3];

pub const BLACK:PixelBytes<u8> = [0, 0, 0];
pub const V_MULT:u16 = 10000;
pub const REDMEAN_MAX:f32 = 764.834;

pub fn redmean_distance(pixel_one:PixelBytes<u8>, pixel_two:PixelBytes<u8>) -> f32 {
  let r = 0.5 * (pixel_one[R_CH] as f32 + pixel_two[R_CH] as f32);

  /*
  
  dR = abs(R1 - R2)
  dG = abs(G1 - G2)
  dB = abs(B1 - B2)

  delta C = sqrt(
    (2 + r / 256)*)dR^2 + 4 * dG^2 + (2 + (255 - r)/256)dB^2
  )

  */

  let d_r = pixel_one[R_CH].abs_diff(pixel_two[R_CH]);
  let d_g = pixel_one[G_CH].abs_diff(pixel_two[G_CH]);
  let d_b = pixel_one[B_CH].abs_diff(pixel_two[B_CH]);

  let mut c_squared = (2. + r / 256.0) * (d_r as f32).powf(2.);
  c_squared += 4. * (d_g as f32).powf(2.);
  c_squared += (2. + (255. - r)/256.0) * (d_b as f32).powf(2.);

  let redmean_distance = c_squared.sqrt();
  
  redmean_distance / REDMEAN_MAX
}

/// Convert RGB to HSV
pub fn rgb_to_hsv<T: Primitive>(pixels: PixelBytes<T>) -> HSVPixel {
  
  let r = pixels[R_CH].to_f32().unwrap();
  let g = pixels[G_CH].to_f32().unwrap();
  let b = pixels[B_CH].to_f32().unwrap();

  let c_max = max(max(r, g), b);
  let c_min = min(min(r, g), b);

  let delta = c_max - c_min;

  let mut hue = 0.;
  if delta == 0. {
    hue = 0.;
  } else if c_max == r {
    hue = (g - b) / delta % 6.;
  } else if c_max == g {
    hue = (b - r) / delta + 2.;
  } else if c_max == b {
    hue = (r - g) / delta + 4.;
  }

  hue = hue * 60.;

  let saturation = if c_max == 0. { 0. } else { delta / c_max };

  [hue, saturation, c_max]
}

/// Convert HSV to RGB
pub fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> PixelBytes<u8> {
  let hi = ((hue / 60.).floor() % 6.) as u32;
  let f = (hue / 60.) - (hue / 60.).floor();

  let value = value * 255.;
  let v = value.round() as u8;
  let p = (value * (1. - saturation)).round() as u8;
  let q = (value * (1. - f * saturation)).round() as u8;
  let t = (value * (1. - (1. - f) * saturation)).round() as u8;

  match hi {
    0 => [v, t, p],
    1 => [q, v, p],
    2 => [p, v, t],
    3 => [p, q, v],
    4 => [t, p, v],
    _ => [v, p, q],
  }
}