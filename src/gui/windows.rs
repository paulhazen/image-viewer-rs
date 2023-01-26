use egui::plot::{Bar, BarChart};
use eframe::{egui::{self}, epaint::{Vec2, Color32}};
use crate::core::{ppm::{Padding, PpmImage}, 
operations::{Histogram, histogram_equalization}
};
use super::gui::{ImageViewer, BUTTON_PADDING, SPACING};

pub fn gamma_window(app: &mut ImageViewer, ctx:&egui::Context) {
  use crate::core::operations::gamma_transform;

  if app.show_gamma_controls {
    egui::Window::new("Gamma Transform Options")
      .collapsible(true)
      .resizable(false)
      .show(ctx, |ui| {
      ui.horizontal(|ui| {
        ui.add(egui::Slider::new(
          &mut app.gamma,
          0.1..=5.0).text("gamma")
        );
        if ui.button("Apply").clicked() {
          if let Some(image) = app.get_image().as_mut() {
            if let Ok(transform) = gamma_transform(
              image, 
              app.gamma, 
              None) {
              app.set_image(Some(transform));
            }
          }
        }
      });
    });
  } else {
    // reset gamma to default
    app.gamma = 1.;
  }
}

pub fn log_window(app: &mut ImageViewer, ctx:&egui::Context) {
  use crate::core::operations::log_transform;
  if app.show_log_controls {
    egui::Window::new("Log Transform Options")
      .collapsible(true)
      .resizable(false)
      .show(ctx, |ui| {
      ui.horizontal(|ui| {
        ui.vertical(|ui| {
          // slider for the c value
          ui.add(egui::Slider::new(
            &mut app.log_c, 
            0.0..=1.0).text("c")
          );
          // slider for the b value
          ui.add(egui::Slider::new(
            &mut app.log_b, 
            0.0..=10.).text("b")
          );
          if ui.button("Apply").clicked() {
            if let Some(new_image) = app.get_image().as_mut() {
              if let Ok(transform) = log_transform(
                new_image,
                None,
                Some(app.log_b)
              ) {
                app.set_image(Some(transform))
              }
            }
          }
        })
      });
    });
  }
}

/// Shows the Connected Component Label window
pub fn ccl_window(app: &mut ImageViewer, ctx:&egui::Context) {
  use crate::core::ccl;
  use crate::core::ccl::Connectivity;
  if app.show_ccl_controls {
    egui::Window::new("Connected Component Labeling")
      .collapsible(true)
      .resizable(false)
      .show(ctx, |ui| {
      ui.horizontal(|ui| {
        ui.vertical_centered_justified(|ui| {
          ui.spacing_mut().button_padding = Vec2::new(
            BUTTON_PADDING, 
            BUTTON_PADDING
          );

          ui.add(egui::Slider::new(
            &mut app.ccl_tolerance, 
            0.95..=1.0).text("tolerance")
          );
          
          if ui.button("8-Connected").clicked() {
            app.ccl_image_mask = Some(ccl::make_ccl_mask(
              app.get_image().as_mut().unwrap(), 
              Connectivity::EIGHT,
              app.ccl_tolerance)
            );
            app.redraw_image("ccl changed to 8-connected".to_string());
          }
          ui.add_space(SPACING);
          if ui.button("4-Connected").clicked() {
            app.ccl_image_mask = Some(ccl::make_ccl_mask(
              app.get_image().as_mut().unwrap(), 
              Connectivity::FOUR, app.ccl_tolerance)
            );
            app.redraw_image("ccl changed to 4-connected".to_string());
          }
          ui.add_space(SPACING);
          if ui.button("NOS Connected").clicked() {
            app.ccl_image_mask = Some(ccl::make_ccl_mask(
              app.get_image().as_mut().unwrap(), 
              Connectivity::NOS, app.ccl_tolerance)
            );
            app.redraw_image("ccl changed to NOS connected".to_string());
          }
          ui.add_space(SPACING);
          if ui.button("Clear").clicked() {
            app.ccl_image_mask = None;
            app.redraw_image("ccl was explicitly cleared".to_string());
          }
        })
      });
    });
  } else {
    app.ccl_image_mask = None;
  }
}

/* #region Window Data Structures */

/* #region UnsharpMaskWindow */
pub struct UnsharpMaskWindow {
  title: String,
  is_open: bool,
  pub error_msg: String,
  pub padding: Padding,
  pub sigma: f32,
  pub kernel_size: i32,
  pub scaling_factor: f32,
}

impl UnsharpMaskWindow {
  pub fn new(title: String) -> Self {
    UnsharpMaskWindow {
      scaling_factor: 1.,
      title: title,
      padding: Padding::Zero,
      error_msg: "".to_owned(),
      is_open: false,
      sigma: 1.,
      kernel_size: 3
    }
  }

  pub fn toggle(&mut self) {
    self.is_open = !self.is_open;
  }

  pub fn draw(&mut self, ctx:&egui::Context) -> bool {
    let mut clicked = false;
    if self.is_open {
      egui::Window::new(self.title.as_str())
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
          ui.vertical(|ui| {
            if !self.error_msg.is_empty() {
              ui.colored_label(Color32::DARK_RED, self.error_msg.as_str());
            }
            ui.vertical(|ui2| {
              ui2.add_space(SPACING);
              ui2.radio_value(
                &mut self.padding,
                Padding::Repeat,
                "Repeat"
              );
              ui2.add_space(SPACING);
              ui2.radio_value(
                &mut self.padding, 
                Padding::Zero, 
                "Zero"
              );
              ui2.add_space(SPACING);
            });
            ui.add(egui::Slider::new(
              &mut self.scaling_factor, 0.0..=20.
            ).text("scaling factor k"));
            
            ui.add(egui::Slider::new(
              &mut self.sigma, 0.0..=8.0
            ).text("sigma"));

            ui.add(egui::Slider::new(
              &mut self.kernel_size, 0..=25
            ).text("kernel size"));
            clicked = ui.button("Apply").clicked()
          });
        }); 
    }

    clicked
  }
}

