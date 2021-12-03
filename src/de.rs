use std::fs;
use std::num::{ParseFloatError, ParseIntError};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::de::value::{StrDeserializer, StringDeserializer};
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

use crate::error::{DeError, Error, Result};

pub struct Deserializer {
    /// The current path this serializer is at
    path: PathBuf,
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_fs<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_fs(s);
    Ok(T::deserialize(&mut deserializer)?)
}

impl Deserializer {
    pub fn from_fs(path: impl AsRef<Path>) -> Self {
        Deserializer {
            path: PathBuf::from(path.as_ref()),
        }
    }

    fn push(&mut self, path: &str) {
        self.path.push(path);
    }

    fn pop(&mut self) {
        self.path.pop();
    }

    // Look at the first character in the input without consuming it.
    fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let path = self.path.to_str().unwrap();
        Ok(fs::read(&self.path)?)
    }

    fn read_string(&mut self) -> Result<String> {
        Ok(String::from_utf8(self.read_bytes()?).map_err(|_| DeError::InvalidUnicode)?)
    }

    fn parse<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        let string = self.read_string()?;
        Ok(string
            .parse()
            .map_err(|_| Error::Deserialize(DeError::ParseError(string)))?)
    }

    fn path_exists(&self) -> bool {
        fs::metadata(&self.path).is_ok()
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let bytes = self.read_string()?;
        let val = match bytes.as_str() {
            "true" => true,
            "false" => false,
            a => return Err(DeError::InvalidBool(a.to_owned(), self.path.clone()).into()),
        };
        visitor.visit_bool(val)
    }

    // The `parse_signed` function is generic over the integer type `T` so here
    // it is invoked with `T=i8`. The next 8 methods are similar.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse()?)
    }

    // Float parsing is stupidly hard.
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse()?)
    }

    // Float parsing is stupidly hard.
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse()?)
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let string = self.read_string()?;
        let mut it = string.chars();
        let c = it
            .next()
            .ok_or_else(|| DeError::EmptyFile(self.path.clone()))?;

        //XXX: We could be picky and return an error about trailing characters here
        visitor.visit_char(c)
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.read_string()?)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.read_bytes()?.as_slice())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.read_bytes()?)
    }

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.path_exists() {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequentialDeserializer::new(self))
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Ok(visitor.visit_map(MapDeserializer::new(self)?)?)
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        dbg!();
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Visit a newtype variant, tuple variant, or struct variant.
        let value = visitor.visit_enum(Enum::new(self))?;
        Ok(value)
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        panic!();
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        dbg!();
        self.deserialize_unit(visitor)
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

pub struct SequentialDeserializer<'a> {
    index: usize,
    de: &'a mut Deserializer,
}

impl<'a> SequentialDeserializer<'a> {
    fn new(de: &'a mut Deserializer) -> Self {
        Self { index: 0, de }
    }

    fn deserialize_next<'de, T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        let mut bytes = [0u8; 32];
        let len = itoa::write(&mut bytes[..], self.index)?;
        let num = std::str::from_utf8(&bytes[..len]).unwrap();
        dbg!(num);

        self.de.push(num);

        if !self.de.path_exists() {
            self.de.pop();
            return Ok(None);
        }

        let val = seed.deserialize(&mut *self.de).map(Some);

        self.de.pop();
        self.index += 1;

        val
    }
}

impl<'de, 'a> SeqAccess<'de> for SequentialDeserializer<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        self.deserialize_next(seed)
    }
}

struct MapDeserializer<'a> {
    de: &'a mut Deserializer,
    it: std::fs::ReadDir,
}

impl<'a> MapDeserializer<'a> {
    fn new(de: &'a mut Deserializer) -> Result<Self> {
        let it = de.path.read_dir()?;
        Ok(Self { de, it })
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for MapDeserializer<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        let dir = self.it.next();
        match dir {
            None => Ok(None),
            Some(Err(err)) => Err(Error::IoError(err)),
            Some(Ok(dir)) => {
                let os_name = dir.file_name();
                let path = os_name.to_str().ok_or(DeError::InvalidUnicode)?;
                self.de.push(path);
                let mut de = KeyDeserializer::new(String::from(path));
                let a = Ok(Some(seed.deserialize(&mut de)?));
                a
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de);
        self.de.pop();
        val
    }
}

struct Enum<'a> {
    de: &'a mut Deserializer,
}

