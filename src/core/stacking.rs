

use std::{collections::HashMap, rc::Rc};

use image::{DynamicImage, ImageBuffer, Rgb};
use palette::encoding::pixel;

use super::{PIXEL_SIZE, R_CH, G_CH, B_CH, color::{self}, V_CH, min, max, io::read_raw};

pub type StackingFunction = dyn Fn(&Vec<Rgb<u16>>) -> Rgb<u16>;

pub type ChunkBuffer = ImageBuffer<Rgb<u16>, Vec<u16>>;
/// This stores the percentage of system memory that is allowed to be allocated
/// for stacking operations. Ultimately, this will determine how large of a 
/// chunk each calibration frame is processed at one time, on a stack by stack
/// basis
pub const STACKING_MEMORY_USAGE:f32 = 0.25;

/// This is a constant that is relevant to the input images given for this
/// project - when functionality is expanded to other types of raw input files,
/// this value will have to change
pub const RAW_BYTES_PER_PIXEL: usize = 6;

#[derive(Clone)]
pub enum ClippingStrategy {
  Remove,
  ReplaceWithMedian
}

pub trait StackOperation {
  fn get_function(&self) -> &Box<StackingFunction>;
}

pub struct ImageStack {
  pub stacking_fns: Vec<(Box<dyn StackOperation>, String)>,
  width: u32,
  height: u32,
  pub data: Vec<String>
}

impl ImageStack {

  pub fn add_algorithm(&mut self, algorithm: Box<dyn StackOperation>, filename: String) {
    self.stacking_fns.push((algorithm, filename));
  }

  pub fn new() -> Self {
    ImageStack {
      stacking_fns: Vec::new(),
      width: 0,
      height: 0,
      data: Vec::new()
    }
  }

  pub fn process_stack(&self) {

    if 0 == self.width() || 0 == self.height() {
      panic!("The stack does not have an image size set (perhaps images were not read properly)");
    }

    let total_width = self.width();
    let total_height = self.height();

    // get the dimensions of the chunks that each image in the stack will need
    // to be broken up into
    let (chunk_dimensions, chunk_cols, chunk_rows) = find_dimensions_that_match_mem_requirements(
      total_width, 
      total_height, 
      self.data.len()
    );

    // this should never really happen, but the check is for safety
    if None == chunk_dimensions || 0 == chunk_rows || 0 == chunk_cols {
      panic!("Could not find chunk dimensions that satisfy memory requirements");
    }

    // more convenient way to reference chunk height and width
    let chunk_width = chunk_dimensions.unwrap().0;
    let chunk_height = chunk_dimensions.unwrap().1;

    // total number of chunks each image will be split into
    let chunk_count = (chunk_rows * chunk_cols) as usize;
    let mut chunks_processed = 0;

    // the number of pixels per chunk
    let pixels_per_chunk = (chunk_width * chunk_height) as usize;

    let stack_depth = self.data.len();

    // create a master frame for each stacking algorithm function
    let mut master_frames: Vec<ChunkBuffer> = Vec::with_capacity(stack_depth);
    for _ in 0..self.stacking_fns.len() {
      master_frames.push(ImageBuffer::new (total_width, total_height));
    }

    // stores the slice of pixels from the stack
    let mut pixel_slice: Vec<Rgb<u16>> = vec![Rgb::<u16>::from([0, 0, 0]); stack_depth];

    // for each chunk
    for chunk_row in 0..chunk_rows {
      for chunk_col in 0..chunk_cols {

        // get the upper left (x, y) corner of the current chunk
        let offset_x = chunk_col * chunk_width;
        let offset_y = chunk_row * chunk_height;

        // get chunks from stack
        let mut chunks_from_stack: Vec<ChunkBuffer> = Vec::with_capacity(chunk_count);
        for image_path in &self.data {
          if let Some(image_chunk) = get_image_chunk(
            image_path.as_str(), offset_x, offset_y, chunk_width, chunk_height
          ) {
            chunks_from_stack.push(image_chunk);
          }
        }

        println!("Chunk ({}, {}) has been loaded into memory", chunk_row, chunk_col);

        // for each pixel across the whole stack
        for x in 0..chunk_width {
          for y in 0..chunk_height  {

            // extract the slice of pixels
            for stack_index in 0..stack_depth {
              
              // NOTE: This will break if the RAW image is not rgb16
              pixel_slice[stack_index] = *chunks_from_stack[stack_index].get_pixel(
                x as u32,
                y as u32
              );
            }

            for frame_index in 0..master_frames.len() {
              let master_pixel = (self.stacking_fns[frame_index].0.get_function())(&pixel_slice);
              master_frames[frame_index].put_pixel(offset_x + x, offset_y + y, master_pixel);
            }
          }
        }

        chunks_processed += 1;
        println!("chunk {} out of {} completed", chunks_processed, chunk_count);
      }
    }

    for frame_index in 0..master_frames.len() {
      master_frames[frame_index].save(self.stacking_fns[frame_index].1.as_str());
    }
  }