/* #endregion */

/* #region GausianBlurWindow */
pub struct GaussianBlurWindow {
  title: String,
  is_open: bool,
  pub padding: Padding,
  pub error_msg: String,
  pub sigma: f32,
  pub kernel_size: i32,
}

impl GaussianBlurWindow {
  pub fn new(title: String) -> Self {
    GaussianBlurWindow {
      title: title,
      padding: Padding::Zero,
      error_msg: "".to_owned(),
      is_open: false,
      sigma: 1.,
      kernel_size: 3
    }
  }

  pub fn toggle(&mut self) {
    self.is_open = !self.is_open;
  }

  pub fn draw(&mut self, ctx:&egui::Context) -> bool {
    let mut clicked = false;
    if self.is_open {
      egui::Window::new(self.title.as_str())
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
          ui.vertical(|ui| {
            if !self.error_msg.is_empty() {
              ui.colored_label(Color32::DARK_RED, self.error_msg.as_str());
            }
            ui.vertical(|ui| {
              ui.add_space(SPACING);
              
              ui.radio_value(
                &mut self.padding,
                Padding::Repeat,
                 "Repeat"
              );
              
              ui.add_space(SPACING);
              
              ui.radio_value(
                &mut self.padding,
                Padding::Zero,
                "Zero"
              );
              
              ui.add_space(SPACING);
            });
            ui.add(egui::Slider::new(
              &mut self.sigma, 0.0..=8.0).text("sigma"
            ));
            ui.add(egui::Slider::new(
              &mut self.kernel_size, 0..=25).text("kernel size"
            ));
            clicked = ui.button("Apply").clicked()
          });
        }); 
    }

    clicked
  }
}

/* #endregion */

/* #region HistogramWindow */
pub struct HistogramWindow {
  title: String,
  pub histogram: Option<Histogram>,
  bars: Vec<Bar>,
  id: String,
  image_file: Option<String>,
  pub has_been_equalized: bool,
  pub should_equalize_current: bool,
  pub apply_to_current: bool,
  pub is_open: bool,
}

impl HistogramWindow {
  
  pub fn new(title: String, id: String) -> Self {
    HistogramWindow {
      title: title, 
      histogram: None,
      bars: Vec::<Bar>::new(),
      id: id,
      image_file: None,
      has_been_equalized: false,
      should_equalize_current: false,
      apply_to_current: false,
      is_open: false,
    }
  }

  pub fn set_filepath(&mut self, filepath:String) {
    self.has_been_equalized = false;
    self.image_file = Some(filepath);
  }

  pub fn toggle(&mut self) {
    self.is_open = !self.is_open;
  }

  pub fn update(&mut self, image:&PpmImage) {
    // given the order of operations, if this is true at the beginning of the
    // update loop, it has already been handled, and we can clear it.
    if self.apply_to_current {
      self.apply_to_current = false;
    }

    let hist = Histogram::from_image(image);

    self.bars.clear();

    for value in hist.intensities() {
      self.bars.push(Bar::new(
        *value as f64,
        *hist.data.get(value).unwrap() as f64
      ));
    }

    self.has_been_equalized = false;
    self.histogram = Some(hist);
  }

  pub fn draw(&mut self, ctx:&egui::Context) {
    use egui::plot::Plot;

    if self.is_open && None != self.histogram {

      let filepath = self.image_file.clone();
      let mut should_equalize = false;
      let mut should_apply = false;

      egui::Window::new(self.title.as_str())
        .collapsible(true)
        .resizable(true)
        .min_height(150.)
        .min_width(300.)
        .show(ctx, |ui| {
          ui.vertical(|ui| {
            
            let hist_plot = BarChart::new(self.bars.clone());
            
            let plotter = Plot::new(self.id.as_str())
              .allow_boxed_zoom(false)
              .allow_scroll(false)
              .allow_zoom(false)
              .allow_drag(false);

            plotter.view_aspect(2.0).show(
                ui,
                |plot_ui| 
                plot_ui.bar_chart(hist_plot)
              );

            if let Some(path) = filepath {
              ui.label(path);
            }

            ui.vertical_centered_justified(|ui| {
              ui.spacing_mut().button_padding = Vec2::new(
                BUTTON_PADDING, BUTTON_PADDING
              );
              
              if ui.add_enabled(
                !self.has_been_equalized, 
                egui::Button::new("Equalize Histogram")
              ).clicked() {
                should_equalize = true;
              }

              if ui.button("Apply to current image").clicked() {
                should_apply = true;
              }
            })
          });
      });

      if should_equalize {
        use crate::core::io::open_image;
        // open the image that this histogram is made for
        if let Some(image_path) = &self.image_file {
          if let Ok(image) = open_image(image_path.as_str()) {
            // if we actually have a histogram (not gauranteed)
            if let Some(hist) = self.histogram.clone() {
              // if we successfully equalized the histogram
              if let Ok(equalized) = histogram_equalization(
                &image, Some(hist)
              ) {
                self.has_been_equalized = true;
                // update the window with the newly equalized image
                self.update(&equalized);
              }
            }
          }
        } else {
          self.should_equalize_current = true;
          self.has_been_equalized = true;
        }
      }

      // should we apply the given histogram to the currently opened image?
      if should_apply {
        self.apply_to_current = true;
      }
    }
  }
}

/* #endregion */

/* #endregion */