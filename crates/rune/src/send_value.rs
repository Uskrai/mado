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

use runestick::{
  AnyObj, FromValue, Object, Shared, SyncFunction, ToValue, Value, VmError,
};

use crate::{http::Client, regex::Regex};

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
  Struct(HashMap<String, SendValue>),

  Regex(Arc<Regex>),
  HttpClient(Client),
  Function(SendFunction),
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
      This::Object(v) | This::Struct(v) => Ok(v),
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

  pub fn into_function(self) -> Result<Arc<SyncFunction>, crate::Error> {
    match self.value {
      SendValueKind::Function(v) => Ok(v.inner),
      _ => Err(crate::Error::expected(
        "Function".to_string(),
        self.value.to_string_variant().to_string(),
      )),
    }
  }
}

impl SendValueKind {
  pub fn to_string_variant(&self) -> &'static str {
    macro_rules! match_value {
      ($name:ident (..)) => {
        Self::$name(..)
      };
      ($name:ident ()) => {
        Self::$name
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
      Struct(..),
      Function(..),
      Regex(..),
      HttpClient(..)
    )
  }
}

impl FromValue for SendValueKind {
  fn from_value(value: runestick::Value) -> Result<Self, runestick::VmError> {
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
        Self::Object(Self::to_send_value_map(v.take()?.iter())?)
      }
      Value::Struct(v) => {
        Self::Struct(Self::to_send_value_map(v.take()?.data().iter())?)
      }
      Value::Any(v) => Self::from_any(v.take()?)?,
      Value::Function(v) => {
        Self::Function(Arc::new(v.take()?.into_sync()?).into())
      }
      _ => {
        panic!("{:#?}", value);
      }
    };

    Ok(value)
  }
}

impl ToValue for SendValueKind {
  fn to_value(self) -> Result<Value, runestick::VmError> {
    let value = match self {
      Self::Unit => Value::Unit,
      Self::Bool(v) => Value::Bool(v),
      Self::Byte(v) => Value::Byte(v),
      Self::Char(v) => Value::Char(v),
      Self::Integer(v) => Value::Integer(v),
      Self::Float(v) => Value::Float(v),
      Self::String(v) => Value::String(Shared::new(v)),
      Self::Vec(v) => {
        let mut vec = runestick::Vec::new();
        for it in v {
          vec.push(it.to_value()?);
        }
        Value::Vec(Shared::new(vec))
      }
      Self::Struct(v) | Self::Object(v) => {
        Value::Object(Shared::new(Self::from_send_value_map(v)?))
      }

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
  fn to_value(self) -> Result<Value, runestick::VmError> {
    self.value.to_value()
  }
}

impl SendValueKind {
  fn to_send_value_map<'a, T>(
    value: T,
  ) -> Result<HashMap<String, SendValue>, runestick::VmError>
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

  pub fn from_send_value_map(
    value: HashMap<String, SendValue>,
  ) -> Result<Object, VmError> {
    let mut obj = Object::new();
    for (k, v) in value {
      obj.insert(k, v.to_value()?);
    }
    Ok(obj)
  }

  fn from_any(value: AnyObj) -> Result<SendValueKind, VmError> {
    if value.is::<Regex>() {
      let regex = value.downcast_borrow_ref::<Regex>().unwrap().clone();
      Ok(Self::Regex(Arc::new(regex)))
    } else if value.is::<Client>() {
      let client = value.downcast_borrow_ref::<Client>().unwrap().clone();
      Ok(Self::HttpClient(client))
    } else {
      Err(VmError::panic(format!(
        "converting {:?} to send value is not supported, \
        consider adding them to SendValueKind if they implement \
        Send safely",
        value
      )))
    }
  }
}
