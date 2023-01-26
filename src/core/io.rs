use crate::core::ppm::{PpmImage, PpmType, PpmHeader};
use std::fs::File;
use std::io::{BufReader, Read, BufWriter, Write};
use std::str::FromStr;
use std::default::Default;
use image::DynamicImage;
use image::io::Reader as ImageReader;

use super::PIXEL_SIZE;
use super::cr2::read_cr2;

/* #region Types and Constants */

/// IOResult is used in functions where the result should be a PpmImage. If 
/// during the course of the function an error is encountered, and a PpmImage
/// cannot be returned, a String is returned instead that contains a message
/// describing the nature of the problem
pub type IOResult = Result<PpmImage, String>;

/// Carriage Return character
const CR:[u8;1] = [13];

/// Line feed character
const LF:[u8;1] = [10];

/// SPACE character
const SPACE:[u8;1] = [32];

/// Comment character
const COMMENT:[u8;1] = [35];

/// Vertical tab character
const VTAB:[u8;1] = [11];

/// Horizontal Tab character
const HTAB:[u8;1] = [9];

/// Feed-forward character
const FF:[u8;1] = [12];

/// All characters that are considered whitespace by the PPM Spec: CR, LF, VTAB,
/// HTAB, FF
const WHITESPACES: [[u8; 1]; 6] = [CR, LF, SPACE, VTAB, HTAB, FF];



/* #endregion */

/* #region Reading Images */

pub fn open_image(path: &str) -> IOResult {
  let p = std::path::Path::new(path);
  
  if !p.exists() {
    return Err(format!("Could not find file: \"{path}\""));
  }

  if let Some(ext) = p.extension() {
    match ext.to_str().unwrap().to_lowercase().as_str() {
      "ppm" => { return read_ppm(path); },
      "cr2" => { return read_cr2(path); },
      _ => { return read_other(path); }
    }
  } else {
    return Err(format!("File \"{path}\" has no file extension, cannot read."));
  }
}

pub fn read_raw(path: &str) -> Option<DynamicImage> {
  if let Ok(img) = ImageReader::open(path).unwrap().decode() {
    return Some(img)
  } else {
    return None
  }
}

fn read_other(path: &str) -> IOResult {
  if let Ok(img) = ImageReader::open(path).unwrap().decode() {
    let rgb8 = img.to_rgb8();
    let mut img_ppm = PpmImage::new(img.width(), img.height());

    let mut pixel_index:usize = 0;
    for t in rgb8.chunks_exact(PIXEL_SIZE) {
      img_ppm.set_pixel(&mut pixel_index, &t);
    }

    return Ok(img_ppm);
  } else {
    return Err(format!("Could not open file: \"{path}\""));
  }
}

fn read_ppm(path: &str) -> IOResult {

  if let Ok(mut file) = File::open(path) {
    let header = read_ppm_header(&mut file);
    let mut ppm = PpmImage::new(header.width, header.height);
  
    ppm.set_header(header);
    
    match ppm.ppm_type() {
      PpmType::P1 | PpmType::P2 | PpmType::P3 => { // ASCII formatted
        read_ppm_ascii_file(&mut ppm, &mut file);
      },
      PpmType::P4 | PpmType::P5 | PpmType::P6 => { // Binary formatted
        read_ppm_binary_image_data(&mut ppm, &mut file);
      }
      _ => {
        return Err(format!("PPM file structure in file: \"{path}\" is corrupted"))
      }
    }
    
    return Ok(ppm)
  } else {
    return Err(format!("Could not open file: \"{path}\""))
  }
  
}

