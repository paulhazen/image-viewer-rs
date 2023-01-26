use crate::core::ppm::PpmImage;
use crate::core::operations::OpType;

/* #region Filter Tests */

#[test]
fn test_sobel() {
  use crate::core::ppm::{Padding};
  use crate::core::io::open_image;
  use crate::core::filters::{SOBEL_H, SOBEL_V, apply_sobel};
  
  let base_dir = "tests\\sobel_filter";

  let h_check = open_image(
    format!("{}\\check_h.ppm", base_dir).as_str()
  ).unwrap();

  let v_check = open_image(
    format!("{}\\check_v.ppm", base_dir).as_str()
  ).unwrap();

  let input = open_image(
    format!("{}\\1.ppm", base_dir).as_str()
  ).unwrap();

  let h_filtered = apply_sobel(
    &input, SOBEL_H, Padding::Repeat
  );

  let v_filtered = apply_sobel(
    &input, SOBEL_V, Padding::Repeat
  );

  if h_filtered != h_check {
    assert!(is_reasonably_similar(
      &h_filtered, 
      &h_check, 
      2));
  }

  if v_filtered != v_check {
    assert!(is_reasonably_similar(
      &v_filtered, 
      &v_check, 
      2));
  }

}

/* #endregion */

/* #region Transform Tests */

#[test]
pub fn test_gamma_correction() {

  use crate::core::operations::gamma_transform;
  
  use crate::core::io::open_image;

  let gamma_values = [
    0.3, 2.8, 5.0
  ];

  let input = open_image(
    "tests\\gamma_correction\\1.ppm")
    .unwrap();

  for gamma_value in gamma_values {
    let gamma_correct_result = gamma_transform(
      &input, gamma_value, None
    );

    assert!(gamma_correct_result.is_ok());

    let check_path = format!(
      "tests\\gamma_correction\\check.{:.1}.ppm", gamma_value
    );

    let check = open_image(check_path.as_str()).unwrap();

    if *gamma_correct_result.as_ref().unwrap() != check {
      // a precision of 4 means to pass this test the images must be 
      // 99.99% similar
      assert!(is_reasonably_similar(
        gamma_correct_result.as_ref().unwrap(), 
        &check, 4)
      );
    } else {
      assert_eq!(gamma_correct_result.unwrap(), check);
    }
  }
}

/**
 * This test is a little more subjective. What I did was find an image that
 * had been log transformed already, then I performed my own log transform and
 * compared it to the expected result, measuring similarity between the images
 * by getting the average percentage that all the pixels were different
 * between the two images.
 */
#[test]
pub fn test_log_transform() {
  use crate::core::io::open_image;
  use crate::core::operations::log_transform;

  let base_dir = "tests\\log_transformation";

  let test_images = [1, 2];

  for image in test_images {
    let input = open_image(
      format!("{}\\{}.ppm", base_dir, image).as_str()
    ).unwrap();
  
    let log_transformed = log_transform(
      &input, None, None
    ).unwrap();
  
    let check = open_image(
      format!("{}\\check{}.ppm", base_dir, image).as_str()
    ).unwrap();
  
    if log_transformed != check {
      assert!(is_reasonably_similar(
        &log_transformed, &check, 4
      ));
    }
  }
}

/* #endregion */

/* #region Operation Tests */

/**
 * The following uses a helper function to test the expected outcomes of
 * product operations on images, then makes sure that they are correct
 */
#[test]
pub fn test_multiplication() {
  // tests for "normal" circumstances
  test_multiplication_helper(2, 2, 4);
  test_multiplication_helper(3, 4, 12);
  test_multiplication_helper(4, 3, 12);
  test_multiplication_helper(4, 4, 16);

  // test the zero / zero case
  test_multiplication_helper(0, 0, 0);

  // test the zero / 255 case
  test_multiplication_helper(0, 255, 0);
  
  // test the one / 255 case
  test_multiplication_helper(1, 255, 255);

  // test the overflow case
  test_multiplication_helper(30, 30, 255);
}

/// Takes an image with a "1" in white in the upper lefthand corner, and adds
/// it to an image that has a "3" in the lower lefthand corner, then compares
/// the sum with a check image
#[test]
pub fn test_add(){
  use crate::core::operations::OpType;

  test_op(
    "tests\\addition\\1.ppm",
    "tests\\addition\\2.ppm",
    OpType::Add,
    "tests\\addition\\check.ppm"
  );
}

