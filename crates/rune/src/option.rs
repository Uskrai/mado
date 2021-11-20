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

use rune::{
  runtime::{Function, Shared, Value, VmError},
  ContextError, Module,
};

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("std", &["result"]);
  module.inst_fn("ok_or_else", ok_or_else)?;
  module.inst_fn("filter", filter)?;
  module.inst_fn("flatten", flatten)?;

  Ok(module)
}

fn ok_or_else(
  option: &Option<Value>,
  then: Function,
) -> Result<Value, VmError> {
  if let Some(v) = option {
    Ok(Value::Result(Shared::new(Ok(v.clone()))))
  } else {
    then.call(())
  }
}

fn filter(
  option: Option<Value>,
  predicate: Function,
) -> Result<Option<Value>, VmError> {
  Ok(match option {
    Some(v) => {
      if predicate.call((v.clone(),))? {
        Some(v)
      } else {
        None
      }
    }
    _ => None,
  })
}

fn flatten(option: Option<Value>) -> Result<Option<Value>, VmError> {
  Ok(match option {
    Some(v) => match v {
      Value::Option(v) => flatten(v.take()?)?,
      val => Some(val),
    },
    None => None,
  })
}
