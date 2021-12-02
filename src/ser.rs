use std::fs;
use std::path::{Path, PathBuf};

use serde::{ser, Serialize};

use crate::error::{Error, Result};

pub struct Serializer {
    /// The current path this serializer is at
    path: PathBuf,
}

pub fn to_fs<T>(value: &T, path: impl AsRef<Path>) -> Result<()>
where
    T: Serialize,
{
    let mut serializer = Serializer::new(path)?;
    value.serialize(&mut serializer)?;
    Ok(())
}

impl Serializer {
    fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = PathBuf::from(path.as_ref());
        Ok(Self { path })
    }

    /// Writes data to the current file position.
    ///
    /// Calling this function repeadiadely without calling [`push`] or [`pop`] will result in data
    /// loss
    fn write_data(&mut self, s: impl AsRef<[u8]>) -> Result<()> {
        match fs::create_dir_all(&self.path.parent().unwrap()) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err.into()),
        }
        fs::write(&self.path, s.as_ref())?;
        Ok(())
    }

    /// Pushes `path` to the current path pointer so that later calls to [`write_data`] create the
    /// parent directories pushed, with the file name being the last item to be pushed
    fn push(&mut self, path: &str) -> Result<()> {
        self.path.push(path);
        Ok(())
    }

    fn pop(&mut self) {
        self.path.pop();
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();

    // The error type when some error occurs during serialization.
    type Error = Error;

    type SerializeSeq = SequentialSerializer<'a>;
    type SerializeTuple = SequentialSerializer<'a>;
    type SerializeTupleStruct = SequentialSerializer<'a>;
    type SerializeTupleVariant = SequentialSerializer<'a>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        let s = if v { "true" } else { "false" };
        self.write_data(s)
    }

    //We do not distinguish between integer types
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let mut bytes = [0u8; 32];
        let len = itoa::write(&mut bytes[..], v)?;
        self.write_data(&bytes[0..len])?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        let mut bytes = [0u8; 32];
        let len = itoa::write(&mut bytes[..], v)?;
        self.write_data(&bytes[..len])?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write_data(v.to_string())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write_data(v.to_string())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.write_data(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.write_data(v)
    }

    // An absent optional is represented as the JSON `null`.
    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        //Nop as we are serializing `()`
        Ok(())
    }

    // Unit struct means a named value containing no data
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    // When serializing a unit variant (or any other kind of variant), formats
    // can choose whether to keep track of it by index or by name. Binary
    // formats typically use the index of the variant and human-readable formats
    // typically use the name.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.push(variant)?;
        self.serialize_str("")?;
        self.pop();
        Ok(())
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain.
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // Note that newtype variant (and all of the other variant serialization
    // methods) refer exclusively to the "externally tagged" enum
    // representation.
    //
    // Serialize this to JSON in externally tagged form as `{ NAME: VALUE }`.
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.push(variant)?;
        value.serialize(&mut *self)?;
        self.pop();
        Ok(())
    }

    // Now we get to the serialization of compound types.
    //
    // The start of the sequence, each value, and the end are three separate
    // method calls. This one is responsible only for serializing the start,
    // which in JSON is `[`.
    //
    // The length of the sequence may or may not be known ahead of time. This
    // doesn't make a difference in JSON because the length is not represented
    // explicitly in the serialized form. Some serializers may only be able to
    // support sequences for which the length is known up front.
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SequentialSerializer::new(self))
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently by omitting the length, since tuple
    // means that the corresponding `Deserialize implementation will know the
    // length without needing to look at the serialized data.
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SequentialSerializer::new(self))
    }

    // Tuple structs look just like sequences in JSON.
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SequentialSerializer::new(self))
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }`. Again
    // this method is only responsible for the externally tagged representation.
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.push(variant)?;
        Ok(SequentialSerializer::new(self))
    }

    // Maps are represented in JSON as `{ K: V, K: V, ... }`.
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    // Structs look just like maps in JSON. In particular, JSON requires that we
    // serialize the field names of the struct. Other formats may be able to
    // omit the field names when serializing structs because the corresponding
    // Deserialize implementation is required to know what the keys are without
    // looking at the serialized data.
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }`.
    // This is the externally tagged representation.
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.push(variant)?;
        Ok(self)
    }
}

pub struct SequentialSerializer<'a> {
    index: usize,
    ser: &'a mut Serializer,
}

impl<'a> SequentialSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        Self { index: 0, ser }
    }

    fn serialize<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let mut bytes = [0u8; 32];
        let len = itoa::write(&mut bytes[..], self.index)?;
        let num = std::str::from_utf8(&bytes[..len]).unwrap();

        self.ser.push(num)?;
        value.serialize(&mut *self.ser)?;
        self.ser.pop();
        self.index += 1;

        Ok(())
    }
}