#[test]
pub fn test_subtract() {
  use crate::core::operations::OpType;

  test_op(
    "tests\\subtraction\\1.ppm",
    "tests\\subtraction\\2.ppm",
    OpType::Subtract,
    "tests\\subtraction\\check.ppm",
  )
}

#[test]
pub fn test_negatation() {

  use crate::core::io::open_image;
  use crate::core::operations::negate;

  let input = open_image("tests\\negation\\1.ppm").unwrap();
  let negated_result = negate(&input);
  assert!(negated_result.is_ok());

  let check = open_image("tests\\negation\\check.ppm").unwrap();

  assert_eq!(negated_result.unwrap(), check);
}


/* #endregion */

/* #region Scaling Tests   */

#[test]
pub fn test_nearest_neighbor_scaling() {
 
  use crate::core::operations::{resize, ResizeAlgorithm};
  use crate::core::io::open_image;

  let base_dir = "tests\\resize\\nearest_neighbor";

  let input = open_image(
    format!("{}\\1.ppm", base_dir).as_str()
  ).unwrap();

  let sizes: [[u32;2];5] = [
    [128,64],  
    [128,128],
    [256,256],
    [64,128],
    [1024,1024],
  ];

  for size in sizes {
    let resized = resize(
      &input, size[0], size[1], 
      Some(ResizeAlgorithm::NearestNeighbor)
    ).unwrap();

    let check = open_image(
      format!(
        "{}\\1.{}x{}.ppm", base_dir, size[0], size[1]
      ).as_str()
    ).unwrap();

    assert_eq!(resized, check);
  }
}

#[test]
pub fn test_bilinear_iterpolation_scaling() {
  
  use crate::core::operations::{resize, ResizeAlgorithm};
  use crate::core::io::open_image;

  let base_dir = "tests\\resize\\bilinear_interpolation";

  let input = open_image(
    format!("{}\\1.ppm", base_dir).as_str()
  ).unwrap();

  let sizes: [[u32;2];5] = [
    [128,64],  
    [128,128],
    [256,256],
    [64,128],
    [1024,1024],
  ];

  for size in sizes {
    let resized = resize(
      &input, size[0], size[1], 
      Some(ResizeAlgorithm::BilinearInterpolation)
    ).unwrap();

    let check = open_image(
      format!(
        "{}\\1.{}x{}.ppm", base_dir, size[0], size[1]
      ).as_str()
    ).unwrap();

    if resized != check {
      // similarity of 3 means 99.9% similarity
      assert!(is_reasonably_similar(&resized, &check, 3));
    }
  }

}

/* #endregion */

/* #region Color Tests */

#[test]
fn test_hsv_to_rgb() {
  use crate::core::io::open_image;
  use crate::core::PIXEL_SIZE;
  use crate::core::ppm::PpmImage;
  use crate::core::color;
  use crate::core::{H_CH, S_CH, V_CH};

  let base_dir = "tests\\colorspace_conversion";

  let open_image_result = open_image(
    format!("{}\\1.ppm", base_dir).as_str()
  );

  assert!(open_image_result.is_ok());

  let image = open_image_result.ok().unwrap();
  
  let mut image_hsv_and_back = PpmImage::new(
    image.width(), image.height()
  );

  let mut abs_diff:usize = 0;

  for y in 0..image.height() {
    for x in 0..image.width() {

      if let Some(rgb_pixel) = image.get_pixel_by_coord(x, y) {
        let hsv_pixel = color::rgb_to_hsv(rgb_pixel);

        let back_to_rgb = color::hsv_to_rgb(
          hsv_pixel[H_CH], 
   hsv_pixel[S_CH], 
        hsv_pixel[V_CH]
        );

        image_hsv_and_back.set_pixel_by_coord(x, y, &back_to_rgb);
        for i in 0..PIXEL_SIZE {
          abs_diff += (rgb_pixel[i]).abs_diff(back_to_rgb[i]) as usize;
        }
      }
    }
  }

  if 0 != abs_diff {
    assert!(
      is_reasonably_similar(
        &image, 
        &image_hsv_and_back, 
        2
      )
    )
  } else {
    assert_eq!(0, abs_diff);
  }
}

/* #endregion */

/* #region IO Tests        */