  pub fn add_image(&mut self, path: &str) {
    if 0 == self.width || 0 == self.height {
      if let Some(image) = read_raw(path) {
        self.width = image.width();
        self.height = image.height();
      }
    }

    // add the file path to the list of image file paths
    self.data.push(path.to_string());
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn width(&self) -> u32 {
    self.width
  }
}

/* #region Average Stack */

pub struct Average {
  stacking_function: Box<StackingFunction>
}

impl Average {
  pub fn new() -> Self {
    fn stack_algorithm(pixels: &Vec<Rgb<u16>>) -> Rgb<u16> {
      let mut r_sum: usize = 0;
      let mut g_sum: usize = 0;
      let mut b_sum: usize = 0;

      for pixel in pixels {
        r_sum += pixel.0[R_CH] as usize;
        g_sum += pixel.0[G_CH] as usize;
        b_sum += pixel.0[B_CH] as usize;
      }

      Rgb::<u16>::from([
        (r_sum as usize / pixels.len()) as u16,
        (g_sum as usize / pixels.len()) as u16,
        (b_sum as usize / pixels.len()) as u16,
      ])
    }

    Average {
      stacking_function: Box::new(stack_algorithm)
    }
  }
}

impl StackOperation for Average {
  fn get_function(&self) -> &Box<StackingFunction> {
      &self.stacking_function
  }
}

/* #endregion */

/* #region Median Stack */

pub struct Median {
  //image_list: ImageList,
  stack_function: Box<StackingFunction>,
}

impl Median {
  fn stack_algorithm(pixels: &Vec<Rgb<u16>>) -> Rgb<u16> {
    let mut intensity_to_rgb = HashMap::<u16, &Rgb<u16>>::new();

    for pixel in pixels {
      let intensity = color::rgb_to_hsv(pixel.0)[V_CH];
      intensity_to_rgb.insert(
        (intensity * u16::MAX as f32) as u16,
         pixel
      );
    }

    let mut intensities: Vec<&u16> = intensity_to_rgb.keys().collect();
    intensities.sort();

    let pixel_key = intensities[intensities.len() / 2];

    **intensity_to_rgb.get(pixel_key).unwrap()
  }

  pub fn new() -> Self {
    Median {
      //image_list: ImageList::new(Box::new(Median::stack_algorithm)),
      stack_function: Box::new(Median::stack_algorithm)
    }
  }
}

impl StackOperation for Median {
  fn get_function(&self) -> &Box<StackingFunction> {
    &self.stack_function
  }
}

/* #endregion */

/* #region Maximum Stack */

pub struct Maximum {
  pub stacking_function: Box<StackingFunction>
}

impl Maximum {
  pub fn new() -> Self {
    fn stack_algorithm(pixels: &Vec<Rgb<u16>>) -> Rgb<u16> {
      let mut max_intensity = 0.0;
      let mut max_pixel = Rgb::<u16>::from([0,0,0]);

      for pixel in pixels {
        let intensity = color::rgb_to_hsv(pixel.0)[V_CH];

        if intensity > max_intensity {
          max_intensity = intensity;
          max_pixel = *pixel;
        }
      }

      max_pixel
    }

    Maximum {
      stacking_function: Box::new(stack_algorithm)
    }
  }
}

impl StackOperation for Maximum {
  fn get_function(&self) -> &Box<StackingFunction> {
      &self.stacking_function
  }
}

/* #endregion */

/* #region KappaSigmaClipping */

pub struct KappaSigmaClipping {
  //image_list: ImageList
  stacking_function: Box<StackingFunction>
}

impl KappaSigmaClipping {

