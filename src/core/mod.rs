use std::collections::HashMap;

use image::Primitive;

pub mod args;
pub mod io;
pub mod operations;
pub mod ppm;
pub mod ccl;
pub mod cr2;
pub mod color;
pub mod filters;
pub mod stacking;
pub mod fourier;

pub const EULER:f32 = 2.718281828459045235360;

pub const PIXEL_SIZE:usize = 3;
pub const R_CH:usize = 0;
pub const G_CH:usize = 1;
pub const B_CH:usize = 2;
/// Type alias for array of [PIXEL_SIZE] bytes

pub type PixelBytes<T: Primitive> = [T; PIXEL_SIZE];

pub const H_CH:usize = 0;
pub const S_CH:usize = 1;
pub const V_CH:usize = 2;

pub const COLOR_CHANNELS:[usize;PIXEL_SIZE] = [R_CH, G_CH, B_CH];

#[macro_export]
macro_rules! to_1d {
  ($x:expr, $y:expr, $width:expr) => {
    {
      ($x as usize + $y as usize * $width as usize) as usize
    }
  }
}

/// helper function to get the key in a hash map that has the highest value
/// currently this is most useful for finding the "background" color of an
/// image when considering a CCL labeling process
fn get_key_with_max_value<K, V>(hash_map: &HashMap<K, V>) -> &K
where V: Ord,
{
  hash_map
    .iter()
    .max_by(|a, b| a.1.cmp(&b.1))
    .map(|(k, _v)| k).unwrap()
}

fn float_pixel_to_rgb(pixel:[f32;PIXEL_SIZE]) -> PixelBytes<u8> {
  [
    pixel[R_CH].round() as u8,
    pixel[G_CH].round() as u8,
    pixel[B_CH].round() as u8
  ]
}

pub fn max<T: PartialOrd>(a: T, b: T) -> T {
  if a > b {
    a
  } else {
    b
  }
}

pub fn min<T: PartialOrd>(a: T, b: T) -> T {
  if a < b {
    a
  } else {
    b
  }
}