#[test]
fn test_read_write() {
  use std::fs::{read_dir, remove_file};
  use crate::core::io::{open_image, write_image};

  let paths = read_dir(
    ".\\samples\\OfficialTestImages\\"
  ).unwrap();

  let mut failed_count = 0;

  const SAMPLE_FILE:&str = "samples\\TEMP.ppm";
  for path in paths {
    let mut image = open_image(
      path.unwrap().path().to_str().unwrap()
    ).unwrap();

    match write_image(&mut image, SAMPLE_FILE) {
      Err(why) => panic!("Could not write file: {}", why),
      Ok(_) => {},
    }

    let read_image = open_image(SAMPLE_FILE).unwrap();

    println!("Type of PPM Image: {}", image.ppm_type());

    if read_image != image {
      failed_count += 1
    } else {
      println!("Image successfully read, written, and read!");
    }

    remove_file(SAMPLE_FILE).expect("Could not delete sample file.");
  }

  assert_eq!(0, failed_count);
}

/* #endregion */

/* #region Helper Functions */

// similarity is a float between 0 and 1 indicating the average
// delta between all discrete RGB values (mapped 1:1 with pixel location)
// by multiplying that value by 10000, rounding up, then comparing with
// that number we multiplied (10000) we can see that the images are
// 99.999% identical
#[cfg(test)]
fn is_reasonably_similar(
  image_one:&PpmImage, 
  image_two:&PpmImage, 
  precision:u8
) -> bool {
  let precision:u32 = (10 as i32).pow(precision as u32) as u32;
  let similarity = measure_similarity(image_one, image_two);
  let precision_test = (similarity * precision as f64).round() as u32;
  
  if precision != precision_test {
    println!("Not reasonably similar: {}/{}", precision_test, precision);
    println!("Similarity measure: {}%", similarity * 100.);
  }

  println!("Similarity measure: {}%", similarity * 100.);
  return precision == precision_test
}

#[cfg(test)]
pub fn measure_similarity(image_one: &PpmImage, image_two: &PpmImage) -> f64 {
  use std::collections::BTreeSet;
  use crate::core::PIXEL_SIZE;

  if image_one.height() != image_two.height() ||
     image_one.width() != image_two.width() {
        return 0.0
      }

  let pixel_count: usize = (image_one.height() * image_one.height()) as usize;

  let mut total_difference:usize = 0;
  let mut running_average: f64 = 0.0;

  let mut distances = BTreeSet::<i32>::new();
  for i in 0..pixel_count {
    let rgb1 = image_one.get_bytes_at(i);
    let rgb2 = image_two.get_bytes_at(i);
    
    for ch in 0..PIXEL_SIZE {
      running_average += 255. - rgb1[ch].abs_diff(rgb2[ch]) as f64 / 255.;

      let temp_diff: i32 = rgb1[ch] as i32 - rgb2[ch] as i32;
      if 0 != temp_diff {
        distances.insert(temp_diff);
        total_difference += temp_diff.abs() as usize;
      }
    }
  }

  // some extra debug information if you want it. I should really turn
  // on debug log switches or something *shrug*
  //println!("Total difference between images is: {}", total_difference);
  //println!("These are the values by which each RGB value was off by:");
  //for d in distances {
  //  println!("{}", d);
  //}
  let similarity = running_average / 3.0 / pixel_count as f64 / 255.0;

  similarity
}

#[cfg(test)]
pub fn test_op(path1:&str, path2:&str, op_type:OpType, result:&str) {
  use crate::core::operations::{perform_operation};
  use crate::core::io::open_image;

  let lhs = open_image(path1).unwrap();
  let rhs = open_image(path2).unwrap();

  let sum_result = perform_operation(
    &lhs, &rhs, op_type
  );
  
  if let Ok(sum) = sum_result {
    let check_sum = open_image(result).unwrap();

    assert_eq!(sum, check_sum);
  } else if let Err(why) = sum_result {
    panic!("{}", why);
  }
}


#[cfg(test)]
pub fn test_multiplication_helper(rgb1: u8, rgb2: u8, check: u8) {
  use crate::core::operations::perform_operation;
  
  let mut lhs = PpmImage::create_color(
    rgb1, rgb1,rgb1, 10, 10
  );

  let mut rhs = PpmImage::create_color(
    rgb2, rgb2, rgb2, 10, 10
  );

  let check_image = PpmImage::create_color(
    check, check, check, 10, 10
  );

  let mult_result = perform_operation(
    &mut lhs, &mut rhs,OpType::Multiply
  );
  
  assert!(mult_result.is_ok());
  assert_eq!(mult_result.ok().unwrap(), check_image);
}

/* #endregion */