  pub fn new(iterations: usize, kappa: f64, strategy: ClippingStrategy) -> Self {

    let stacking_algorithm = move |pixels: &Vec<Rgb<u16>>| -> Rgb<u16> {

      let mut pixels_in_stack = pixels.clone();
      let pixel_count = pixels_in_stack.len();
      let iterations_real = min(iterations, pixel_count);

      for _ in 0..iterations_real {
        // calculate the pixel intensities for the pixels in the stack. This 
        // will create a parallel array, such that each entry in 
        // pixel_intensities corresponds to each entry in pixels_in_stack
        let mut pixel_intensities: Vec<u16> = Vec::with_capacity(pixel_count);
        for pixel in &pixels_in_stack {
          let hsv_pixel = color::rgb_to_hsv([
            pixel.0[R_CH] as f32 / u16::MAX as f32,
            pixel.0[G_CH] as f32 / u16::MAX as f32,
            pixel.0[B_CH] as f32 / u16::MAX as f32,
          ]);

          pixel_intensities.push((hsv_pixel[V_CH] * u16::MAX as f32) as u16);
        }

        // find the index of the intensity value to exclude
        if let Some(to_clip) = find_intensity_to_clip(&pixel_intensities, &kappa) {
          match strategy {
            ClippingStrategy::Remove => {
              pixels_in_stack.remove(to_clip); 
            },
            ClippingStrategy::ReplaceWithMedian => {
              let mut intensity_mapping = HashMap::<u16, usize>::new();
              let mut intensity_index = 0;
              for intensity in pixel_intensities {
                intensity_mapping.insert(intensity, intensity_index);
                intensity_index += 1;
              }

              let intensities: Vec<&u16> = intensity_mapping.keys().collect();
              let median_intensity = intensities[intensities.len() / 2];

              let index_of_median_pixel = intensity_mapping.get(median_intensity).unwrap();

              pixels_in_stack[to_clip] = pixels_in_stack[*index_of_median_pixel];
            },
          }
        }
      }

      pixels_in_stack[0]
    };

    KappaSigmaClipping { 
      stacking_function: Box::new(stacking_algorithm)
    }
  }
  

  
}

impl StackOperation for KappaSigmaClipping {
  fn get_function(&self) -> &Box<StackingFunction> {
    &self.stacking_function
  }
}
  
/* #endregion */

/* #region Utility Functions */

fn get_image_chunk(path: &str, x: u32, y: u32, width: u32, height: u32) -> Option<ChunkBuffer> {

  let mut image_chunk: Option<ChunkBuffer> = None;

  if let Some(image) = read_raw(path) {
    image_chunk = Some(
      image.crop_imm(
        x as u32, y as u32,
        width as u32, height as u32
      ).as_rgb16().unwrap().clone()
    );

    // dunno if this helps or not
    drop(image);
  }

  image_chunk
}

fn get_system_memory() -> u64 {
  use sysinfo::{System, SystemExt};

  let s = System::new_all();
  
  s.total_memory()
}

fn get_factors(n: u32) -> Vec<u32> {
  (1..n + 1).into_iter().filter(|&x| n % x == 0).collect::<Vec<u32>>()
}

fn find_intensity_to_clip(
  intensities: &Vec<u16>,
  kappa: &f64,
) -> Option<usize> {
  let mut to_clip: Option<usize> = None;
  
  let mut sum:f64 = 0.0;
  for intensity in intensities {
    sum += *intensity as f64;
  }

  let mean = sum / intensities.len() as f64;

  let variance: f64 = intensities.iter().map(|x| (*x as f64 - mean).powi(2)).sum();
  let standard_deviation = variance.sqrt();

  let mut furthest_distance: f64 = 0.0;

  for intensity_index in 0..intensities.len() {
    let intensity = intensities[intensity_index] as f64;
    if kappa * standard_deviation < intensity {
      let distance = (mean - intensity).abs();
      if furthest_distance < distance {
        furthest_distance = distance;
        to_clip = Some(intensity_index);
      }
    }
  }

  to_clip
}

fn find_dimensions_that_match_mem_requirements(total_width: u32, total_height: u32, image_count: usize) -> (Option<(u32, u32)>, u32, u32) {

  let sys_mem = get_system_memory() as f32;
  let mem_limit = (sys_mem * STACKING_MEMORY_USAGE).round() as u64;

  let mut height_factors = get_factors(total_height);
  let mut width_factors = get_factors(total_width);


  // reverse the factor lists, because we're going to try and use the largest
  // image chunk size that we can while staying beneath our memory limit
  height_factors.reverse();
  width_factors.reverse();

  // get the index of the largest factor
  let largest_factor_count = max(height_factors.len(), width_factors.len());

  let mut chunk_height = height_factors[0];
  let mut chunk_width = width_factors[0];
  let mut factor_index = 0;

  let mut memory_usage = chunk_height as u64 * chunk_width as u64 * (image_count * RAW_BYTES_PER_PIXEL) as u64;

  // keep lowering which factor of height and width to use until memory 
  // requirements are met
  while memory_usage > mem_limit {

    // get the next height factor
    chunk_height = if factor_index < height_factors.len() {
      height_factors[factor_index]
    } else {
      *height_factors.last().unwrap()
    };

    // get the next width factor
    chunk_width = if factor_index < width_factors.len() {
      width_factors[factor_index]
    } else {
      *width_factors.last().unwrap()
    };

    // update the memory usage candidate
    memory_usage = chunk_height as u64 * chunk_width as u64 * (image_count * RAW_BYTES_PER_PIXEL) as u64;

    // increment the factor index
    factor_index += 1;

    // this should practically never happen, but it's here for safety
    if factor_index >= largest_factor_count {
      return (None, 0, 0);
    }
  }

  // get the number of rows and columns in the chunk grid (not zero based)
  let chunk_cols = total_width / chunk_width;
  let chunk_rows = total_height / chunk_height;

  println!("Chunk grid dimensions ({}, {})", chunk_rows, chunk_cols);

  (Some((chunk_width, chunk_height)), chunk_cols, chunk_rows)
}
/* #endregion */