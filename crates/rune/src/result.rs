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

use runestick::{ContextError, Function, Module, Shared, Value, VmError};

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("std", &["result"]);
  module.inst_fn("or_else", or_else)?;
  module.inst_fn("or", or)?;

  Ok(module)
}

fn or_else(
  result: Result<Value, Value>,
  then: Function,
) -> Result<Value, VmError> {
  match result {
    Ok(_) => Ok(Value::Result(Shared::new(result))),
    Err(_) => then.call(()),
  }
}

fn or(
  result: Result<Value, Value>,
  then: Result<Value, Value>,
) -> Result<Value, VmError> {
  match result {
    Ok(_) => Ok(Value::Result(Shared::new(result))),
    Err(_) => Ok(Shared::new(then).into()),
  }
}