fn read_ppm_header(file: &mut File) -> PpmHeader {
    let mut magic_number = [0; 2];
    
    /*#region Get the type of PPM file */

    // Get the type of PPM file we are reading
    file.read_exact(&mut magic_number).unwrap();
    let ppm_type = match magic_number {
      [80, 49] => { PpmType::P1 },
      [80, 50] => { PpmType::P2 },
      [80, 51] => { PpmType::P3 },
      [80, 52] => { PpmType::P4 },
      [80, 53] => { PpmType::P5 },
      [80, 54] => { PpmType::P6 },
      _ => { PpmType::P0 }
    };
    let ppm_type = ppm_type;

    /* #endregion */

    let width = read_number_ascii::<u32>(file);
    let height = read_number_ascii::<u32>(file);
    let max_value = read_number_ascii::<u16>(file);
    
    if max_value > 255 {
      panic!(
        "Cannot support PPM files with maxvalue greater than 255"
      );
    }

    PpmHeader {
      ppm_type: ppm_type,
      width: width,
      height: height,
      max_value: max_value,
    }
}

fn read_ppm_binary_image_data(image: &mut PpmImage, file: &mut File) {

    match image.ppm_type() {
      PpmType::P6 => {
        let mut b = [0; PIXEL_SIZE];
        let mut pixel_index:usize = 0;

        let mut overflow_count:usize = 0;
        while let Ok(n) = file.read(&mut b) {
          if 0 == n { break;}  

          if pixel_index >= image.get_data().len() {
            overflow_count += 1;
          } else {
            image.set_pixel(&mut pixel_index, &b);
          }
        }
        
        if overflow_count > 0 {
          panic!("Overflowed image buffer when reading from file 
          (means that there was more data in the file than there 
            should have been");
        }
      },
      PpmType::P5 => {
        let mut byte_for = [0; 1];
        let mut pixel_index:usize = 0;
        while let Ok(n) = file.read(&mut byte_for) {
          if 0 == n { break; }
          // TODO: Since we only implement 8 bit images - this code will fail
          let gs_data = u32::from_be_bytes([0,0,0,byte_for[0]]);
          
          let pixel = [
            ((gs_data as f32 / image.max_value() as f32) * 255.0) as u8,
            ((gs_data as f32 / image.max_value() as f32) * 255.0) as u8,
            ((gs_data as f32 / image.max_value() as f32) * 255.0) as u8
          ];

          image.set_pixel(&mut pixel_index, &pixel);

        }
      },
      PpmType::P4 => {
        let mut byte_buff:[u8; 1] = [0];
        let mut pixel_index:usize = 0;
        while let Ok(n) = file.read(&mut byte_buff) {
          let byte = byte_buff[0];
          if 0 == n { break; }
          for i in 8..0 {
            let pixel = if 1 == byte & (1 << i) {
              [255;PIXEL_SIZE]
            } else {
              [0;PIXEL_SIZE]
            };

            image.set_pixel(&mut pixel_index, &pixel);
          }
        }
      },
      _ => { panic!("Improperly formatted PPM file"); }
    }
}

// for P1, P2, and P3 images
fn read_ppm_ascii_file(ppm: &mut PpmImage, file: &mut File) {
  let mut reader = BufReader::new(file);
  let pixel_count: usize = (ppm.height() * ppm.width()) as usize;
  let mut current_pixel:usize = 0;

  let mut contents: String = "".to_string();
  
  let rts_result = reader.read_to_string(
    &mut contents
  );

  if rts_result.is_err() {
    // TODO: Do not panic here, return a sensible result instead
    panic!("Could not read image contents to string");
  }

  let pieces = contents.split_whitespace();
  
  let mut v = Vec::new();
  
  // TODO: Do not panic here - return a sensible result instead
  for p in pieces {
    match p {
      "" => panic!("Empty string!"),
      " " => panic!("Basically empty string!"),
      _ => v.push(p.to_string().parse::<u8>().unwrap())
    }
  }

  let mut i = 0;
  let mut pixel_index:usize = 0;
  while current_pixel < pixel_count {

    ppm.set_pixel(&mut pixel_index, &v[i..(i + PIXEL_SIZE)]);

    i += PIXEL_SIZE;

    current_pixel += 1;

}
}

/* #endregion */

/* #region Writing Images */

/**
 * Note that PPMs whenever written are going to be written as P6 (binary) files
 */
pub fn write_image(
  image: &PpmImage, filepath: &str
) -> Result<(), std::io::Error> {
  // TODO: Comments should be preserved between read and write. Currently this
  // is not supported. Also - comments inline with the image data cannot be 
  // preserved with the P6 Data type, so this may need some additional 
  // consideration.

  let path = std::path::Path::new(filepath);
  let display = path.display();

  let file = match File::create(&path) {
    Err(why) => panic!("Couldn't create {}: {}", display, why),
    Ok(file) => file,
  };

  let mut file_buffer = BufWriter::new(file);

  // build the image header here
  let mut header_str = PpmType::P6.to_string() + "\n";
  header_str.push_str(image.width().to_string().as_str());
  header_str.push_str(" ");
  header_str.push_str(image.height().to_string().as_str());
  header_str.push_str("\n");
  header_str.push_str(image.max_value().to_string().as_str());
  header_str.push_str("\n");
  

  match file_buffer.write(header_str.as_bytes()) {
    Err(why) => panic!("Couldn't write header to file buffer: {}", why),
    Ok(_) => {},
  }

  match file_buffer.write_all(image.get_data()){
    Err(why) => { panic!("Couldn't write to file buffer: {}", why)},
    Ok(_) => {},
  };

  let result = file_buffer.flush();

  result
}

/* #endregion */

/* #region Utility Functions */

/// Reads a file stream until one of the bytes provided in [until_bytes] is 
/// encountered, at which point the function returns. This equates to a sort
/// of "scan until" functionality
fn read_until(file: &mut File, until_bytes: Vec<[u8; 1]>) {
  let mut byte_read: [u8; 1] = [0];
  while let Ok(n) = file.read(&mut byte_read) {
    if 0 == n {
      break;
    }
    // if one of the bytes has been reached, then we stop here
    if until_bytes.contains(&byte_read) {
      break;
    }
  }
}

/// Reads a file stream until the bytes in the stream are *not* found in 
/// until_not_bytes. IF a byte is encountered that is inside the given vector of
/// characters, that character is returned so that it is not lost.
/// This is helpful primarily for parsing past comment lines in a PPM file.
fn read_until_not(file: &mut File, until_not_bytes: Vec<[u8;1]>) -> u8 {
  let mut byte_read: [u8; 1] = [0];
  while let Ok(n) = file.read(&mut byte_read) {
    if 0 == n { break; }

    // if we encounter a comment, we read until the character is a CR or LF
    // note that this is not just a whitespace, specifically the PPM spec
    // states that a comment line ends with CR or LF.
    if COMMENT == byte_read {
      read_until(file, [CR, LF].to_vec());

      // TODO: Don't println here - but maybe silently fail
      match file.read(&mut byte_read) {
        Err(why) => println!("Error reading file: {}", why),
        Ok(_) => {}
      }
    }

    // if the byte read does not 
    if false == until_not_bytes.contains(&byte_read) {
      break;
    }
  }

  byte_read[0]
}

/// Reads a number (type indicated by the templated variable "T", which must 
/// implement the "FromStr" and "Default" traits).
fn read_number_ascii<T : FromStr + Default>(file: &mut File) -> T {
  let mut ascii_number_bytes: Vec<u8> = Vec::<u8>::new();
  
  // read until it's not a whitespace
  ascii_number_bytes.push(
    read_until_not(file, WHITESPACES.to_vec())
  );

  // stores the current byte being read
  let mut byte_read: [u8;1] = [0];

  // while there are bytes to read
  while let Ok(n) = file.read(&mut byte_read) {
    // while more than one byte has been read, and that one byte is not a 
    // whitespace character
    if 0 == n || WHITESPACES.contains(&byte_read) {
      break;
    }

    // add the byte to the current bytes of the ascii number being parsed
    ascii_number_bytes.push(byte_read[0]);
  }

  // convert the bytes read for the number from their string representation into
  // the requested type, and return.
  String::from_utf8_lossy(&ascii_number_bytes).parse::<T>().unwrap_or_default()
}

/* #endregion */