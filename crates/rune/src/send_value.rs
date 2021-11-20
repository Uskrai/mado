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

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use rune::runtime::{
  AnyObj, FromValue, Object, Shared, Struct, SyncFunction, ToValue, Value,
  VmError,
};

use crate::{function::DebugSyncFunction, http::Client, regex::Regex};

#[derive(Clone, Debug)]
pub struct SendValue {
  value: SendValueKind,
}

impl From<SendValueKind> for SendValue {
  fn from(value: SendValueKind) -> Self {
    Self { value }
  }
}

#[derive(Clone)]
pub struct SendFunction {
  inner: Arc<SyncFunction>,
}

impl From<Arc<SyncFunction>> for SendFunction {
  fn from(v: Arc<SyncFunction>) -> Self {
    Self { inner: v }
  }
}

impl Debug for SendFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "SendFunction")
  }
}

/// this is a value that can be used to rune that require Send
#[derive(Clone, Debug)]
pub enum SendValueKind {
  Unit,
  Byte(u8),
  Bool(bool),
  Char(char),
  Integer(i64),
  Float(f64),
  String(String),
  Vec(Vec<SendValue>),
  Object(HashMap<String, SendValue>),
  Struct {
    from_rust: DebugSyncFunction,
    data: HashMap<String, SendValue>,
  },

  Option(Box<Option<SendValue>>),
  Regex(Arc<Regex>),
  HttpClient(Client),
  Function(DebugSyncFunction),
}

impl SendValue {
  pub fn kind(self) -> SendValueKind {
    self.value
  }

  pub fn kind_ref(&self) -> &SendValueKind {
    &self.value
  }

  pub fn into_object(self) -> Result<HashMap<String, SendValue>, crate::Error> {
    use SendValueKind as This;
    match self.value {
      This::Object(v) => Ok(v),
      This::Struct { data, .. } => Ok(data),
      _ => Err(crate::Error::expected(
        "Object".to_string(),
        self.value.to_string_variant().to_string(),
      )),
    }
  }

  pub fn into_vec(self) -> Result<Vec<SendValue>, crate::Error> {
    use SendValueKind as This;
    match self.value {
      This::Vec(v) => Ok(v),
      _ => Err(crate::Error::expected(
        "object".to_string(),
        self.value.to_string_variant().to_string(),
      )),
    }
  }

  pub fn into_string(self) -> Result<String, crate::Error> {
    match self.value {
      SendValueKind::String(v) => Ok(v),
      _ => Err(crate::Error::expected(
        "String".to_string(),
        self.value.to_string_variant().to_string(),
      )),
    }
  }

  pub fn into_function(self) -> Result<DebugSyncFunction, crate::Error> {
    match self.value {
      SendValueKind::Function(v) => Ok(v),
      _ => Err(crate::Error::expected(
        "Function".to_string(),
        self.value.to_string_variant().to_string(),
      )),
    }
  }
}

impl SendValueKind {
  /// Get Enum variant as String
  /// ```
  /// use mado_rune::SendValueKind;
  /// let value = SendValueKind::Integer(5);
  /// assert_eq!(value.to_string_variant(), "Integer");
  /// ```
  pub fn to_string_variant(&self) -> &'static str {
    macro_rules! match_value {
      ($name:ident (..)) => {
        Self::$name(..)
      };
      ($name:ident ()) => {
        Self::$name
      };
      ($name:ident {..}) => {
        Self::$name { .. }
      };
    }

    macro_rules! variants {
      ($($name:ident $p:tt),+) => {
        match self {
          $(match_value!($name $p) => {
            stringify!($name)
          }),+
        }
      };
    }

    variants!(
      Unit(),
      Bool(..),
      Byte(..),
      Char(..),
      Integer(..),
      Float(..),
      String(..),
      Vec(..),
      Object(..),
      Option(..),
      Struct { .. },
      Function(..),
      Regex(..),
      HttpClient(..)
    )
  }
}

impl FromValue for SendValueKind {
  fn from_value(
    value: rune::runtime::Value,
  ) -> Result<Self, rune::runtime::VmError> {
    let value = match value {
      Value::Unit => Self::Unit,
      Value::Integer(v) => Self::Integer(v),
      Value::Float(v) => Self::Float(v),
      Value::String(v) => Self::String(v.borrow_ref()?.clone()),
      Value::StaticString(v) => Self::String(v.to_string()),
      Value::Bool(v) => Self::Bool(v),
      Value::Byte(v) => Self::Byte(v),
      Value::Char(v) => Self::Char(v),
      Value::Vec(v) => {
        let v = v.borrow_ref()?;
        let mut res = Vec::new();
        for it in v.iter() {
          res.push(Self::from_value(it.clone())?.into());
        }
        Self::Vec(res)
      }
      Value::Object(v) => {
        Self::Object(Self::convert_object_to_map(v.take()?.iter())?)
      }
      Value::Struct(v) => Self::from_struct(v.take()?)?,
      Value::Any(v) => Self::from_any(v.take()?)?,
      Value::Function(v) => Self::Function(v.take()?.into_sync()?.into()),
      Value::Option(v) => {
        let v = v.take()?;
        let v = v.map(SendValue::from_value).transpose()?.into();

        Self::Option(v)
      }
      _ => {
        panic!("{:#?}", value);
      }
    };

    Ok(value)
  }
}

