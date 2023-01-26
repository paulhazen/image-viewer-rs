use std::{fmt, collections::{BTreeMap, HashMap}};

use crate::core::{PixelBytes, PIXEL_SIZE, max, min};

use super::color::BLACK;

#[derive(PartialEq, Clone, Copy)]
pub enum Padding {
  Zero,
  Repeat
}

/* #region PPM object        */
#[derive(Debug, Clone)]
pub struct PpmImage {
    header: PpmHeader,
    pixels: Vec<u8>,
    histogram: HashMap<PixelBytes<u8>, usize>,
    rgb_components_used: BTreeMap<u8, usize>,
    pub keep_histogram_updated: bool,
}

impl PpmImage {
  
  pub fn new(width: u32, height: u32) -> Self {
    
    let capacity: usize = PIXEL_SIZE * (height * width) as usize;

    let mut histogram = HashMap::<PixelBytes<u8>, usize>::with_capacity(u8::MAX as usize);

    histogram.insert(BLACK, capacity);

    PpmImage {
        header: PpmHeader::new(width, height),
        pixels: vec![0;capacity],
        histogram: histogram,
        rgb_components_used: BTreeMap::new(),
        keep_histogram_updated: false,
    }
  }

  pub fn get_background(&self) -> PixelBytes<u8> {
    use crate::core::get_key_with_max_value;

    *get_key_with_max_value(&self.histogram)
  }

  pub fn get_data(&self) -> &[u8] {
    &self.pixels
  }

  /* #region Header Accessors / Modifier functions */

  pub const fn height(&self) -> u32 {
    self.header.height
  }

  pub const fn width(&self) -> u32 {
    self.header.width
  }

  pub fn max_value(&self) -> u8 {
    if let Some(key_val) = self.rgb_components_used.iter().last() {
      *key_val.0
    } else {
      u8::MAX
    }
  }

  pub const fn ppm_type(&self) -> PpmType {
    self.header.ppm_type
  }

  pub fn hint_max_value(&mut self, max_value: u16) {
    self.header.max_value = max_value
  }

  pub fn set_header(&mut self, header:PpmHeader) {
    self.header = header
  }
  /* #endregion */

  /* #region Setting Pixels  */
  
  fn remove_from_hist(&mut self, rgb: &[u8]) {
    use crate::core::{R_CH, B_CH};
    if let Some(count) = self.histogram.get_mut(rgb) {
      *count -= 1;

      if *count == 0 {
        self.histogram.remove(rgb);
      }

      for ch in R_CH..B_CH {
        if let Some(value_count) = self.rgb_components_used.get_mut(
          &rgb[ch]
        ) {
          *value_count -= 1;

          if *value_count == 0 {
            self.rgb_components_used.remove(&rgb[ch]);
          }
        }
      }
    }
  }

  fn add_to_hist(&mut self, rgb: &[u8]) {

    use crate::core::{R_CH, G_CH, B_CH};

    if let Some(count) = self.histogram.get_mut(rgb) {
      *count += 1;

      for ch in R_CH..B_CH {
        if let Some(value_count) = self.rgb_components_used.get_mut(
          &rgb[ch]
        ) {
          *value_count += 1;
        }
      }
    } else {
      let pixel:Result<PixelBytes<u8>, _> = rgb.try_into();

      if let Ok(pixel_arr) = pixel {
        self.histogram.insert(pixel_arr, 1);

        self.rgb_components_used.insert(pixel_arr[R_CH], 1);
        self.rgb_components_used.insert(pixel_arr[G_CH], 1);
        self.rgb_components_used.insert(pixel_arr[B_CH], 1);
      } else {
        // TODO: Handle this more gracefully
        println!("Couldn't convert slice to array.");
      }

      
    }
  }

  pub fn set_pixel(&mut self, index:&mut usize, pixel:&[u8]) {
    use crate::core::{R_CH, G_CH, B_CH};

    let removed_pixel = [
      self.pixels[*index + R_CH],
      self.pixels[*index + G_CH],
      self.pixels[*index + B_CH],
    ];

    self.remove_from_hist(&removed_pixel);

    for ch in R_CH..(B_CH + 1) {
      self.pixels[*index + ch] = pixel[ch];
    }

    self.add_to_hist(pixel);

    // increment index by pixel size
    *index += PIXEL_SIZE;
  }

  pub fn set_pixel_by_coord(&mut self, x:u32, y:u32, pixel:&[u8]) {
    let mut index = get_index(x as i32, y as i32, self.width());
    self.set_pixel(&mut index, pixel);
  }

  /* #endregion */

  /* #region Getting Pixels */

  pub fn get_matrix_at(
    &self, x:u32, y:u32, size:usize, padding:Padding
  ) -> Vec<&[u8]> {
    assert!(size % 2 != 0); // size must be odd for it to be centered on (x, y)

    let start_delta = -1 * (size as i32 - 1) / 2;

    let start_x = x as i32 + start_delta;
    let start_y = y as i32 + start_delta;

    let mut matrix = Vec::<&[u8]>::with_capacity(size * size);

    for x in start_x..(start_x + size as i32) {
      for y in start_y..(start_y + size as i32) {
        // if we are using repeat padding
        if padding == Padding::Repeat {
          // if x or why is below 0, then set it to zero
          let mut x_adj = max(x, 0);
          let mut y_adj = max(y, 0);
          
          // if x or y is greater than the highest possible, then set it to 
          // the highest possible
          x_adj = min(x_adj, self.width() as i32 - 1);
          y_adj = min(y_adj, self.height() as i32 - 1);
          
          matrix.push(self.get_pixel_by_coord_ref(x_adj as u32, y_adj as u32));
        } else if padding == Padding::Zero {
          if x < 0 || y < 0 ||
             x as u32 >= self.width() || y as u32 >= self.height() {
            matrix.push(&[0, 0, 0]); 
          } else {
            matrix.push(self.get_pixel_by_coord_ref(x as u32, y as u32));
          }
        }
      }
    }

    return matrix
  }

