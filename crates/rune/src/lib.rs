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

mod builder;
mod de;
mod deserializer;
mod error;
mod error_impl;
mod function;
pub mod http;
pub mod json;
mod module;
mod module_map;
mod regex;
mod rune;
mod selector;
mod send_value;
mod source_loader;
mod test;
pub mod testing;

// rune std stuff
mod option;
mod result;
mod vec;

pub use self::rune::Rune;
pub use error::{BuildError, Error, RuneError, VmError};
pub use json::Json;
pub use module::WebsiteModule;
pub use module_map::WebsiteModuleMap;
pub use send_value::{SendValue, SendValueKind};

pub use builder::{create_context, Build};
pub use source_loader::SourceLoader;

pub use de::{DeserializeResult, DeserializeValue};

pub fn load_modules(
  context: &mut ::rune::compile::Context,
) -> Result<(), ::rune::compile::ContextError> {
  context.install(&http::load_module()?)?;
  context.install(&json::load_module()?)?;
  context.install(&regex::load_module()?)?;
  context.install(&error::load_module()?)?;
  context.install(&selector::load_module()?)?;
  context.install(&test::load_module()?)?;

  // rune std stuff
  context.install(&result::load_module()?)?;
  context.install(&option::load_module()?)?;
  context.install(&vec::load_module()?)?;

  context.install(&rune_modules::test::module(true)?)?;
  context.install(&rune_modules::fmt::module(true)?)?;

  Ok(())
}
