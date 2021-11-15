/*
 *  Copyright (c) 2021 Uskrai
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

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
