use crate::core::ppm::{PpmImage};
use crate::to_1d;
use std::collections::{HashMap, BTreeSet};
use rand::Rng;

use super::PixelBytes;

/// Any pixels that are either unlabeled, or are the background color 
const UNLABELED:u64 = 0;

/* #region Data Structures */
#[derive(Copy, Clone)]
pub enum Connectivity {
  EIGHT,
  FOUR,
  NOS, // "Not Otherwise Specified" Experimental connectivity option
}

// uses the cll to create a new image that serves as a mask to illustrate things
// as an overlay on the loaded image
pub fn make_ccl_mask(
  image: &PpmImage, c_type: Connectivity, tolerance:f32
) -> PpmImage {

  let (pixel_labels, label_count) = ccl(
    image, c_type, tolerance
  );

  let mut new_image = PpmImage::new(image.width(), image.height());
  // keeping the histogram updated can be very demanding from a performance
  // perspective - so we turn it off here since a CCL mask doesn't need it.
  new_image.keep_histogram_updated = false;
  let mut label_colors = Vec::<PixelBytes<u8>>::with_capacity(
    label_count
  );

  let bg_color = image.get_background();

  label_colors.push(bg_color);

  for _ in 0..label_count {
    label_colors.push([
      rand::thread_rng().gen_range(u8::MIN..u8::MAX),
      rand::thread_rng().gen_range(u8::MIN..u8::MAX),
      rand::thread_rng().gen_range(u8::MIN..u8::MAX)
    ]);
  }


  for y in 0..image.height() {
    for x in 0..image.width() {
      let color_index = pixel_labels[
        to_1d!(x, y, image.width())
      ] as usize;
      new_image.set_pixel_by_coord(x, y, &label_colors[color_index]);
    }
  }

  new_image
}

// Creates a vector of labels, and a count of how many of them are unique
fn ccl(
  image: &PpmImage, c_type: Connectivity, tolerance:f32
) -> (Vec<u64>, usize) {

  // will store the labels that are linked together
  let mut linked_labels: HashMap<u64, BTreeSet<u64>> = HashMap::new();
  let mut labels = vec![
    UNLABELED; (image.width() * image.height()) as usize
  ];

  let mut cur_label = UNLABELED + 1;
  let bg_color = image.get_background();

  // first pass
  for y in 0..image.height() {
    for x in 0..image.width() {
      let pixel = image.get_pixel_by_coord(x, y).unwrap();
      // is the pixel a background color
      if pixel != bg_color {
        
        let possible_neighbors = get_valid_neighbors(
          x as i32, y as i32, image, c_type
        );

        let mut valid_neighbors: Vec<(u32, u32)> = Vec::new();
        let mut neighbor_labels = BTreeSet::<u64>::new();
        for possible_neighbor in possible_neighbors {
          let neighbor_pixel = image.get_pixel_by_coord(
            possible_neighbor.0, possible_neighbor.1
          ).unwrap();
          // does the pixel have the same value as the current one?
          if is_neighbor_equivalent(pixel, neighbor_pixel, tolerance) {
            let neighbor_label = labels[
              to_1d!(possible_neighbor.0, possible_neighbor.1, image.width())
            ];
            // does the neighbor have a label?
            if neighbor_label != UNLABELED {
              // add the neighbor label to the list of neighbor labels
              neighbor_labels.insert(neighbor_label);
              // the possible neighbor has become a valid neighbor
              valid_neighbors.push(possible_neighbor);
            }
          }
        }

        if valid_neighbors.is_empty() {
          cur_label += 1;
          linked_labels.insert(cur_label, BTreeSet::from([cur_label]));
          labels[to_1d!(x, y, image.width())] = cur_label;
        } else {
          labels[to_1d!(x, y, image.width())] = *neighbor_labels
            .iter()
            .next()
            .unwrap();

          for label in neighbor_labels.iter() {
            if let Some(linked) = linked_labels.get_mut(
              label
            ) {
              linked.extend(neighbor_labels.iter());
            }
          }
        }

      }
    }
  }

  // uncomment in order to work on fill algorithm to piggy-back on ccl
  // this will keep track of how many pixels per label
  //let mut label_pixel_count = BTreeMap::<u64, u64>::new();
  //let total_labeled_pixels: u64 = 0;

  // second pass
  for y in 0..image.height() {
    for x in 0..image.width() {
      // get the label that was originally set
      let current_label = labels[to_1d!(x, y, image.width())];
      if current_label != UNLABELED {
        // use label equivalency data structure to use smallest equivalent label
        let label = *linked_labels
          .get(&current_label)
          .unwrap()
          .iter()
          .next()
          .unwrap();

        labels[to_1d!(x, y, image.width())] = label;

        //if let Some(label_count) = label_pixel_count.get_mut(&label) {
        //  *label_count += 1;
        //} else {
        //  label_pixel_count.insert(label, 1);
        //}
      }
    }
  }

  (labels, cur_label as usize)
}

fn is_neighbor_equivalent(
  pixel:PixelBytes<u8>, 
  neighbor_pixel:PixelBytes<u8>, 
  tolerance:f32
) -> bool {
  if 1. <= tolerance {
    pixel == neighbor_pixel
  } else {
    use crate::core::color::redmean_distance;
    let redmean_dist = redmean_distance(
      pixel,
      neighbor_pixel
    );
    let is_close = redmean_dist <= 1. - tolerance;

    is_close
  }
}


/* #endregion */

/*
(-1, -1) (0, -1) (1, -1)
(-1,  0) (0,  0) (1,  0)
(-1,  1) (0,  1) (1,  1)
*/

  const      NORTH:(i8, i8) = ( 0, -1);
  const NORTH_EAST:(i8, i8) = ( 1, -1);
  const       EAST:(i8, i8) = ( 1,  0);
  const SOUTH_EAST:(i8, i8) = ( 1,  1);
  const      SOUTH:(i8, i8) = ( 0,  1);
  const SOUTH_WEST:(i8, i8) = (-1,  1);
  const       WEST:(i8, i8) = (-1,  0);
  const NORTH_WEST:(i8, i8) = (-1, -1);

fn get_valid_neighbors(
  x:i32, y:i32, image: &PpmImage, c_type: Connectivity
) -> Vec<(u32, u32)> {
  let mut shifts: Vec<(i8, i8)> = Vec::new();
  match c_type {
    Connectivity::EIGHT => {
      shifts.push(WEST); 
      shifts.push(NORTH_WEST);      
      shifts.push(NORTH); 
      shifts.push(NORTH_EAST);     
    },
    Connectivity::FOUR => {
      shifts.push(WEST);
      shifts.push(NORTH);
    }
    Connectivity::NOS => {
      shifts.push(NORTH);
      shifts.push(NORTH_EAST);
      shifts.push(EAST);
      shifts.push(SOUTH_EAST);    
      shifts.push(SOUTH);
      shifts.push(SOUTH_WEST);
      shifts.push(WEST);
      shifts.push(NORTH_WEST);
    }
  }

  let mut valid_neighbors: Vec<(u32, u32)> = Vec::new();

  for shift in shifts {
    let n_index = (x + shift.0 as i32, y + shift.1 as i32);
    if n_index.0 >= 0 && (n_index.0 as u32) < image.width() &&
       n_index.1 >= 1 && (n_index.1 as u32) < image.height() {
        valid_neighbors.push((n_index.0 as u32, n_index.1 as u32));
       }
  }

  valid_neighbors
}

