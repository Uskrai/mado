pub mod app;
pub mod chapter_list;
pub mod download;
pub mod manga_info;
pub mod task_list;
pub mod task;
pub mod vec_chapters;

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

#[cfg(test)]
mod tests {
    pub fn run_loop() {
        let context = gtk::glib::MainContext::thread_default()
            .unwrap_or_else(gtk::glib::MainContext::default);

        while context.pending() {
            context.iteration(true);
        }
    }

    pub async fn try_recv<T>(sender: &relm4::Receiver<T>) -> Result<T, ()> {
        tokio::time::timeout(std::time::Duration::from_millis(1), sender.recv())
            .await
            .transpose()
            .and_then(|it| it.ok())
            .ok_or(())
    }
}
