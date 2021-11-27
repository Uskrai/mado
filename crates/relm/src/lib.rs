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

use tokio::task::JoinHandle;

/// Wrapper to [`tokio::task::JoinHandle`] that call
/// `abort` when dropped
#[derive(Debug)]
pub struct AbortOnDropHandle<R>(JoinHandle<R>);

impl<R> From<JoinHandle<R>> for AbortOnDropHandle<R> {
  fn from(v: JoinHandle<R>) -> Self {
    Self(v)
  }
}

impl<R> Drop for AbortOnDropHandle<R> {
  fn drop(&mut self) {
    self.0.abort()
  }
}
