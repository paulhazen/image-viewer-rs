use std::{thread::current, f32::consts::PI, result};

use super::{ppm::PpmImage, color, V_CH, PIXEL_SIZE};
use rustfft::{*, num_complex::{Complex32, Complex}, algorithm::Dft};
use fft2d::*;

pub fn make_complex(image: PpmImage) -> Vec<Complex32> {
  let size = (image.width() * image.height()) as usize;
  let mut complex_image_data: Vec<Complex32> = Vec::with_capacity(size);

  for x in 0..image.width() {
    for y in 0..image.height() {
      let pixel = image.get_pixel_by_coord(x, y).unwrap();
      let pixel_intensity = color::rgb_to_hsv(pixel)[V_CH];

      complex_image_data.push(Complex32 { re: pixel_intensity, im: 0.0});
    }
  }

  complex_image_data
}
pub fn fast_fourier(input: PpmImage) -> PpmImage {
  let mut resultant_image = PpmImage::new (
    input.width(), input.height()
  );

  let mut complex = make_complex(input.clone());

  for x in 0..input.width() {
    fast_fourier_1d(&mut complex)
  }

  for i in 0..complex.len() {
    let y = i / input.width() as usize;
    let x = i - y * input.height() as usize;

    resultant_image.set_pixel_by_coord(
      x as u32, y as u32, &[(complex[i].re * u8::MAX as f32) as u8; PIXEL_SIZE]
    );
  }

  resultant_image
}

pub fn fast_fourier_1d(input: &mut Vec<Complex32>) {
  let size = input.len();
  let mut angle: f32 = 0.0;

  let mut even = Vec::<Complex32>::new();
  let mut odd = Vec::<Complex32>::new();

  for pixel_index in (0..size).step_by(2) {
    let mut temp_even = Vec::<Complex32>::new();
    let mut temp_odd = Vec::<Complex32>::new();

    temp_even.push(input[pixel_index]);
    temp_odd.push(input[pixel_index]);

    even.append(&mut temp_even);
    odd.append(&mut temp_odd);
  }

  fast_fourier_1d(&mut even);
  fast_fourier_1d(&mut odd);

  for pixel_index in 0..(size / 2) {
    angle = 2.0 * PI * pixel_index as f32 / size as f32;

    let real = angle.cos();
    let imaginary = angle.sin();

    let mut w = Complex {re: real, im: imaginary};

    w = w * odd[pixel_index];

    input[pixel_index] = even[pixel_index] + w;
    input[(size / 2) + pixel_index] = even[pixel_index] - w;
  }

}

pub fn dft_rows(image: PpmImage) -> PpmImage {
  let pixel_count = (image.height() * image.width()) as usize;

  let mut rows: Vec<Vec<Complex32>> = Vec::with_capacity(image.width() as usize);
  for y in 0..image.height() {
    let mut current_row: Vec<Complex32> = Vec::with_capacity(image.width() as usize);
    for x in 0..image.width() {
      let pixel = image.get_pixel_by_coord(x, y).unwrap();
      let pixel_intensity = color::rgb_to_hsv(pixel)[V_CH] as f32 / u16::MAX as f32;

      current_row.push(Complex32 { re: pixel_intensity, im: pixel_intensity });
    }
    rows.push(current_row);
  }

  for mut row in rows.iter_mut() {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(row.len());
    fft.process(&mut row);
  }

  let mut new_image = PpmImage::new(
    image.width(), image.height()
  );

  for y in 0..image.height() {
    for x in 0..image.width() {
      let intensity = &rows[y as usize][x as usize].im;
      let new_pixel = [(u8::MAX as f32 * intensity) as u8; PIXEL_SIZE];
      new_image.set_pixel_by_coord(x, y, &new_pixel);
    }
  }
  
  new_image
}