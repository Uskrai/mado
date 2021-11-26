use rune::{
  runtime::{AnyObj, Object, Shared, Value},
  Any,
};
use serde::Deserialize;
use serde_json::Value as JValue;

trait JsonWrapper {
  fn inner(&'static self) -> &'static serde_json::Value;
}

#[derive(Any)]
pub struct Json {
  inner: serde_json::Value,
}

#[derive(Any)]
pub struct JsonRef {
  inner: &'static serde_json::Value,
}

#[derive(Any)]
pub struct JsonRefMut {
  inner: &'static mut serde_json::Value,
}

impl Json {
  pub fn new(value: serde_json::Value) -> Self {
    Self { inner: value }
  }
}

impl JsonRef {
  pub fn new(value: &'static serde_json::Value) -> Self {
    Self { inner: value }
  }
}

impl JsonRefMut {
  pub fn new(value: &'static mut serde_json::Value) -> Self {
    Self { inner: value }
  }
}
impl JsonMethod for Json {
  fn inner(&'static self) -> &'static serde_json::Value {
    &self.inner
  }
}

impl JsonMethod for JsonRef {
  fn inner(&'static self) -> &'static serde_json::Value {
    self.inner
  }
}

impl JsonMethod for JsonRefMut {
  fn inner(&'static self) -> &'static serde_json::Value {
    self.inner
  }
}

impl JsonMethodMut for JsonRefMut {
  fn inner_mut(&'static mut self) -> &'static mut serde_json::Value {
    self.inner
  }
}

impl From<serde_json::Value> for Json {
  fn from(v: serde_json::Value) -> Self {
    Self { inner: v }
  }
}

trait JsonMethod {
  fn inner(&'static self) -> &'static serde_json::Value;

  fn to_string(&'static self) -> String {
    self.inner().to_string()
  }

  fn to_string_pretty(&'static self) -> String {
    serde_json::to_string_pretty(&self.inner()).unwrap()
  }

  fn select_as_value(
    &'static self,
    query: &str,
  ) -> Result<Vec<Value>, crate::Error> {
    let selected = jsonpath_lib::select(self.inner(), query)?;
    Ok(
      selected
        .into_iter()
        .map(|v| Value::deserialize(v).unwrap())
        .collect(),
    )
  }

  /// Return json as [Self]
  fn select_as_json(
    &'static self,
    query: &str,
  ) -> Result<Vec<JsonRef>, crate::Error> {
    let selected = jsonpath_lib::select(self.inner(), query)?;
    Ok(selected.into_iter().map(|v| JsonRef::new(v)).collect())
  }

  fn clone_to_value(&'static self) -> Result<Value, crate::Error> {
    serde_json::from_value(self.inner().clone()).map_err(|e| e.into())
  }

  fn get(&'static self, index: String) -> Option<Value> {
    let val = self.inner().get(index)?;

    Some(Self::convert_to_rune(val))
  }

  fn convert_trivial_to_rune(value: &serde_json::Value) -> Option<Value> {
    Some(match value {
      JValue::Null => Value::Option(Shared::new(None)),
      JValue::Bool(v) => Value::Bool(*v),
      JValue::Number(v) => {
        if v.is_i64() {
          Value::Integer(v.as_i64().unwrap())
        } else if v.is_f64() {
          Value::Float(v.as_f64().unwrap())
        } else {
          Value::String(Shared::new(v.as_u64().unwrap().to_string()))
        }
      }
      JValue::String(v) => Value::String(Shared::new(v.clone())),
      _ => return None,
    })
  }

  fn convert_to_rune(value: &'static serde_json::Value) -> Value {
    let val = Self::convert_trivial_to_rune(value);

    if let Some(v) = val {
      return v;
    }

    match value {
      JValue::Object(v) => {
        let mut object = Object::new();
        for (key, val) in v {
          object.insert(key.clone(), Self::convert_to_rune(val));
        }
        Value::Object(Shared::new(object))
        // Value::Any(Shared::new(AnyObj::new(JsonRef::new(value))))
      }
      JValue::Array(v) => {
        let val = v.iter().map(|v| Self::convert_to_rune(v));

        let mut vec = rune::runtime::Vec::new();
        for it in val {
          vec.push(it);
        }

        Value::Vec(Shared::new(vec))
      }

      _ => panic!("This should not happen"),
    }
  }

  fn pointer(&'static self, index: String) -> Option<Value> {
    let val = self.inner().pointer(&index)?;
    Some(Self::convert_to_rune(val))
  }

  fn clone(&'static self) -> Json {
    Json::new(self.inner().clone())
  }
}

trait JsonMethodMut: JsonMethod {
  fn inner_mut(&'static mut self) -> &'static mut serde_json::Value;

  fn as_ref(&'static mut self) -> JsonRef {
    JsonRef::new(self.inner_mut())
  }

  fn pointer_mut(&'static mut self, index: String) -> Option<Value> {
    let val = self.inner_mut().pointer_mut(&index)?;
    Some(Self::convert_to_rune_mut(val))
  }

  fn convert_to_rune_mut(value: &'static mut JValue) -> Value {
    let val: Option<Value> = Self::convert_trivial_to_rune(value);

    if let Some(val) = val {
      return val;
    }

    match value {
      JValue::Object(_) => {
        Value::Any(Shared::new(AnyObj::new(JsonRef::new(value))))
      }
      JValue::Array(v) => {
        let val = v.iter_mut();

        let mut vec = rune::runtime::Vec::new();
        for it in val {
          vec.push(Self::convert_to_rune_mut(it));
        }

        Value::Vec(Shared::new(vec))
      }
      _ => {
        panic!("This should not happen");
      }
    }
  }
}

pub fn load_module(
) -> Result<rune::compile::Module, rune::compile::ContextError> {
  let module = rune::compile::Module::with_crate_item("mado", &["json"]);

  mado_rune_macros::register_module! {
    (Json,JsonRef,JsonRefMut) => {
      inst => {
        to_string, to_string_pretty, clone_to_value, pointer,
        select_as_value, select_as_json, clone
      },
      protocol => {
        get: GET, get: INDEX_GET,
      }
    }
  }

  load_module_with(module)
}
