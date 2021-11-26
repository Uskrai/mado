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
