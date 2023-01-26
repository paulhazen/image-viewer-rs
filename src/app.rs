use crate::gui::gui::{ImageViewer, SelectedAction};

pub trait AppLogic {
  fn handle_selected_action(&mut self, action:SelectedAction);
}