impl<'a> SerializeSeq for SequentialSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeTuple for SequentialSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for SequentialSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Tuple variants are a little different. Refer back to the
// `serialize_tuple_variant` method above:
//
//    self.output += "{";
//    variant.serialize(&mut *self)?;
//    self.output += ":[";
//
// So the `end` method in this impl is responsible for closing both the `]` and
// the `}`.
impl<'a> ser::SerializeTupleVariant for SequentialSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Some `Serialize` types are not able to hold a key and value in memory at the
// same time so `SerializeMap` implementations are required to support
// `serialize_key` and `serialize_value` individually.
//
// There is a third optional method on the `SerializeMap` trait. The
// `serialize_entry` method allows serializers to optimize for the case where
// key and value are both available simultaneously. In JSON it doesn't make a
// difference so the default behavior for `serialize_entry` is fine.
impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    // The Serde data model allows map keys to be any serializable type. JSON
    // only allows string keys so the implementation below will produce invalid
    // JSON if the key serializes as something other than a string.
    //
    // A real JSON serializer would need to validate that map keys are strings.
    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        //convert key to string so we can stick in path
        let mut str_serializer = StringSerializer::new();
        key.serialize(&mut str_serializer)?;
        let name = str_serializer.finish();
        self.push(name.as_str())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        self.pop();

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.push(key)?;
        value.serialize(&mut **self)?;
        self.pop();

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.push(key)?;
        value.serialize(&mut **self)?;
        self.pop();

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.pop();

        Ok(())
    }
}

struct StringSerializer {
    s: String,
}

#[track_caller]
fn unsupported() -> ! {
    panic!("Unsupported")
}

impl StringSerializer {
    fn new() -> Self {
        Self { s: String::new() }
    }

    fn set_str(&mut self, new_string: impl ToString) -> Result<()> {
        debug_assert!(self.s.is_empty());
        self.s = new_string.to_string();
        Ok(())
    }

    fn finish(self) -> String {
        self.s
    }
}

use serde::ser::{Impossible, SerializeSeq, SerializeTuple, SerializeTupleStruct};
impl<'a> ser::Serializer for &'a mut StringSerializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        if v {
            self.set_str("true")
        } else {
            self.set_str("false")
        }
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.set_str(v)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.set_str(String::from(v))
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        unsupported()
    }

    fn serialize_none(self) -> Result<()> {
        unsupported()
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        unsupported()
    }

    fn serialize_unit(self) -> Result<()> {
        unsupported()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        unsupported()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        unsupported()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        unsupported()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        unsupported()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        unsupported()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        unsupported()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        unsupported()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        unsupported()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        unsupported()
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        unsupported()
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        unsupported()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn check_and_reset(base_dir: &str, files: Vec<(&str, &str)>) {
        for (path, expected) in files {
            let path = format!("{}/{}", base_dir, path);
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(err) => panic!("Failed to open file {}: {}", path, err),
            };
            let actual = std::str::from_utf8(&bytes[..]).unwrap();
            if expected != actual {
                println!("{:?} {:?}", expected, actual);
                panic!("In file {}: expected {}, got {}", path, expected, actual);
            }
        }

        //Reset for next time
        std::fs::remove_dir_all(base_dir).unwrap();
    }

    #[test]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    fn test_struct() {
        #[derive(Serialize)]
        struct Test {
            int: u32,
            seq: Vec<&'static str>,
        }

        #[derive(Serialize)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        let base_dir = "./test-ser-struct";

        let test = Test {
            int: 100,
            seq: vec!["a", "b"],
        };

        to_fs(&test, base_dir).unwrap();
        check_and_reset(
            base_dir,
            vec![("int", "100"), ("seq/0", "a"), ("seq/1", "b")],
        );

        let u = E::Unit;
        to_fs(&u, base_dir).unwrap();
        check_and_reset(base_dir, vec![("Unit", "")]);

        let n = E::Newtype(1);
        to_fs(&n, base_dir).unwrap();
        check_and_reset(base_dir, vec![("Newtype", "1")]);

        let t = E::Tuple(1, 10);
        to_fs(&t, base_dir).unwrap();
        check_and_reset(base_dir, vec![("Tuple/0", "1"), ("Tuple/1", "10")]);

        let s = E::Struct { a: 510 };
        to_fs(&s, base_dir).unwrap();
        check_and_reset(base_dir, vec![("Struct/a", "510")]);
    }
}
