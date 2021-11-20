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

use rune::{ContextError, Module};

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("std", &["vec"]);
  module.inst_fn("reverse", reverse)?;

  Ok(module)
}

pub fn reverse(vec: &mut rune::runtime::Vec) {
  vec.reverse();
}