  /// This gets a pixel, where index points to the "r" byte in the array
  /// The assumption is that the following two pixels are (in order): G, B
  pub fn get_bytes_at(&self, index:usize) -> PixelBytes<u8> {
    [
      self.pixels[index],
      self.pixels[index + 1],
      self.pixels[index + 2],
    ]
  }

  pub fn get_pixel_at(&self, index:usize) -> PixelBytes<u8> {
    self.get_bytes_at(index * PIXEL_SIZE)
  }

  /// Gets a byte array representing a pixel at the x and y coordinate indicated
  /// the coordinate system has (0, 0) in the upper left hand of the image
  pub fn get_pixel_by_coord(&self, x: u32, y: u32) -> Option<PixelBytes<u8>> {
    let index = get_index(x as i32, y as i32, self.width());
    if index < self.pixels.len() {
      Some(self.get_bytes_at(index))
    } else {
      None
    }
  }

  pub fn get_pixel_by_coord_ref(&self, x:u32, y:u32) -> &[u8] {
    let index = get_index(x as i32, y as i32, self.width());
    
    return &self.pixels[index..(index + PIXEL_SIZE)];
  }

  /* #endregion */

  /**
   * Create a PPM image with the given RGB values and the given height and
   * width. This is primarily for purposes of testing
   */
  #[cfg(test)]
  pub fn create_color(r: u8, g: u8, b: u8, height: u32, width: u32) -> Self {
    let pixel_count: usize = (height * width) as usize;

    let mut header = PpmHeader::new(width, height);
    header.height = height;
    header.width = width;

    let max_value = max(max(r, g), b) as u16;

    header.max_value = max_value;
    
    let mut single_color_image = PpmImage {
      header: header,
      pixels: vec![0;pixel_count * PIXEL_SIZE],
      histogram: HashMap::new(),
      rgb_components_used: BTreeMap::new(),
      keep_histogram_updated: false
    };

    let mut pixel_index:usize = 0;
    for _ in 0..pixel_count {
      single_color_image.set_pixel(&mut pixel_index, &[r, g, b]);
    }

    single_color_image
  }
}

impl fmt::Display for PpmImage {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}x{}, max_value = {}", 
      self.width(), 
      self.height(), 
      self.max_value())
  }
}

impl PartialEq for PpmImage {
  fn eq(&self, other: &PpmImage) -> bool {
    let mut is_equal = true;

    if self.header != other.header {
      is_equal = false;
    } else {
      if self.pixels.len() != other.pixels.len() {
        is_equal = false;
      } else {
        for i in 0..self.pixels.len() {
          if self.pixels[i] != other.pixels[i] {
            is_equal = false;
            break;
          }
        }
      }
    }

    is_equal
  }
}
/* #endregion */

/* #region PPMHeader         */
#[derive(Debug, Clone)]
pub struct PpmHeader {
  pub ppm_type: PpmType,
  pub width: u32,
  pub height: u32,
  pub max_value: u16,
}

impl PpmHeader {
  pub fn new(width: u32, height: u32) -> Self {
    PpmHeader {
      ppm_type: PpmType::P6,
      width: width,
      height: height,
      max_value: 0,
    }
  }
}

impl PartialEq for PpmHeader {
  fn eq(&self, other: &PpmHeader) -> bool {
    self.height == other.height &&
    self.width == other.width
    //self.max_value == other.max_value
    // ppm type is excluded from partial equality because the images are 
    // considered to be equivalent, even if their types don't match
    // self.ppm_type == other.ppm_type 
    // we also do not consider the data_position to be a meaningful
    // differentiator, so it is excluded from equality comparison
    // self.data_position == other.data_position
    // furthermore, we do not consider the file name to be a part of equality
    // the image data is the only part that is considered
    // self.filepath == other.filepath
  }
}

/* #endregion */

/* #region PpmType           */

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PpmType {
    /// P1 is the Bitmap Data in ASCII
    P1,
    /// P2 is the Grayscale Data in ASCII
    P2,
    /// P3 is the RGB Image data in ASCII
    P3,
    /// P4 is the Bitmap Data in Binary Format
    P4,
    /// P5 is the Grayscale Data in Binary Format
    P5,
    /// P6 is the RGB Image Data in Binary Format
    P6,
    /// This is not a valid PPM/PGM/PBM File Format 
    P0,
}

impl fmt::Display for PpmType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      PpmType::P0 => write!(f, "P0"),
      PpmType::P1 => write!(f, "P1"),
      PpmType::P2 => write!(f, "P2"),
      PpmType::P3 => write!(f, "P3"),
      PpmType::P4 => write!(f, "P4"),
      PpmType::P5 => write!(f, "P5"),
      PpmType::P6 => write!(f, "P6"),
    }
  }
}

/* #endregion */

/* #region Utility Functions */

/**
 * Utility function to convert x/y coordinates in an image to array index, 
 * given the width of the image.
 */
const fn get_index(x:i32, y:i32, w:u32)-> usize {
  // the pixels are stored in a one dimensional array, and the pixels are in the
  // array scanning the image from left to right starting at the top row, and 
  // moving to the bottom. Therefore, the index of an (x, y) in the image can
  // be calculated by adding to the x value, the y value times the image width
  let mut index: u32 = x as u32;
  index += y as u32 * w as u32;
  
  // remember also that we store three bytes for each pixel, so the real index
  // to return will be multiplied by three (that is, the index returned by this
  // function will be the index of the 'r' value for the pixel)
  PIXEL_SIZE * index as usize
}

/* #endregion */