use rune::runtime::Value;
use serde::{
  de::{
    value::{MapDeserializer, SeqDeserializer},
    EnumAccess, Error as DeserializeError, IntoDeserializer, VariantAccess,
    Visitor,
  },
  Deserializer as _,
};
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
  #[error("{0}")]
  Custom(String),

  #[error("{0}")]
  AccessError(#[from] rune::runtime::AccessError),
  #[error("{0}")]
  DeserializeError(#[from] serde::de::value::Error),
}

struct IntoDeser {
  value: Value,
}

impl IntoDeser {
  pub fn new(value: Value) -> Self {
    Self { value }
  }
}

impl<'de> IntoDeserializer<'de, Error> for IntoDeser {
  type Deserializer = Deserializer;
  fn into_deserializer(self) -> Self::Deserializer {
    Deserializer { value: self.value }
  }
}

impl serde::de::Error for Error {
  fn custom<T>(msg: T) -> Self
  where
    T: std::fmt::Display,
  {
    Self::Custom(msg.to_string())
  }
}

pub struct Deserializer {
  value: Value,
}

impl Deserializer {
  pub fn new(value: Value) -> Self {
    Self { value }
  }
}

macro_rules! deserialize_with {
  (
    $self:ident, $visitor:ident,
    $($pat:pat => $exp:expr),+
  ) => {
    match &$self.value {
      Value::Option(v) => {
        let opt = v.borrow_ref()?.clone();
        match opt {
          Some(v) => match &v {
            $($pat => $exp,)+
            _ => $self.deserialize_any($visitor),
          },
          None => $visitor.visit_none(),
        }
      }
      $($pat => $exp,)+
      _ => $self.deserialize_any($visitor),
    }
  };
}

impl<'de: 'a, 'a> serde::Deserializer<'de> for Deserializer {
  type Error = Error;

  fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    macro_rules! cannot {
      ($name:literal) => {
        Err(Error::custom(concat!("cannot deserialize ", $name)))
      };
    }
    match &self.value {
      Value::Unit => self.deserialize_unit(visitor),
      Value::Bool(..) => self.deserialize_bool(visitor),
      Value::Char(..) => self.deserialize_char(visitor),
      Value::Integer(..) => self.deserialize_i64(visitor),
      Value::Float(..) => self.deserialize_f64(visitor),
      Value::Byte(..) => self.deserialize_u8(visitor),
      Value::Bytes(..) => self.deserialize_bytes(visitor),
      Value::String(..) => self.deserialize_string(visitor),
      Value::StaticString(..) => self.deserialize_str(visitor),
      Value::Option(..) => self.deserialize_option(visitor),
      Value::Tuple(..) => self.deserialize_seq(visitor),
      Value::Vec(..) => self.deserialize_seq(visitor),
      Value::Struct(..) => self.deserialize_map(visitor),
      Value::Object(..) => self.deserialize_map(visitor),

      // doesnt support
      Value::TupleStruct(..) => cannot!("tuple structs"),
      Value::UnitStruct(..) => cannot!("unit structs"),
      Value::Variant(..) => cannot!("variants"),
      Value::Result(..) => cannot!("results"),
      Value::Type(..) => cannot!("types"),
      Value::Future(..) => cannot!("futures"),
      Value::Stream(..) => cannot!("streams"),
      Value::Generator(..) => cannot!("generators"),
      Value::GeneratorState(..) => cannot!("generator states"),
      Value::Function(..) => cannot!("function pointers"),
      Value::Format(..) => cannot!("format specifications"),
      Value::Iterator(..) => cannot!("iterators"),
      Value::Range(..) => cannot!("range"),
      Value::Any(..) => cannot!("any"),
    }
  }

  fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Unit => {
        visitor.visit_unit()
      }
    }
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Byte(v) => visitor.visit_u8(*v)
    }
  }

  fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Bool(v) => visitor.visit_bool(*v)
    }
  }

  fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Char(v) => visitor.visit_char(*v)
    }
  }

  fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Integer(v) => visitor.visit_i64(*v)
    }
  }

  fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Float(v) => visitor.visit_f64(*v)
    }
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::String(v) => visitor.visit_string(v.borrow_ref()?.clone())
    }
  }

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::StaticString(v) => visitor.visit_str(v.as_ref())
    }
  }

  fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Bytes(v) => visitor.visit_bytes(&v.borrow_ref()?.clone())
    }
  }

  fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    Err(Error::custom("Cannot deserialize byte_buf"))
  }

  fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Tuple(v) => {
        visitor.visit_seq(SeqDeserializer::new(
          v.borrow_ref()?
            .to_vec()
            .into_iter()
            .map(IntoDeser::new),
        ))
      },
      Value::Vec(v) => visitor.visit_seq(SeqDeserializer::new(
          v.borrow_ref()?.iter().map(|v| IntoDeser::new(v.clone())),
      ))
    }
  }

  fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    deserialize_with! {
      self, visitor,
      Value::Object(v) => {
        visitor.visit_map(MapDeserializer::new(v.borrow_ref()?.iter().map(|(k, v)| {
          (StringDeser::new(k.clone()), IntoDeser::new(v.clone()))
        })))
      },
      Value::Struct(v) => {
        visitor.visit_map(MapDeserializer::new(v.borrow_ref()?.data().iter().map(|(k,v)| {
          (StringDeser::new(k.clone()), IntoDeser::new(v.clone()))
        })))
      }
    }
  }

  fn deserialize_enum<V>(
    self,
    _: &'static str,
    _: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.value {
      Value::String(v) => {
        visitor.visit_enum(v.borrow_ref()?.clone().into_deserializer())
      }
      Value::StaticString(v) => {
        visitor.visit_enum(v.to_string().into_deserializer())
      }
      _ => visitor.visit_enum(EnumSerde::new(self.value.clone())),
    }
  }

  fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.value {
      Value::Option(v) => {
        let opt = v.borrow_ref()?.clone();
        match opt {
          Some(v) => visitor.visit_some(Self::new(v)),
          None => visitor.visit_none(),
        }
      }
      _ => visitor.visit_some(Self::new(self.value)),
    }
  }

  serde::forward_to_deserialize_any! {
    i8 i16 i32 i128 u16 u32 u64 u128 f32
    unit_struct struct newtype_struct tuple
    tuple_struct identifier ignored_any
  }
}

