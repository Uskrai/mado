mod app;
mod download;
pub mod manga_info;

pub use app::*;

pub fn get_toplevel(mut widget: gtk::Widget) -> gtk::Window {
  use gtk::prelude::*;

  while let Some(parent) = widget.parent() {
    widget = parent;
  }

  widget.downcast::<gtk::Window>().unwrap()
}

// macros
mod dynamic_function;
mod gobject;

#[allow(unused_imports)]
pub(crate) use dynamic_function::create_dynamic_function;