impl ToValue for SendValueKind {
  fn to_value(self) -> Result<Value, rune::runtime::VmError> {
    let value = match self {
      Self::Unit => Value::Unit,
      Self::Bool(v) => Value::Bool(v),
      Self::Byte(v) => Value::Byte(v),
      Self::Char(v) => Value::Char(v),
      Self::Integer(v) => Value::Integer(v),
      Self::Float(v) => Value::Float(v),
      Self::String(v) => Value::String(Shared::new(v)),
      Self::Vec(v) => {
        let mut vec = rune::runtime::Vec::new();
        for it in v {
          vec.push(it.to_value()?);
        }
        Value::Vec(Shared::new(vec))
      }
      Self::Object(v) => {
        Value::Object(Shared::new(Self::convert_map_to_object(v)?))
      }
      Self::Struct { from_rust, data } => from_rust.call((data,))?,
      Self::Option(v) => ToValue::to_value(*v)?,
      Self::Regex(v) => AnyObj::new(v.as_ref().clone()).to_value()?,
      Self::HttpClient(v) => AnyObj::new(v).to_value()?,
      Self::Function(_) => {
        panic!("Cannot convert function to value");
      }
    };

    Ok(value)
  }
}

impl FromValue for SendValue {
  fn from_value(value: Value) -> Result<Self, VmError> {
    Ok(Self {
      value: SendValueKind::from_value(value)?,
    })
  }
}

impl ToValue for SendValue {
  fn to_value(self) -> Result<Value, rune::runtime::VmError> {
    self.value.to_value()
  }
}

impl SendValueKind {
  fn from_struct(mut data: Struct) -> Result<Self, rune::runtime::VmError> {
    let from_rust = data
      .get("from_rust")
      .ok_or_else(|| {
        let f = "Struct should have from_rust variable to \
                 be used to convert back to rune";
        VmError::panic(format!("{} found: {:?}", f, data))
      })?
      .clone()
      .take()?
      .into_function()?
      .take()?
      .into_sync()?;

    let from_rust = from_rust.into();

    *data.get_mut("from_rust").unwrap() =
      ToValue::to_value(None::<String>).unwrap();

    let data = Self::convert_object_to_map(data.data().iter())?;

    Ok(Self::Struct { from_rust, data })
  }

  // convert value to HashMap<String, SendValue>
  fn convert_object_to_map<'a, T>(
    value: T,
  ) -> Result<HashMap<String, SendValue>, rune::runtime::VmError>
  where
    T: Iterator<Item = (&'a String, &'a Value)>,
  {
    let mut res = HashMap::new();
    // value.map(|(k,v)| (k.clone(), SendValue::from_value(
    for it in value {
      let (k, v) = it;
      let v = SendValueKind::from_value(v.clone())?;
      res.insert(k.clone(), v.into());
    }

    Ok(res)
  }

  // convert HashMap<String,SendValue to Object>
  pub fn convert_map_to_object(
    value: HashMap<String, SendValue>,
  ) -> Result<Object, VmError> {
    let mut obj = Object::new();
    for (k, v) in value {
      obj.insert(k, v.to_value()?);
    }
    Ok(obj)
  }

  fn from_any(value: AnyObj) -> Result<SendValueKind, VmError> {
    macro_rules! is {
      ($type:ty, $variant:ident) => {
        is!($type, $variant, |value| value)
      };
      ($type:ty, $variant:ident, $convert:expr) => {
        if value.is::<$type>() {
          let value = value.downcast_borrow_ref::<$type>().unwrap().clone();
          return Ok(Self::$variant($convert(value)));
        }
      };
    }
    is!(Regex, Regex, |value| { Arc::new(value) });
    is!(Client, HttpClient);

    return Err(VmError::panic(format!(
      "converting {:?} to send value is not supported, \
        consider adding them to SendValueKind if they implement \
        Send safely",
      value
    )));
  }
}

const _: () = {
  fn assert_send<T: Send>() {}
  fn assert_sync<T: Sync>() {}

  fn assert_all() {
    assert_send::<SendValue>();
    assert_sync::<SendValue>();
  }
};

#[cfg(test)]
mod tests {
  use rune::{runtime::Function, Vm};

  use super::*;

  #[test]
  fn call_method_fail() -> Result<(), VmError> {
    let sources = rune::sources! {
      entry => {
        pub fn main() {
          Foo {}
        }

        struct Foo {};
      }
    };

    let vm = crate::Build::default().build_vm(sources).unwrap();
    SendValue::from_value(vm.complete().unwrap())
      .expect_err("expecting error because from_rust not defined");

    Ok(())
  }

  #[test]
  fn call_method() -> Result<(), VmError> {
    let mut sources = rune::sources! {
      entry => {
        pub fn main() {
          (Foo::new(), call)
        }

        struct Foo { from_rust };
        impl Foo {
          pub fn new() {
            Foo {
              from_rust: Self::from_rust
            }
          }

          pub fn from_rust(data) {
            Foo {
              from_rust: data.from_rust
            }
          }

          pub fn test(self) {}
        }

        pub fn call(foo) {
          foo.test();
        }
      }
    };
    let unit = rune::prepare(&mut sources).build().unwrap();
    let vm = Vm::without_runtime(Arc::new(unit));

    let (foo, call): (SendValue, Function) =
      FromValue::from_value(vm.complete().unwrap()).unwrap();

    call.call::<_, ()>((foo,)).unwrap();

    Ok(())
  }
}
