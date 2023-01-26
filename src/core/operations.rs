use std::collections::btree_map::Keys;
use std::{collections::BTreeMap};
use std::result::Result::Err;
use strum_macros::{EnumIter, Display};

use crate::core::{R_CH, G_CH, B_CH, H_CH, S_CH, V_CH, COLOR_CHANNELS};
use crate::core::ppm::PpmImage;

use super::PIXEL_SIZE;
use super::{color::{HSVPixel, self, V_MULT}};

pub type OperationResult = Result<PpmImage, String>;

#[derive(PartialEq, Clone)]
pub struct Histogram {
  pub data: BTreeMap::<u32, f32>,
  pub max_value: f32,
  pub min_value: f32,
  pub pixel_count: u32,
}

impl Histogram {
  fn new() -> Self {
    Histogram {
      data: BTreeMap::<u32, f32>::new(),
      max_value: f32::MIN,
      min_value: f32::MAX,
      pixel_count: 0,
    }
  }

  pub fn from_image(image:&PpmImage) -> Self {
    let mut histogram = Histogram::new();

    for y in 0..image.height() {
      for x in 0..image.width() {
        if let Some(pixel) = image.get_pixel_by_coord(x, y) {
          // convert the pixel to HSV
          let hsv_pixel = color::rgb_to_hsv::<u8>(pixel);
          // add the value channel to the histogram
          histogram.add(&hsv_pixel[V_CH]);
        }
      }
    }

    histogram
  }

  fn update_max_min(&mut self, value: u32) {
    if value > self.max_value as u32 {
      self.max_value = value as f32;
    }
    if value < self.min_value as u32 {
      self.min_value = value as f32;
    }
  }

  fn downsample_float(value: f32) -> u32 {
    // we multiply by V_MULT to store a less precise version of the f32 value
    // inside a u32 variable
    (value * V_MULT as f32) as u32
  }

  pub fn intensities(&self) -> Keys<u32, f32> {
    self.data.keys()
  }

  pub fn add(&mut self, value: &f32) {
    
    let key = Histogram::downsample_float(*value);

    self.update_max_min(key);

    if let Some(count) = self.data.get_mut(&key) {
      *count += 1.; // increment the count
    } else {
      self.data.insert(key, 1.); // or add it for the first time
    }

    self.pixel_count += 1;
  }

  pub fn equalize(&self) -> BTreeMap<u32, f32> {
    // calculate the probability for each intensity
    let mut intensity_eq = BTreeMap::<u32, f32>::new();
    let mut running_cdf = 0.;
    for (intensity, count) in self.data.iter() {
      let pr = *count as f32 / self.pixel_count as f32;
      running_cdf = running_cdf + pr;
      intensity_eq.insert(
        *intensity, 
        running_cdf * self.max_value as f32
      );
    }

    intensity_eq
  }
}