pub struct StringDeser {
  key: String,
}

impl StringDeser {
  pub fn new(key: String) -> Self {
    Self { key }
  }
}

impl<'de> IntoDeserializer<'de, Error> for StringDeser {
  type Deserializer = Self;

  fn into_deserializer(self) -> Self::Deserializer {
    self
  }
}

impl<'de> serde::Deserializer<'de> for StringDeser {
  type Error = Error;

  fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: serde::de::Visitor<'de>,
  {
    visitor.visit_string(self.key)
  }

  serde::forward_to_deserialize_any! {
    bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
    bytes byte_buf option unit unit_struct newtype_struct tuple seq map
    tuple_struct struct enum identifier ignored_any
  }
}

pub struct EnumSerde {
  value: Value,
}

impl EnumSerde {
  pub fn new(value: Value) -> Self {
    Self { value }
  }
}

impl<'de> EnumAccess<'de> for EnumSerde {
  type Error = Error;
  type Variant = Self;

  fn variant_seed<V>(
    self,
    seed: V,
  ) -> Result<(V::Value, Self::Variant), Self::Error>
  where
    V: serde::de::DeserializeSeed<'de>,
  {
    macro_rules! deser_rtti {
      ($rtti:ident) => {{
        let types = $rtti.item.last();
        if let Some(types) = types {
          let val = seed.deserialize(StringDeser::new(types.to_string()))?;
          Ok((val, self))
        } else {
          Err(Error::custom(format!(
            "Can't find last name on {}",
            $rtti.item
          )))
        }
      }};
    }
    match &self.value {
      Value::Struct(v) => {
        let rtti = v.borrow_ref()?.rtti().clone();
        deser_rtti!(rtti)
      }
      Value::UnitStruct(v) => {
        let obj = v.borrow_ref()?.rtti().clone();
        deser_rtti!(obj)
      }
      Value::Object(v) => {
        let obj = v.borrow_ref()?.clone();
        let get = |string| {
          let val = obj.get(string);
          if let Some(val) = val {
            Ok(val)
          } else {
            Err(Error::custom(format!(
              "expecting field `{}` inside {:#?}",
              string, obj
            )))
          }
        };

        let types = get("type")?;
        let val = seed.deserialize(Deserializer::new(types.clone()))?;

        let value = get("content")?;
        Ok((val, Self::new(value.clone())))
      }
      _ => {
        let val = seed.deserialize(Deserializer::new(self.value.clone()))?;
        Ok((val, self))
      }
    }
  }
}

impl<'de> VariantAccess<'de> for EnumSerde {
  type Error = Error;

  fn unit_variant(self) -> Result<(), Self::Error> {
    match self.value {
      Value::Unit => Ok(()),
      Value::UnitStruct(..) => Ok(()),
      _ => Err(Error::custom(format!(
        "Expected unit, found: {:#?}",
        self.value,
      ))),
    }
  }

  fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
  where
    T: serde::de::DeserializeSeed<'de>,
  {
    seed.deserialize(Deserializer::new(self.value))
  }

  fn tuple_variant<V>(
    self,
    _: usize,
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    Deserializer::new(self.value).deserialize_seq(visitor)
  }

  fn struct_variant<V>(
    self,
    _: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    Deserializer::new(self.value).deserialize_map(visitor)
  }
}
