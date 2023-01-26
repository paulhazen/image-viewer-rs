use crate::core::{operations::{perform_operation, OpType}, float_pixel_to_rgb};
use std::f32::consts::PI;
use super::{ppm::{PpmImage, Padding}, operations::OperationResult};
use crate::core::{EULER, R_CH, B_CH, G_CH, COLOR_CHANNELS, PIXEL_SIZE};

pub const SOBEL_H: [i32;9] = [
   1,  2,  1, 
   0,  0,  0, 
  -1, -2, -1
];

pub const SOBEL_H_REV: [i32;9] = [
  -1, -2, -1, 
   0,  0,  0, 
   1,  2,  1
];

pub const SOBEL_V: [i32;9] = [
  1,  0, -1,
  2,  0, -2,
  1,  0, -1
];

pub const SOBEL_V_REV: [i32;9] = [
  -1,  0, 1,
  -2,  0, 2,
  -1,  0, 1
];

pub fn gaussian_blur(
  image: &PpmImage, 
  sigma:f32, 
  kernel_size:i32, 
  padding:Padding
) -> OperationResult {

  /* #region Error Handling */
  if sigma <= 0. {
    return Err(format!("Sigma value must be greater than 0, cannot be: {:.3}", sigma))
  }

  if kernel_size % 2 == 0 {
    return Err(format!("Cannot have a blur filter with an even kernel size of {}. Kernel size must be odd.", kernel_size))
  }

  if kernel_size < 3 {
    return Err(format!("Cannot have a kernel size that is less than three"));
  }
  /* #endregion */

  let blur_mask = get_gaussian_weight_matrix(kernel_size, sigma);

  apply_mask(image, blur_mask, padding)
}

pub fn unsharp_mask(
  image: &PpmImage, 
  sigma: f32, 
  kernel_size:i32, 
  scale:f32, 
  padding:Padding
) -> OperationResult {
  
    // Unsharp Mask: OriginalImage + Blurred(Negated(OriginalImage))

    let blur_filter = get_gaussian_weight_matrix(kernel_size, sigma);
    let origin = get_origin_matrix(kernel_size);

    let mut sharpen_mask = vec![0.;origin.len()];

    for i in 0..origin.len() {
      sharpen_mask[i] = origin[i] + (origin[i] - blur_filter[i]) * scale;
    }

    // apply the unsharp mask
    apply_mask(image, sharpen_mask, padding)
}

fn apply_mask(
  image:&PpmImage, 
  mask:Vec<f32>, 
  padding:Padding
) -> OperationResult {
  let mut new_image = PpmImage::new(
    image.width(), image.height()
  );

  let kernel_size = (mask.len() as f32).sqrt() as usize;

  for y in 0..image.height() {
    for x in 0..image.width() {
      let matrix = image.get_matrix_at(
        x, y, kernel_size as usize, padding
      );
      let mut new_pixel_value: [f32; PIXEL_SIZE] = [0.; PIXEL_SIZE];
      for i in 0..matrix.len() {
        for ch in COLOR_CHANNELS {
          new_pixel_value[ch] += matrix[i][ch] as f32 * mask[i];
        }
      }

      new_image.set_pixel_by_coord(
        x, y,
        &float_pixel_to_rgb(new_pixel_value)
      );
    }
  }

  Ok(new_image)
}

/* #endregion */

pub fn apply_sobel(
  image: &PpmImage, sobel:[i32;9], padding:Padding
) -> PpmImage {
  let mut result_image = PpmImage::new(
    image.width(), image.height()
  );

  for y in 0..image.height() {
    for x in 0..image.width() {
      let matrix = image.get_matrix_at(
        x, y, 3, padding
      );
      let mut new_pixel_value:[i32; PIXEL_SIZE] = [0; PIXEL_SIZE];
      for i in 0..matrix.len() {
        for ch in [R_CH, G_CH, B_CH] {
          new_pixel_value[ch] += matrix[i][ch] as i32 * sobel[i];
        }
      }

      for i in 0..PIXEL_SIZE {
        if new_pixel_value[i] > u8::MAX as i32 {
          new_pixel_value[i] = u8::MAX as i32;
        } else if new_pixel_value[i] < 0 {
          new_pixel_value[i] = u8::MIN as i32;
        }
      }

      result_image.set_pixel_by_coord(
        x, y,  &[
        new_pixel_value[R_CH] as u8,
        new_pixel_value[G_CH] as u8,
        new_pixel_value[B_CH] as u8,
      ]);
    }
  }

  result_image
}

pub fn edge_detect(image: &PpmImage) -> OperationResult {
  
  let h_filtered = apply_sobel(image, SOBEL_H, Padding::Repeat);
  let v_filtered = apply_sobel(image, SOBEL_V, Padding::Repeat);

  perform_operation(&h_filtered, &v_filtered, OpType::Add)
}

/// Creates a matrix of float values that is kernel_size by kernel_size
fn get_origin_matrix(kernel_size:i32) -> Vec<f32> {

  let size = (kernel_size * kernel_size) as usize;
  let mut matrix = vec![0.;size];
  matrix[size / 2] = 1.;

  matrix
}

/// Creates a gaussian weight using the given kernel size and sigma
fn get_gaussian_weight_matrix(kernel_size:i32, sigma:f32) -> Vec<f32> {
  let mut matrix = Vec::<f32>::with_capacity(
    (kernel_size * kernel_size) as usize
  );

  // to keep track of the total weight
  let mut weight_total:f32 = 0.;

  // the delta start and end of the matrix to get
  let start = -1 * (kernel_size - 1) / 2;
  let end = (kernel_size - 1) / 2;

  let sigma_squared = sigma * sigma;
  let denominator = 2. * PI * sigma_squared;

  for x in start..(end + 1) {
    for y in start..(end + 1) {
      let weight = EULER.powf(
        -1. * (x*x + y*y) as f32 / (2. * sigma_squared)
      ) / denominator;
      weight_total += weight;
      matrix.push(weight);
    }
  }

  for i in 0..matrix.len() {
    matrix[i] /= weight_total;
  }

  matrix
}

