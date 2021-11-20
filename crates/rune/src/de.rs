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

use rune::runtime::{FromValue, ToValue, Value};
use serde::Deserialize;

use crate::Error;

pub struct DeserializeValue<T>
where
  T: Send + Deserialize<'static>,
{
  value: Result<T, super::deserializer::Error>,
}

impl<T> DeserializeValue<T>
where
  T: Send + Deserialize<'static>,
{
  pub fn get(self) -> Result<T, super::deserializer::Error> {
    self.value
  }
}

impl<T> rune::runtime::FromValue for DeserializeValue<T>
where
  T: Send + Deserialize<'static> + 'static,
{
  fn from_value(value: Value) -> Result<Self, rune::runtime::VmError> {
    let deserializer = super::deserializer::Deserializer::new(value);
    Ok(Self {
      value: T::deserialize(deserializer),
    })
  }
}

pub struct DeserializeResult<T>
where
  T: Send + Deserialize<'static>,
{
  value: Result<T, Error>,
}

impl<T> DeserializeResult<T>
where
  T: Send + Deserialize<'static>,
{
  pub fn get(self) -> Result<T, Error> {
    self.value
  }
}

impl<T> FromValue for DeserializeResult<T>
where
  T: Send + Deserialize<'static> + 'static,
{
  fn from_value(value: Value) -> Result<Self, rune::runtime::VmError> {
    let value = value.into_result()?.borrow_ref()?.clone();

    let value = match value {
      Ok(v) => Self {
        value: DeserializeValue::from_value(v.to_value()?)?
          .get()
          .map_err(|e| e.into()),
      },
      Err(v) => {
        let error = Error::from_value(v)?;
        Self { value: Err(error) }
      }
    };

    Ok(value)
  }
}
