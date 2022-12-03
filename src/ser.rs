use std::fs;
use std::path::{Path, PathBuf};

use serde::{ser, Serialize};

use crate::error::SerError;

type Error = SerError;
pub type Result<T> = std::result::Result<T, Error>;

pub struct Serializer {
    /// The current path this serializer is at
    path: PathBuf,
    path_dirty: bool,
    /// How many push we have
    dir_level: usize,
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
        Ok(Self {
            path,
            path_dirty: false,
            dir_level: 0,
        })
    }

    /// Writes data to the current file position.
    ///
    /// # Panics
    /// This function panics if it is called representedly without a call to [`pop`] before.
    /// This is done to prevet data loss, as there may be data already written to the current path
    /// that we cant overwrite
    fn write_data(&mut self, s: impl AsRef<[u8]>) -> Result<()> {
        dbg!(self.dir_level);
        if self.path_dirty {
            panic!("BUG: path dirty: {}", self.path.to_string_lossy());
        }
        assert!(self.dir_level > 0);
        match fs::create_dir_all(&self.path.parent().unwrap()) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err.into()),
        }
        fs::write(&self.path, s.as_ref())?;
        self.path_dirty = true;
        Ok(())
    }

    /// Pushes `path` to the current path pointer so that later calls to [`write_data`] create the
    /// parent directories pushed, with the file name being the last item to be pushed
    fn push(&mut self, path: &str) -> Result<()> {
        self.path.push(path);
        self.dir_level += 1;
        Ok(())
    }

    fn pop(&mut self) {
        self.path.pop();
        self.dir_level -= 1;
        self.path_dirty = false;
    }

    /// Returns Err(..) if no paths have been pushed yet
    fn fail_if_at_root(&self, msg: &'static str) -> Result<()> {
        if self.dir_level == 0 {
            Err(Error::NotSupportedAtRootLevel(msg))
        } else {
            Ok(())
        }
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();

    // The error type when some error occurs during serialization.
    type Error = SerError;

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
        self.fail_if_at_root("i8's")?;
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.fail_if_at_root("i16's")?;
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.fail_if_at_root("i32's")?;
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.fail_if_at_root("i64's")?;
        let mut bytes = [0u8; 32];
        let len = itoa::write(&mut bytes[..], v)?;
        self.write_data(&bytes[0..len])?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.fail_if_at_root("u8's")?;
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.fail_if_at_root("u16's")?;
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.fail_if_at_root("u32's")?;
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.fail_if_at_root("u64's")?;
        let mut bytes = [0u8; 32];
        let len = itoa::write(&mut bytes[..], v)?;
        self.write_data(&bytes[..len])?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.fail_if_at_root("f32's")?;
        self.write_data(v.to_string())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.fail_if_at_root("f64's")?;
        self.write_data(v.to_string())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.fail_if_at_root("chars")?;
        let mut bytes = [0u8; 8];
        v.encode_utf8(&mut bytes);
        self.write_data(bytes)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.fail_if_at_root("str's")?;
        self.write_data(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.fail_if_at_root("bytes")?;
        self.write_data(v)
    }

    fn serialize_none(self) -> Result<()> {
        self.fail_if_at_root("options")?;
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        dbg!(self.dir_level);
        self.fail_if_at_root("units")?;
        // write empty file
        self.write_data(&[])
    }

    // Unit struct means a named value containing no data
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        dbg!(self.dir_level);
        self.fail_if_at_root("unit structs")?;
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        dbg!(self.dir_level);
        self.fail_if_at_root("enums")?;
        self.serialize_str(variant)?;
        Ok(())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        dbg!(self.dir_level);
        value.serialize(self)
    }

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
        dbg!(self.dir_level);
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
        dbg!(self.dir_level);
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
        dbg!(self.dir_level);
        self.push(variant)?;
        Ok(SequentialSerializer::new(self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        dbg!(self.dir_level);
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

    type Error = SerError;

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

    type Error = SerError;

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

    type Error = SerError;

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

impl<'a> ser::SerializeTupleVariant for SequentialSerializer<'a> {
    type Ok = ();
    type Error = SerError;

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

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = SerError;

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
    type Error = SerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.push(key)?;
        if key.starts_with("json") {
            let s = serde_json::to_string(value)?;
            s.serialize(&mut **self)?;
        } else {
            value.serialize(&mut **self)?;
        }
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
    type Error = SerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.push(key)?;
        if key.starts_with("json") {
            let s = serde_json::to_string(value)?;
            s.serialize(&mut **self)?;
        } else {
            value.serialize(&mut **self)?;
        }
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
    type Error = SerError;
    type SerializeSeq = Impossible<(), SerError>;
    type SerializeTuple = Impossible<(), SerError>;
    type SerializeTupleStruct = Impossible<(), SerError>;
    type SerializeTupleVariant = Impossible<(), SerError>;
    type SerializeMap = Impossible<(), SerError>;
    type SerializeStruct = Impossible<(), SerError>;
    type SerializeStructVariant = Impossible<(), SerError>;

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
        variant: &'static str,
    ) -> Result<()> {
        self.set_str(String::from(variant))
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
    use std::collections::BTreeMap;

    use super::*;

    fn check_and_reset(test_dir: &str, files: Vec<(&str, &str)>) {
        for (path, expected) in files {
            let path = format!("{}/{}", test_dir, path);
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
        std::fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    #[allow(dead_code)]
    fn test_struct() {
        #[derive(Serialize)]
        struct Test {
            int: u32,
            seq: Vec<&'static str>,
        }

        let test_dir = "./.test-ser-struct";
        let _ = std::fs::remove_dir_all(test_dir);

        let test = Test {
            int: 100,
            seq: vec!["a", "b"],
        };

        to_fs(&test, test_dir).unwrap();
        check_and_reset(
            test_dir,
            vec![("int", "100"), ("seq/0", "a"), ("seq/1", "b")],
        );
    }

    #[test]
    #[allow(dead_code)]
    fn test_unit_enum() {
        let test_dir = "./.test-ser-unit-enum";
        let _ = std::fs::remove_dir_all(test_dir);

        #[derive(Serialize)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        #[derive(Serialize)]
        struct X {
            e: E,
        }

        dbg!();
        let u = X { e: E::Unit };
        to_fs(&u, test_dir).unwrap();
        check_and_reset(test_dir, vec![("e", "Unit")]);

        dbg!();
        let n = E::Newtype(1);
        to_fs(&n, test_dir).unwrap();
        check_and_reset(test_dir, vec![("Newtype", "1")]);

        dbg!();
        let t = E::Tuple(1, 10);
        to_fs(&t, test_dir).unwrap();
        check_and_reset(test_dir, vec![("Tuple/0", "1"), ("Tuple/1", "10")]);

        dbg!();
        let s = E::Struct { a: 510 };
        to_fs(&s, test_dir).unwrap();
        check_and_reset(test_dir, vec![("Struct/a", "510")]);
    }

    #[test]
    #[allow(dead_code)]
    fn test_json() {
        let test_dir = "./.test-ser-json";
        let _ = std::fs::remove_dir_all(test_dir);

        #[derive(Serialize)]
        enum Enum {
            Inner {
                json: BTreeMap<&'static str, &'static str>,
            },
        }

        let u = Enum::Inner {
            json: [("k1", "v1"), ("k2", "v2")].into(),
        };
        to_fs(&u, test_dir).unwrap();
        check_and_reset(test_dir, vec![("Inner/json", r#"{"k1":"v1","k2":"v2"}"#)]);

        #[derive(Serialize)]
        struct Basic {
            json: u8,
            json_comp: String,
        }

        let u = Basic {
            json: 0,
            json_comp: "abc".into(),
        };
        to_fs(&u, test_dir).unwrap();
        check_and_reset(test_dir, vec![("json", "0"), ("json_comp", "\"abc\"".into())]);

        #[derive(Serialize)]
        struct Struct {
            // make sure renaming works
            #[serde(rename = "json")]
            my_map: BTreeMap<&'static str, &'static str>,
        }

        let u = Struct {
            my_map: [("k1", "v1"), ("k2", "v2")].into(),
        };
        to_fs(&u, test_dir).unwrap();
        check_and_reset(test_dir, vec![("json", r#"{"k1":"v1","k2":"v2"}"#)]);
    }
}