impl<'a> Enum<'a> {
    fn new(de: &'a mut Deserializer) -> Self {
        Enum { de }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // This is called and we have to figure which enum we are based on the current path.
        // The problem is that there are many files in the current path that might not be what we
        // want, so just iterate through the dir in deserialize_identifier hoping it gets it right?
        unimplemented!("Deserializing enums needs more work!");
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        unimplemented!()
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

struct KeyDeserializer {
    inner: String,
}

impl KeyDeserializer {
    fn new(inner: String) -> Self {
        Self { inner }
    }

    fn parse_int<T: FromStr>(&self) -> Result<T>
    where
        T: FromStr<Err = ParseIntError>,
    {
        Ok(self
            .inner
            .parse::<T>()
            .map_err(|e| Error::Deserialize(DeError::ParseError(e.to_string())))?)
    }

    fn parse_float<T: FromStr>(&self) -> Result<T>
    where
        T: FromStr<Err = ParseFloatError>,
    {
        Ok(self
            .inner
            .parse::<T>()
            .map_err(|e| Error::Deserialize(DeError::ParseError(e.to_string())))?)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut KeyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_int()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_int()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_int()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_int()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_int()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_int()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_int()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_int()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let c = self
            .inner
            .chars()
            .next()
            .ok_or(Error::Deserialize(DeError::EmptyFile(PathBuf::new())))?;

        visitor.visit_char(c)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.inner.as_str())
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.inner.clone())
    }

    serde::forward_to_deserialize_any! {

    bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test(base_dir: &str, files: Vec<(&str, &str)>) {
        let _ = std::fs::remove_dir_all(base_dir);

        for (path, expected) in files {
            let path = format!("{}/{}", base_dir, path);
            let path = Path::new(path.as_str());
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            std::fs::write(&path, expected).unwrap();
        }
    }

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct BasicTest {
            int: u32,
            seq: Vec<String>,
        }
        let test_dir = "./test-de-struct";
        setup_test(test_dir, vec![("int", "7"), ("seq/0", "a"), ("seq/1", "b")]);

        let expected = BasicTest {
            int: 7,
            seq: vec!["a".to_owned(), "b".to_owned()],
        };
        assert_eq!(expected, from_fs(test_dir).unwrap());

        use std::collections::HashMap;

        #[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
        pub struct Test {
            #[serde(rename = "in")]
            pub input: String,
            #[serde(rename = "out")]
            pub expected_output: String,
        }

        #[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
        pub struct Data {
            pub input: String,
            #[serde(rename = "p1")]
            pub part1_tests: Vec<Test>,
            #[serde(rename = "p2")]
            pub part2_tests: Option<Vec<Test>>,
        }

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Deserialize)]
        pub struct Day {
            pub year: u32,
            pub day: u32,
        }

        #[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
        pub struct Problems {
            /// Mapping of years to days to problem data
            //TODO: Make better once serde_fs supports more types as keys
            years: HashMap<u32, HashMap<u32, Data>>,
            session: String,
        }

        let mut year2020 = HashMap::new();
        year2020.insert(
            3,
            Data {
                input: "I am input".to_owned(),
                part1_tests: vec![Test {
                    input: "b".to_owned(),
                    expected_output: "b".to_owned(),
                }],
                part2_tests: None,
            },
        );

        let mut years = HashMap::new();
        years.insert(2020, year2020);

        let expected = Problems {
            years,
            session: "ABCD167".to_owned(),
        };

        setup_test(
            test_dir,
            vec![
                ("session", "ABCD167"),
                ("years/2020/3/input", "I am input"),
                ("years/2020/3/p1/0/out", "b"),
                ("years/2020/3/p1/0/in", "b"),
            ],
        );

        assert_eq!(expected, from_fs(test_dir).unwrap());

        //Enum tests to be implemented
        /*
        #[derive(Deserialize, PartialEq, Debug)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        setup_test(test_dir, vec![("Unit", "")]);
        let expected = E::Unit;
        assert_eq!(expected, from_fs(test_dir).unwrap());

        setup_test(test_dir, vec![("Newtype", "8")]);
        let expected = E::Newtype(8);
        assert_eq!(expected, from_fs(test_dir).unwrap());

        setup_test(test_dir, vec![("Tuple/0", "1"), ("Tuple/1", "2")]);
        let expected = E::Tuple(1, 2);
        assert_eq!(expected, from_fs(test_dir).unwrap());

        setup_test(test_dir, vec![("Struct/a", "14")]);
        let expected = E::Struct { a: 14 };
        assert_eq!(expected, from_fs(test_dir).unwrap());
        */
        let _ = std::fs::remove_dir_all(test_dir);
    }
}
