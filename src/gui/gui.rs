
use eframe::{egui::{CentralPanel, TopBottomPanel, self, Modifiers, Response}};
use image::{ImageBuffer, Rgb};
use egui::Vec2;
use egui_extras::RetainedImage;
use strum::IntoEnumIterator;

use crate::core::{ppm::{PpmImage, Padding}, filters, 
args::parse_arguments
};
use crate::core::operations::{ResizeAlgorithm, OpType, OperationResult};
use crate::core::{io};
use crate::core::operations::{
  perform_operation, 
  resize, 
  histogram_equalization, 
  negate
};

use super::windows::{
  self, HistogramWindow, GaussianBlurWindow, UnsharpMaskWindow
};

pub const BUTTON_PADDING: f32 = 5.0;
pub const SPACING: f32 = 2.5;
const VIEWPORT_HMARGIN:f32 = 50.;
const VIEWPORT_WMARGIN:f32 = 50.;
const DEBUG_FILE_NAME:&str = "0.png";

pub struct ImageViewer {
  // option is used because the image viewer may or may not actually have an
  // image open
  drawn_image: Option<egui_extras::RetainedImage>,
  image_hidden: Option<crate::core::ppm::PpmImage>,
  pub ccl_image_mask: Option<crate::core::ppm::PpmImage>,
  
  histogram_window: HistogramWindow,
  image_histogram_window: HistogramWindow,
  gaussian_blur_window: GaussianBlurWindow,
  unsharp_mask_window: UnsharpMaskWindow,

  previous_images: Vec<PpmImage>,
  viewport_height: f32,
  viewport_width: f32,

  fit_to_window: bool,
  maintain_aspect_ratio: bool,
  
  command: String,
  command_resp: String,
  resize_algorithm: ResizeAlgorithm,
  pub padding_strategy: Padding,

  pub show_ccl_controls: bool,
  pub ccl_tolerance: f32,

  /* Gamma window stuff */
  pub show_gamma_controls: bool,
  pub gamma: f32,
  
  /* log window stuff */
  pub show_log_controls: bool,
  pub log_c: f32,
  pub log_b: f32,

  pub show_histogram_window: bool,

  quit: bool,
}

impl ImageViewer {
  pub fn new() -> Self {
    ImageViewer {
      image_histogram_window: HistogramWindow::new(
        "Histogram from Selected Image".to_string(),
        "image_histogram".to_string(),
      ),

      histogram_window: HistogramWindow::new(
        "Current Histogram".to_string(),
        "histogram_current".to_string(),
      ),

      gaussian_blur_window: GaussianBlurWindow::new(
        "Gaussian Blur".to_string()
      ),

      unsharp_mask_window: UnsharpMaskWindow::new(
        "Unsharp Masking".to_string()
      ),

      drawn_image: None,
      image_hidden: None,
      previous_images: Vec::<PpmImage>::new(),
      ccl_image_mask: None,
      viewport_height: 0.,
      viewport_width: 0.,

      fit_to_window: true,
      maintain_aspect_ratio: true,
      
      command: "".to_owned(),
      command_resp: "".to_owned(),
      resize_algorithm: ResizeAlgorithm::NearestNeighbor,
      padding_strategy: Padding::Zero,

      // determines whether the gamma controls should be shown
      show_gamma_controls: false,
      show_log_controls: false,
      show_ccl_controls: false,
      ccl_tolerance: 0.9,
      show_histogram_window: false,

      gamma: 1.,

      log_c: 0.,
      log_b: 10.,

      quit: false,
    }
  }

  pub fn undo(&mut self) {
    // if there are previous images
    if !self.previous_images.is_empty() {

      // set the current image to the last image popped off the previous_images
      // list
      if let Some(last_image) = self.previous_images.pop() {
        // update the histogram window
        self.histogram_window.update(&last_image);

        // explicitly set the underlying image to the last image. Note here that
        // set_image is not used here - because that would mess up the undo list
        self.image_hidden = Some(last_image);

        // request redraw
        self.redraw_image("Undo action taken".to_string());
      }
    }
  }

  pub fn get_image(&self) -> Option<&PpmImage> {
    return self.image_hidden.as_ref()
  }

  pub fn set_image(&mut self, image:Option<PpmImage>) {
    // if the new image being set exists
    if let Some(new_image) = image {
      // if the current image exists
      if None != self.image_hidden {
        // push a copy of the current image onto the stack of "previous" images
        let current_image = self.image_hidden.clone();
        self.previous_images.push(current_image.unwrap());
      }
      
      // update the histogram window
      self.histogram_window.update(&new_image);

      // set the new image
      self.image_hidden = Some(new_image);

      // clear ccl in case it is open
      self.ccl_image_mask = None;

      // redraw the image
      self.redraw_image("set_image was called".to_string());
    }
  }

