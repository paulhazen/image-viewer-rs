use crate::core::{stacking::{StackOperation, ClippingStrategy, ImageStack}, io::open_image, io::write_image, color, V_CH};
use crate::core::cr2::read_cr2;
use crate::core::io::read_raw;
use crate::core::fourier::dft_rows;
use crate::core::fourier::make_complex;
use crate::core::fourier::fast_fourier;
use std::{collections::HashMap, fs, io::Cursor, iter::Map};

use crate::core::{args::parse_arguments, stacking};
use byteorder::{LittleEndian, ReadBytesExt};
use eframe::{NativeOptions, run_native, epaint::Vec2};
use fft2d::slice::fft_2d;
use image::{Rgb, ImageBuffer, Primitive};
use palette::encoding::pixel;
use crate::gui::gui::ImageViewer;

use nalgebra::{
  allocator::Allocator, DMatrix, DefaultAllocator, Dim, MatrixSlice, Scalar, Vector2, Vector3,
};

mod core;
mod gui;

#[cfg(test)]
mod tests;

fn main() {
  
  create_master_images("astrophotography\\calibration\\test");
  //create_master_images("astrophotography\\calibration\\flats");
  //create_master_images("astrophotography\\calibration\\biases");
  //create_master_images("astrophotography\\calibration\\darks");
  //create_master_images("astrophotography\\lights");

  let mut _argument_success = false;

  match parse_arguments(None) {
    Err(why) => {
      println!("{}", why);
    }
    Ok(_) => { _argument_success = true}
  }

  let mut app = ImageViewer::new();
  
  //let test_image = open_image("samples\\man8.ppm").ok();

  //let complex = fast_fourier(test_image.unwrap());
  
  //app.set_image(Some(complex));

  let mut win_option = NativeOptions::default();
  //win_option.min_window_size = Some(Vec2::new(680., 480.));
  win_option.initial_window_size = Some(Vec2::new(1280.0, 720.0));
  run_native(
    "Image Viewer",
    win_option,
    Box::new(|_cc| Box::new(app))
  );
}


fn create_master_images(directory: &str) {

  let path_helper = move |stack_name: &str| -> String {
    format!("{}\\master.{}.tiff", directory, stack_name)
  };

  let mut stack = ImageStack::new();
  stack.add_algorithm(Box::new(
    stacking::Maximum::new()), 
    path_helper("maximum")
  );
  stack.add_algorithm(Box::new(
    stacking::Median::new()), 
    path_helper("median")
  );
  stack.add_algorithm(
    Box::new(stacking::Average::new()), 
    path_helper("average")
  );
  
  let ksc_params = [(10, 0.74), (10, 1.0), (10, 2.0), (10, 0.5), (5, 0.74)];
  
  for params in ksc_params {
    stack.add_algorithm(
      Box::new(stacking::KappaSigmaClipping::new(
        params.0, 
        params.1, 
        ClippingStrategy::Remove
      )),
      path_helper(
        format!("ksc_{}_{}_remove", params.0, params.1).as_str()
      )
    );
    stack.add_algorithm(
      Box::new(stacking::KappaSigmaClipping::new(
        params.0, 
        params.1, 
        ClippingStrategy::ReplaceWithMedian
      )),
      path_helper(
        format!("ksc_{}_{}_median", params.0, params.1).as_str()
      )
    );
  }

  println!("Creating calibration master for: {}", directory);
  if let Ok(calibration_type) = fs::read_dir(directory) {
    
    for image in calibration_type {

      let this_image = image.unwrap().path().clone();
      
      if this_image.file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
        .contains("master") {
        continue;
      }

      if let Some(extension) = this_image.extension() {
      
        let ext = extension.to_str().unwrap();
        if "TIFF" == ext {
          stack.add_image(this_image.to_str().unwrap());
        }
      }
    }

    stack.process_stack();
  }
}