/* #region Data Structures */
#[derive(PartialEq, EnumIter, Display, Clone, Copy)]
pub enum OpType {
  Add,
  Subtract,
  Multiply,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ResizeAlgorithm {
  NearestNeighbor,
  BilinearInterpolation
}

/* #endregion */

/* #region Overflow safe math functions for pixels  */

fn mult_safe(lhs: u8, rhs:u8) -> u8 {
  let temp: u32 = lhs as u32 * rhs as u32;

  if temp > (u8::MAX as u32) {
    return u8::MAX
  }

  temp as u8
}

fn add_safe(lhs: u8, rhs:u8) -> u8 {
  // cast up to u32, perform addition
  let temp: u32 = lhs as u32 + rhs as u32;
  
  // check to see if overflow
  if temp > (u8::MAX as u32) {
    return u8::MAX
  }

  // otherwise, return sum
  temp as u8
}

fn sub_safe(lhs: u8, rhs:u8) -> u8 {
  // cast to i32, perform subtraction
  let temp: i32 = lhs as i32 - rhs as i32;

  // if temp is greater than u8, set to max
  if temp > (u8::MAX as i32) {
    return u8::MAX;
  } 
  // if less than 0, set to zero
  if temp < 0 {
    return 0;    
  }

  temp as u8
}

fn log_transform_safe(pixel:u8, c:f32, b:f32) -> u8 {
  let new_value = c * (pixel as f32 + 1.0).log(b);
  if new_value > u8::MAX as f32 {
    return u8::MAX
  } else {
    return new_value.round() as u8
  }
}

fn gamma_transform_safe(pixel:u8, gamma_correction:f32, _c:f32) -> u8 {
  // get the new value 
  let new_value: f32 = 255.0 * (pixel as f32 / 255.).powf(gamma_correction);

  if new_value > u8::MAX as f32 {
    return u8::MAX;
  } else {
    return new_value.round() as u8;
  }
}

/* #endregion */

/* #region Gamma and Log Transformations */

/**
 * Perform gamma transformation on the given image
 */
pub fn gamma_transform(
  ppm: &PpmImage,
  gamma: f32, 
  c_value: Option<f32>) -> OperationResult {
  let c = c_value.unwrap_or(1.0);
  let gamma_correction = 1.0 / gamma;

  // create our new ppm image
  let mut new_image = PpmImage::new(ppm.width(), ppm.height());

  let mut pixel_index:usize = 0;
  for i in 0..(ppm.width() * ppm.height()) {
    let rgb = ppm.get_pixel_at(i as usize);
    let transformed_rgb = [
      gamma_transform_safe(rgb[R_CH], gamma_correction, c),
      gamma_transform_safe(rgb[G_CH], gamma_correction, c),
      gamma_transform_safe(rgb[B_CH], gamma_correction, c),
    ];

    new_image.set_pixel(&mut pixel_index, &transformed_rgb);
  }

  Ok(new_image)
}

/**
 * Performs log transform on the given image with an optional c_value provided
 * If a c value is not provided, then the most ideal c will be selected which
 * is: 255 / (1 + max_value)
 */
pub fn log_transform(
  ppm: &PpmImage, 
  c_value: Option<f32>, 
  base_value: Option<f32>) -> OperationResult {
  // use the value of c provided, or default to what the value for c should be
  let c = c_value.unwrap_or(
    255.0 / (1.0 + ppm.max_value() as f32).log(10.0)
  );

  // default value of 10 for base
  let b = base_value.unwrap_or(10.0);

  // create our new ppm image
  let mut new_image = PpmImage::new(ppm.width(), ppm.height());
  let mut pixel_index:usize = 0;
  for i in 0..(ppm.width() * ppm.height()) {
    let rgb = ppm.get_pixel_at(i as usize);

    let transformed_rgb = [
      log_transform_safe(rgb[R_CH], c, b),
      log_transform_safe(rgb[G_CH], c, b),
      log_transform_safe(rgb[B_CH], c, b),
    ];

    new_image.set_pixel(&mut pixel_index, &transformed_rgb);
  }

  Ok(new_image)
}

/* #endregion */

/* #region Image Operations (addition, subtraction, and multiplication) */

pub fn perform_operation(
  lhs: &PpmImage, 
  rhs: &PpmImage, 
  optype:OpType) -> OperationResult {
  
    use crate::core::PixelBytes;

  /*
  TODO: 
  - when lhs and rhs have different values for max_value, there can be problems
  - check for zero dimension images
  */

  let mut lhs_copy = lhs.clone();
  let mut rhs_copy = rhs.clone();

  // determine the dimensions for the new image
  let (w, h) = friendly_scale_match(
    lhs.width(), lhs.height(), 
    rhs.width(), rhs.height()
  );

  // if the two images are of different sizes
  if lhs.width() != rhs.width() || lhs.height() != rhs.height() {
    let left_resize = resize(
      lhs, w, h, 
      Some(ResizeAlgorithm::BilinearInterpolation)
    );

    let right_resize = resize(
      rhs, w, h, 
      Some(ResizeAlgorithm::BilinearInterpolation)
    );

    // if either resize operation failed, bubble up the error and stop
    if let Err(msg) = left_resize { return Err(msg); }
    if let Err(msg) = right_resize { return Err(msg); }

    lhs_copy = left_resize.ok().unwrap();
    rhs_copy = right_resize.ok().unwrap();
  }

  // container for the resulting image
  let mut new_image = PpmImage::new(w, h);

  // the number of pixels in the new image
  let ppm_pixel_capacity = (w * h) as usize;
    
  // declare a type for the pixel operation function
  type PixelOperation = fn(u8, u8) -> u8;

  // default to setting the operation function to add_safe
  let mut operation_fn: PixelOperation = add_safe;

  // depending on the optype, set the operation function
  match optype {
    OpType::Add => {}, // if the optype is Add, then we're already good
    OpType::Subtract => operation_fn = sub_safe,
    OpType::Multiply => operation_fn = mult_safe,
  }

  let mut pixel_index = 0;
  for i in 0..ppm_pixel_capacity {
    let mut output_pixel:PixelBytes<u8> = [0;PIXEL_SIZE];

    for ch in COLOR_CHANNELS {
      output_pixel[ch] = operation_fn(
        lhs_copy.get_pixel_at(i)[ch],
        rhs_copy.get_pixel_at(i)[ch]
      );
    }

    new_image.set_pixel(&mut pixel_index, &output_pixel);
  }
  // return the result of the operation
  Ok(new_image)
}

/* #endregion */

/* #region Image Scaling */

/**
 * Resizes an image given the image input object and a new width and height
 * 
 * Optionally include an indicator of what resizing algorithm to use, either
 * Nearest Neighbor, or Bilinear Interpolation.
 * 
 * If resize_algo is set to None, then the resize algorithm will be set to 
 * Nearest Neighbor
 */
pub fn resize(
  image: &PpmImage, 
  width: u32, 
  height:u32, 
  resize_algo:Option<ResizeAlgorithm>) -> OperationResult {
  
  let resize_algorithm = resize_algo.unwrap_or(
    ResizeAlgorithm::NearestNeighbor
  );
  
  // if the image already has the defined dimensions, then just return
  if image.width() == width && image.height() == height {
    // TODO: Is there a better way to handle this? 
    // If the image is the same size then a copy is made, can't we just return 
    // the image as-is, or return a flag saying the image does not need to be 
    // resized?
    // YES - you should go implement the Copy trait - only problem is that
    // this gets involved with some of the collection data structures that are
    // members of PpmImage
    // HOWEVER, it's possible that clone is sufficient. It's not *guaranteed* to
    // be as inexpensive as Copy, but you're not allowed to implement your own
    // Copy trait, so look into this.
    return Ok(image.clone()); 
  }

  // if the image is trying to be resized to zero zero, then just return
  if width == 0 || height == 0 {
    return Err("The image cannot have height or width be zero.".to_string())
  }

  // based on the resize algorithm to use, resize the image
  match resize_algorithm {
    ResizeAlgorithm::NearestNeighbor => {
      nearest_neighbor(image, width, height)
    },
    ResizeAlgorithm::BilinearInterpolation => {
      bilinear_interpolation(image, width, height)
    },
  }
}

/**
 * Perform a resize on the given image using the bilinear interpolation 
 * method
 */
fn bilinear_interpolation(
  image: &PpmImage, 
  width:u32, 
  height:u32) -> OperationResult {
  /**
   * Standard lerp function
   */
  fn lerp(s: f32, e: f32, t:f32) -> f32 {
    s + (e - s) * t
  }

  /**
   * lerp in two directions
   */
  fn blerp(xa: f32, xb: f32, ya: f32, yb: f32, tx: f32, ty: f32) -> f32 {
    lerp(lerp(xa, xb, tx), lerp(ya, yb, tx), ty)
  }
  
  fn get_pixel_as_float(image:&PpmImage, x:i32, y:i32) -> (f32, f32, f32) {
    if let Some(pixel) = image.get_pixel_by_coord(x as u32, y as u32) {
      (
        pixel[R_CH] as f32,
        pixel[G_CH] as f32,
        pixel[B_CH] as f32,
      )
    } else {
      (0.,0.,0.) // implicit zero-padding TODO: make this explicit
    }
  }
  let mut new_image = PpmImage::new(width, height);

  // cast the current dimensions to floats (because of math reasons)
  let fcur_width = image.width() as f32;
  let fcur_height = image.height() as f32;

  // cast the future dimensions to floats (because of math reasons)
  let fnew_width = width as f32;
  let fnew_height = height as f32;

  for y in 0..height {
    for x in 0..width {
      let gx = (x as f32) / fnew_width * (fcur_width - 1.0);
      let gy = (y as f32) / fnew_height * (fcur_height - 1.0);

      // get integer values of gx and gy
      let gxi = gx as i32;
      let gyi = gy as i32;

      // extract the four colors of note
      let a = get_pixel_as_float(image, gxi, gyi);
      let b = get_pixel_as_float(image, gxi + 1, gyi);
      let c = get_pixel_as_float(image, gxi, gyi + 1);
      let d = get_pixel_as_float(image, gxi + 1, gyi + 1);

      // get the proper color values as f32
      let fr = blerp(a.0, b.0, c.0, d.0, gx - gxi as f32, gy - gyi as f32);
      let fg = blerp(a.1, b.1, c.1, d.1, gx - gxi as f32, gy - gyi as f32);
      let fb = blerp(a.2, b.2, c.2, d.2, gx - gxi as f32, gy - gyi as f32);

      // round and clamp to 255 max
      let r = clamp_color(fr.round() as u32);
      let g = clamp_color(fg.round() as u32);
      let b = clamp_color(fb.round() as u32);

      // push the new pixel onto the new image
      new_image.set_pixel_by_coord(x as u32, y as u32, &[r, g, b]);
    }
  }
  
  // set the new image
  Ok(new_image)
}

/**
 * Perform a resize on the given image using the nearest neighbor algorithm
 */
fn nearest_neighbor(
  image: &PpmImage, 
  width: u32, 
  height: u32) -> OperationResult {
  let mut new_image = PpmImage::new(width, height);
  
  let x_ratio = image.width() as f32 / width as f32;
  let y_ratio = image.height() as f32 / height as f32;

  for i in 0..height {
    for j in 0..width {
      let px = (j as f32 * x_ratio).floor();
      let py = (i as f32 * y_ratio).floor();

      let pixel = image.get_pixel_at(
        ((py * image.width() as f32) + px) as usize
      );
      
      new_image.set_pixel_by_coord(j, i, &pixel);
    }
  }

  // replace the old ppm with the new one
  Ok(new_image)
}

/* #endregion */

/**
 * Negates an image 
 */
pub fn negate(image: &PpmImage) -> OperationResult {

  let mut negated_image = PpmImage::new(
    image.width(), 
    image.height()
  );



  negated_image.hint_max_value(image.max_value() as u16);
  
  let mut pixel_index:usize = 0;
  for bytes in image.get_data().chunks_exact(PIXEL_SIZE) {
    let negated_pixel = [
      image.max_value() - bytes[R_CH], 
      image.max_value() - bytes[G_CH], 
      image.max_value() - bytes[B_CH]
    ];

    negated_image.set_pixel(&mut pixel_index, &negated_pixel);
  }
  
  Ok(negated_image)
}

pub fn histogram_equalization(
  image: &PpmImage, 
  target_histogram: Option<Histogram>) -> OperationResult {

  // are we equalizing to ourself, or using a given histogram?
  let equalizing_to_self = None == target_histogram;
  
  // this will be the histogram that we actually use
  let mut histogram = if equalizing_to_self { 
    Histogram::new()
  } else { 
    target_histogram.unwrap()
  };

  let pixel_count = (image.width() * image.height()) as usize;

  // create a vector to hold the HSV version of the pixels
  let mut hsv_pixels = Vec::<HSVPixel>::with_capacity(
    pixel_count
  );
  
  // convert the pixels to HSV so we can do histogram equalization on color
  // images (even though this implementation kinda sucks)
  for y in 0..image.height() {
    for x in 0..image.width() {
      if let Some(pixel) = image.get_pixel_by_coord(x, y) {
        // get the pixel as hsv
        let hsv_pixel = color::rgb_to_hsv(pixel);
        // put the actual pixel in the right spot
        hsv_pixels.push(hsv_pixel);  
        // if we're equalizing to self, then we need to update the histogram
        // for each pixel as well
        if equalizing_to_self {
          histogram.add(&hsv_pixel[V_CH]);
        }
      }
    }
  }

  let intensity_eq = histogram.equalize();

  let mut equalized_image = PpmImage::new(
    image.width(), 
    image.height()
  );

  let mut pixel_index:usize = 0;
  for hsv_pixel in hsv_pixels {

    let orig_key = Histogram::downsample_float(hsv_pixel[V_CH]);

    if let Some(equalized_value) = intensity_eq.get(&orig_key) {

      let rgb = color::hsv_to_rgb(
        hsv_pixel[H_CH], 
        hsv_pixel[S_CH], 
        *equalized_value / V_MULT as f32
      );

      equalized_image.set_pixel(&mut pixel_index, &rgb);
    }
  }

  Ok(equalized_image)
}


/* #region Utility Functions */


/**
 * Special clamp function for color values between 0 and 255
 */
fn clamp_color(num: u32) -> u8 {
  if num > u8::MAX as u32 {
    return u8::MAX
  }
  
  num as u8
}

// Gets the dimensions that are between the two given dimensions
const fn friendly_scale_match(
  w1: u32, h1: u32, 
  w2: u32, h2: u32) -> (u32, u32) {
  if w1 == w2 && h1 == h2 {
    return (w1, h1)
  }
  let w_new = (w1 + w2) / 2;
  let h_new = (h1 + h2) / 2;

  (w_new, h_new)
}

/* #endregion */