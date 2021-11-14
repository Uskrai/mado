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

use runestick::{ContextError, Module};

use crate::DeserializeResult;
use mado_core::MangaInfo;

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("mado", &["test"]);

  macro_rules! register_type {
    ($name:ident) => {
      module.function(
        &[stringify!($name)],
        |v: DeserializeResult<$name>| match v.get() {
          Ok(_) => Ok(()),
          Err(v) => return Err(runestick::VmError::panic(v)),
        },
      )?;
    };
  }

  register_type!(MangaInfo);

  Ok(module)
}
//
