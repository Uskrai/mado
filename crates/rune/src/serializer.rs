use rune::{
    runtime::{Bytes, Object, VmError},
    ToValue, Value,
};
use serde::{
    ser::{
        Error as _, Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple,
        SerializeTupleStruct,
    },
    Serialize, Serializer,
};

#[derive(Default)]
pub struct ValueSerializer;

impl ValueSerializer {
    pub fn to_value<T>(value: T) -> Result<Value, VmError>
    where
        T: Serialize,
    {
        let this = ValueSerializer::default();
        let it: Value = value.serialize(this)?;

        Ok(it)
    }

    fn to_val<T>(value: T) -> Result<Value, Error>
    where
        T: Serialize,
    {
        Ok(Self::to_value(value)?)
    }
}

/// Convert Serializeable data to ToValue that can be used
/// to be passed to rune function that need Send
pub fn for_async_call<T>(value: T) -> impl ToValue
where
    T: Serialize,
{
    struct Ser<T>(T);
    impl<T> ToValue for Ser<T>
    where
        T: Serialize,
    {
        fn to_value(self) -> Result<Value, VmError> {
            ValueSerializer::to_value(self.0)
        }
    }
    Ser(value)
}

#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct Error(#[from] VmError);

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error(VmError::panic(format!("{}", msg)))
    }
}

impl From<Error> for VmError {
    fn from(err: Error) -> Self {
        err.0
    }
}

impl Serializer for ValueSerializer {
    type SerializeSeq = VecSerializer;
    type SerializeMap = MapSerializer;
    type SerializeTuple = TupleSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeTupleStruct = Impossible<Value, Error>;
    type SerializeTupleVariant = Impossible<Value, Error>;
    type SerializeStructVariant = Impossible<Value, Error>;

    type Ok = Value;
    type Error = Error;

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v.to_string()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(ToValue::to_value(None::<bool>)?)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Self::to_val(value)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let v = i64::try_from(v).map_err(Error::custom)?;
        self.serialize_i64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let serializer = VecSerializer {
            vec: len.map(Vec::with_capacity).unwrap_or_default(),
        };
        Ok(serializer)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let serializer = MapSerializer {
            map: len.map(Object::with_capacity).unwrap_or_default(),
        };
        Ok(serializer)
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Unit)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let serializer = StructSerializer(Object::with_capacity(len));
        Ok(serializer)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let serializer = TupleSerializer(Vec::with_capacity(len));
        Ok(serializer)
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::custom("Cannot serialize tuple struct"))
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Error::custom("Cannot serialize unit variant"))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::custom("Cannot serialize tuple variant"))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::custom("Cannot serialize struct variant"))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Err(Error::custom("Cannot serialize newtype variant"))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let bytes = Bytes::from(v.to_vec());
        Ok(bytes.to_value()?)
    }
}

#[derive(Default)]
pub struct VecSerializer {
    vec: Vec<Value>,
}

impl SerializeSeq for VecSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = ValueSerializer::to_value(value)?;
        self.vec.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::vec(self.vec))
    }
}

pub struct MapSerializer {
    map: Object,
}

impl SerializeMap for MapSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!();
    }

    fn serialize_value<T: ?Sized>(&mut self, _: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!();
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        let key = ValueSerializer::to_value(key)?
            .into_string()?
            .take()
            .unwrap();
        let value = ValueSerializer::to_value(value)?;
        self.map.insert(key, value);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(self.map))
    }
}

pub struct StructSerializer(Object);

impl SerializeStruct for StructSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value = ValueSerializer::to_value(value)?;
        self.0.insert(key.to_string(), value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(self.0))
    }
}

pub struct TupleSerializer(Vec<Value>);

impl SerializeTuple for TupleSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value = ValueSerializer::to_value(value)?;
        self.0.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let tuple = self.0.into_boxed_slice();
        let tuple = rune::runtime::Tuple::from(tuple);

        Ok(tuple.to_value()?)
    }
}

struct TupleStructSerializer(TupleSerializer);

impl SerializeTupleStruct for TupleStructSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}