  /* #region Helper functions */

  fn image_ops_helper(&mut self, op_type: OpType) {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
      let lh_image = self.image_hidden.clone().unwrap();
      
      // TODO: Deal with bad open image / image open failure stuff
      let rh_image = io::open_image(
        path.to_str().unwrap()
      ).unwrap();
    
      let operation_result = perform_operation(
        &lh_image,
        &rh_image, 
        op_type);

      match operation_result {
        Ok(image) => {
          self.image_hidden = Some(image);
          self.redraw_image(format!(
            "Image operation \"{}\" completed successfully.", op_type
          ));
        },
        Err(why) => {
          println!("{}", why);
        }
      }
    }
  }

  fn fit_to_screen(&mut self, image:&mut Option<PpmImage>) -> OperationResult {

    use crate::core::min;

    if let Some(image) = image.as_mut() {

      if self.maintain_aspect_ratio {
        let w_ratio = self.viewport_width / image.width() as f32;
        let h_ratio = self.viewport_height / image.height() as f32;

        let ratio = min(w_ratio, h_ratio);

        let new_width = image.width() as f32 * ratio;
        let new_height = image.height() as f32 * ratio;

        return resize(
          image, 
          new_width as u32, 
          new_height as u32, 
          Some(self.resize_algorithm))
      } else {
        return resize(
          image, 
          self.viewport_width as u32,
          self.viewport_height as u32,
          Some(self.resize_algorithm)
        )
      }

      
    } else {
      return Err("No image to resize".to_string())
    }
  }

  /* #endregion */

  pub fn redraw_image(&mut self, reason:String) {

    println!("Redrawing because: '{}'", reason);

    let mut image_copy = if None != self.ccl_image_mask { 
      self.ccl_image_mask.clone() 
    } else { 
      self.image_hidden.clone() 
    };
    
    if None != image_copy {
      if self.fit_to_window {
        if let Ok(resized) = self.fit_to_screen(&mut image_copy) {
          image_copy = Some(resized);
        }
      }

      let image = image_copy.unwrap();
 
      let mut buf: ImageBuffer<Rgb<u8>, Vec<u8>> = image::ImageBuffer::new(
        image.width(),
        image.height()
      );

      for (x, y, pixels) in buf.enumerate_pixels_mut() {
        if let Some(pixel) = image.get_pixel_by_coord(x, y) {
          *pixels = image::Rgb(pixel);
        }
      }
    
      let color_image = egui::ColorImage::from_rgb(
        [image.width() as usize, image.height() as usize],
        &buf.as_ref(),
        );
    
      let render_result = RetainedImage::from_color_image(
        DEBUG_FILE_NAME, 
        color_image
      );
      
      self.drawn_image = Some(render_result);
    }
  }

  /* #region Control Windows */

  /* #endregion */

  fn create_file_menu(&mut self, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
      ui.spacing_mut().button_padding = Vec2::new(
        BUTTON_PADDING, 
        BUTTON_PADDING
      );
      if ui.button("Open").clicked() {
        ui.close_menu();
        if let Some(path) = rfd::FileDialog::new().pick_file() {
          let open_image_result = io::open_image(
            path.to_str().unwrap()
          );

          match open_image_result {
            Ok(image) => {
              self.set_image(Some(image));
            },
            Err(why) => {
              println!("{}", why);
            }
          }
        }
      }

      // should the save as be enabled?
      let save_as_enabled = None != self.get_image();
      if ui.add_enabled(
        save_as_enabled, egui::Button::new("Save as")
      ).clicked() {
        if let Some(path) = rfd::FileDialog::new().add_filter(
          "Portable Pixel Map",
          &["ppm", "PPM"]).save_file() {      
            // TODO: Do a better job error handling when you can't write file
            // note that we can safely use unwrap here with get_image, because
            // the button is only enabled if get_image() is not none
            match io::write_image(
              self.get_image().unwrap(), 
              path.to_str().unwrap()
            ) {
              Err(why) => {
                println!("Not able to save file: {}", why)
              },
              Ok(_) => {}
            }
        }
      }

      if ui.button("Quit").clicked() { 
        self.quit = true;
      }
    });
  }

  fn create_edit_menu(&mut self, ui: &mut egui::Ui) {
    let edit_enabled = None != self.get_image();
    ui.menu_button("Edit", |ui| {
      ui.spacing_mut().button_padding = Vec2::new(
        BUTTON_PADDING, 
        BUTTON_PADDING
      );

      // undo is only enabled if the previous images vector is not empty, and
      // if edit itself is actually enabled.
      let undo_enabled = !self.previous_images.is_empty() && edit_enabled;

      if ui.add_enabled(
        undo_enabled, egui::Button::new("Undo")
      ).clicked() {
        ui.close_menu();
        self.undo();
      }

      if ui.add_enabled(
        edit_enabled, egui::Button::new("Negate")
      ).clicked() {
        ui.close_menu();
        // we can safely use unwrap here because this button is only enabled
        // if get_image is not none
        if let Ok(negated_image) = negate(
          self.get_image().unwrap()
        ) {
          self.set_image(Some(negated_image));
        }
      }

      ui.menu_button("Image", |ui| {
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING, 
          BUTTON_PADDING
        );

        // for each OpType, create a button and add it to the menu, appending
        // "image" to the value of the enum as a string
        for op_type in OpType::iter() {
          if ui.add_enabled(edit_enabled, egui::Button::new(
            format!("{} image", op_type.to_string()))
          ).clicked() {
            ui.close_menu();
            self.image_ops_helper(op_type);
          }
        }
      });

      ui.menu_button("Filters", |ui| {
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING, 
          BUTTON_PADDING
        );

        if ui.add_enabled(
          edit_enabled, 
          egui::Button::new("Gaussian smoothing")
        ).clicked() {
          ui.close_menu();
          self.gaussian_blur_window.toggle();
        }

        if ui.add_enabled(
          edit_enabled, 
          egui::Button::new("Unsharp masking")
        ).clicked() {
          ui.close_menu();
          self.unsharp_mask_window.toggle();
        }

        if ui.add_enabled(edit_enabled, 
          egui::Button::new("Edge detection")
        ).clicked() { 
          ui.close_menu();
          // note that we can use unwrap with confidence because the button
          // is disabled if image is None
          if let Ok(edge_detected) = filters::edge_detect(
            self.get_image().unwrap()
          ) {
            self.set_image(Some(edge_detected));
          }
        }
      });

      ui.menu_button("Transforms", |ui| {
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING,
          BUTTON_PADDING
        );

        if ui.add_enabled(
          edit_enabled, 
          egui::Button::new("Gamma Transformation")
        ).clicked() {
          ui.close_menu();
          self.show_gamma_controls = !self.show_gamma_controls;
        }

        if ui.add_enabled(
          edit_enabled, 
          egui::Button::new("Log Transformation")
        ).clicked() {
          ui.close_menu();
          self.show_log_controls = !self.show_log_controls;
        }
      });

      ui.menu_button("Hist. Equalization", |ui| {
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING, 
          BUTTON_PADDING
        );

        if ui.add_enabled(
          edit_enabled,
          egui::Button::new("Equalize to current")
        ).clicked() {
          ui.close_menu();
          if let Ok(equalized_image) = histogram_equalization(
            self.get_image().unwrap(), None
          ) {
            self.set_image(Some(equalized_image));
          }
        }

        if ui.add_enabled(
          edit_enabled, 
          egui::Button::new("Histogram from image")
        ).clicked() {
          ui.close_menu();
          if let Some(path) = rfd::FileDialog::new().pick_file() {
            // TODO: Deal with scenario where image isn't valid or otherwise
            // cannot be opened
            use io::open_image;

            match open_image(path.to_str().unwrap()) {
              Ok(image) => {
                // image has been opened
                self.image_histogram_window.set_filepath(
                  path.to_string_lossy().to_string()
                );
                self.image_histogram_window.update(&image);
                self.image_histogram_window.is_open = true;
              },
              Err(_why) => {
                // image could not be opened, display dialog
              }
            }
          }
        }
      });
    });
  }

  fn create_view_menu(&mut self, ui: &mut egui::Ui) {
    ui.menu_button("View", |ui| {
      ui.spacing_mut().button_padding = Vec2::new(
        BUTTON_PADDING, 
        BUTTON_PADDING
      );
      ui.add_space(SPACING);
      if ui.checkbox(
        &mut self.fit_to_window, 
        "Fit image to screen"
      ).changed() {
        self.redraw_image(
          "fit image to screen setting changed".to_string()
        );
      }
      if ui.checkbox(
        &mut self.maintain_aspect_ratio,
        "Maintain aspect ratio"
      ).changed() {
        self.redraw_image(
          "aspect ratio setting changed.".to_string()
        );
      }
      ui.add_space(SPACING);
      ui.add_enabled(
        None != self.get_image(), 
        egui::Checkbox::new(
        &mut self.histogram_window.is_open, "Show histogram"
        )
      );
      ui.add_space(SPACING);
    });
  }

  fn create_options_menu(&mut self, ui: &mut egui::Ui) {
    ui.menu_button("Options", |ui| {
      ui.spacing_mut().button_padding = Vec2::new(
        BUTTON_PADDING, 
        BUTTON_PADDING
      );

      ui.menu_button("Resizing Algorithm", |ui|{
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING, 
          BUTTON_PADDING
        );

        ui.add_space(SPACING);

        if ui.radio_value(
          &mut self.resize_algorithm, 
          ResizeAlgorithm::BilinearInterpolation, 
          "Bilinear"
        ).changed() {
          self.redraw_image("resize algorithm changed".to_string());
        };

        ui.add_space(SPACING);

        if ui.radio_value(
          &mut self.resize_algorithm, 
          ResizeAlgorithm::NearestNeighbor, 
          "Nearest Neighbor"
        ).changed() {
          self.redraw_image("resize algorithm changed".to_string());
        }

        ui.add_space(SPACING);
      });
      ui.menu_button("Padding Strategy", |ui|{
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING, 
          BUTTON_PADDING
        );
        ui.add_space(SPACING);
        ui.radio_value(
          &mut self.padding_strategy, 
          Padding::Repeat, "Repeat"
        );
        ui.add_space(SPACING);
        ui.radio_value(
          &mut self.padding_strategy, 
          Padding::Zero, "Zero"
        );
        ui.add_space(SPACING);
      });
      
    });
  }

  fn create_menu_bar(&mut self, ctx: &egui::Context) {

    // define TopBottomPanel widget
    let top_panel = TopBottomPanel::top("top_panel");
    
    top_panel.show(ctx, |ui| {
      egui::menu::bar(ui, |ui| {
        ui.spacing_mut().button_padding = Vec2::new(
          BUTTON_PADDING, 
          BUTTON_PADDING
        );
        self.create_file_menu(ui);
        self.create_edit_menu(ui);
        self.create_view_menu(ui);
        self.create_options_menu(ui);
        
        let ccl_enabled = None != self.get_image();

        if ui.add_enabled(
          ccl_enabled, 
          egui::Button::new("CCL")
        ).clicked() {
          ui.close_menu();
          self.show_ccl_controls = !self.show_ccl_controls;
          if !self.show_ccl_controls {
            self.ccl_image_mask = None;
            self.redraw_image(
              "ccl turned off, clearing mask".to_string()
            );
          }
        }
        
        ui.horizontal(|ui| {
          ui.label(
            format!(
              "Viewport Size: {} x {}", 
              self.viewport_width, self.viewport_height
            )
          );
          if let Some(current_image) = &self.drawn_image {
            ui.label(format!("Drawn Image Dimensions: ({} by {})", 
              current_image.width(), 
              current_image.height()
            ));
          }
          ui.add_space(SPACING);
          if let Some(pos) = ctx.pointer_latest_pos() {
            ui.label(
              format!(
                "Mouse Position: ({}, {})", 
                pos.x as usize, pos.y as usize
              )
            );
          }
        });
      });
    });
  }

  // responsible for rendering the command window
  fn create_command_box(&mut self, ctx: &egui::Context) {

    // the command box at the bottom of the screen
    let command_box = egui::TextEdit::singleline(
      &mut self.command
    ).code_editor()
    .hint_text("enter commands here")
    .desired_width(f32::INFINITY);

    let mut command_box_response: Option<Response> = None;

    TopBottomPanel::bottom("bottom_panel").show(
      ctx,
      |ui| {
      ui.with_layout(
        egui::Layout::bottom_up(egui::Align::BOTTOM),
        |ui| {
        ui.with_layout(
          egui::Layout::left_to_right(egui::Align::Center), 
          |ui| {
          ui.vertical(|ui| {
            ui.label(&self.command_resp);
            ui.add_space(5.);
            ui.horizontal(|ui| {
              command_box_response = Some(ui.add_sized(
                Vec2::new(ui.available_width(), 20.0), 
                command_box)
              );
            });
            ui.add_space(5.);
          });
        });  
      });
    });

    // handle call-back for the command box
    if command_box_response.unwrap().lost_focus() && 
       ctx.input().key_pressed(egui::Key::Enter) {
      
      // split the contents of the command text box into a vector of &str
      let args: Vec<&str> = self.command.split_whitespace().collect();

      // convert args to vector of Strings
      let mut string_args = Vec::<String>::with_capacity(
        args.len() + 1
      );

      // add "argslist" as a ghost item - it will be ignored but is important
      // in order to maintain the proper order of arguments
      string_args.push("arglist".to_string());
      for s in args.iter() {
        string_args.push(s.to_string());
      }

      // pass the arguments to the argument parser
      match parse_arguments(Some(string_args)) {
        Err(why) => self.command_resp = format!("Error: {why}"),
        Ok(image_output) => {
          if let Some(image) = image_output {
            self.command = "".to_string();
            self.set_image(Some(image));
          }
        }
      }
    }
  }
}

impl eframe::App for ImageViewer {
  
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

    ctx.request_repaint();

    // perform "undo" when control-z is pressed
    if ctx.input().key_pressed(egui::Key::Z) && 
       Modifiers::CTRL.matches(Modifiers::CTRL) {
      self.undo();
    }

    let scroll_x = ctx.input().scroll_delta.x;
    let scroll_y = ctx.input().scroll_delta.y;

    if scroll_x != 0. || scroll_y != 0. {
      println!("Scrolling: ({:3}, {:3})", scroll_x, scroll_y);
    }

    if self.quit {
      frame.close();
    }

    // handle loading of the various windows.
    windows::ccl_window(self, ctx);
    windows::gamma_window(self, ctx);
    windows::log_window(self, ctx);
    
    self.create_menu_bar(ctx);
    
    self.create_command_box(ctx);

    CentralPanel::default().show(ctx, |ui| {
      
      self.histogram_window.draw(ctx);
      if self.histogram_window.should_equalize_current {
        if let Ok(equalized_image) = histogram_equalization(
          self.get_image().unwrap(), None
        ) {
          self.histogram_window.update(&equalized_image);
        }
        self.histogram_window.should_equalize_current = false;
        self.histogram_window.has_been_equalized = true;
      }

      if self.histogram_window.apply_to_current {
        if let Ok(equalized_image) = histogram_equalization(
          self.get_image().unwrap(), None
        ) {
          self.histogram_window.apply_to_current = false;
          self.set_image(Some(equalized_image));
        }
      }

      self.image_histogram_window.draw(ctx);

      if self.image_histogram_window.apply_to_current {
        if let Ok(equalized_image) = histogram_equalization(
          self.get_image().unwrap(), 
          self.image_histogram_window.histogram.clone()
        ) {
          self.image_histogram_window.apply_to_current = false;
          self.set_image(Some(equalized_image));
        }
      }
      /* #region Handle Gaussian Blur Window */
      if self.gaussian_blur_window.draw(ctx) {
        if let Some(image) = self.get_image() {
          let result = filters::gaussian_blur(
            image,
            self.gaussian_blur_window.sigma,
            self.gaussian_blur_window.kernel_size,
            self.padding_strategy);
          
          match result {
            Ok(image) => {
              self.gaussian_blur_window.error_msg = "".to_owned();
              self.set_image(Some(image))
            },
            Err(why) => {
              self.gaussian_blur_window.error_msg = why;
            }
          }
        }
      }
      /* #endregion */

      if self.unsharp_mask_window.draw(ctx) {
        if let Some(image) = self.get_image() {
          let result = filters::unsharp_mask(
            image,
            self.unsharp_mask_window.sigma,
            self.unsharp_mask_window.kernel_size,
            self.unsharp_mask_window.scaling_factor,
            self.padding_strategy);
          
          match result {
            Ok(image) => {
              self.unsharp_mask_window.error_msg = "".to_owned();
              self.set_image(Some(image))
            },
            Err(why) => {
              self.unsharp_mask_window.error_msg = why;
            }
          }
        }
      }

      /* #region Resizing logic */

      let mut resized = false;
      
      let new_viewport_height = ui.available_height() - VIEWPORT_HMARGIN;
      let new_viewport_width = ui.available_width() - VIEWPORT_WMARGIN;

      if new_viewport_height != self.viewport_height || 
         new_viewport_width != self.viewport_width {
        resized = true;

        self.viewport_height = new_viewport_height;
        self.viewport_width = new_viewport_width;
      }

      if resized {
        self.redraw_image(
          "viewport available size has changed".to_string()
        );
      }

      /* #endregion */

      ui.centered_and_justified(|ui| {
        if let Some(buf) = &self.drawn_image {
          buf.show(ui);
        }
      });
    });
  }
}

