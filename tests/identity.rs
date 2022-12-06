#![allow(dead_code)]
use std::{collections::BTreeMap, ops::Range};

use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum BasicEnum {
    A,
    B,
    C,
    D,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum AdvEnum {
    Ok(String),
    Err(String),
    Struct { a: String, b: u8, c: char },
    Tup(String, Vec<u8>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct JsonInner {
    user_count: usize,
    map: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Inner {
    map: BTreeMap<String, BasicEnum>,
    map2: BTreeMap<BasicEnum, String>,
    bytes: [u8; 4],
    strings: Vec<String>,
    json: JsonInner,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct S {
    i_u8: u8,
    i_u16: u16,
    i_u32: u32,
    i_u64: u64,
    i_usize: usize,
    i_i8: i8,
    i_i16: i16,
    i_i32: i32,
    i_i64: i64,
    i_isize: isize,
    boolean: bool,
    c: char,
    f_f32: f32,
    f_f64: f64,
    // we cant support borowwed data, since the bytes to reference are on disk and not in the
    // user's object we are deserializing into
    //s: &'s str,
    string: String,
    // same as &str, not supported
    //b: &'b [u8],
    #[serde(with = "serde_bytes")]
    bytes: Vec<u8>,
    opt: Option<String>,
    unit_variant: BasicEnum,
    newtype: Millimeters,
    advanced_enum: AdvEnum,
    tup: (u8, u32, String),
    inner: Inner,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Millimeters(u8);

#[test]
fn identity() {
    for _ in 0..1_000 {
        let test_dir = "/tmp/.test-identity";
        let _ = std::fs::remove_dir_all(test_dir);

        let mut rng = rand::thread_rng();
        let expected = S::random(&mut rng);
        serde_fs::to_fs(&expected, test_dir).unwrap();
        let actual: S = serde_fs::from_fs(test_dir).unwrap();
        pretty_assertions::assert_eq!(expected, actual);
    }
}

impl BasicEnum {
    fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..4) {
            0 => BasicEnum::A,
            1 => BasicEnum::B,
            2 => BasicEnum::C,
            3 => BasicEnum::D,
            _ => unreachable!(),
        }
    }
}

fn rand_string(rng: &mut impl Rng, range: Range<usize>) -> String {
    range.map(|_| rng.sample(Alphanumeric) as char).collect()
}

impl AdvEnum {
    fn random(rng: &mut impl Rng) -> Self {
        let s: String = rand_string(rng, 8..32);

        match rng.gen_range(0..2) {
            0 => AdvEnum::Ok(s),
            1 => AdvEnum::Err(s),
            _ => unreachable!(),
        }
    }
}

/// Creates an iterator sutible for creating maps
fn map_iter<K, V, R>(
    count: usize,
    rng: &mut R,
    key: impl Fn(&mut R) -> K + 'static,
    value: impl Fn(&mut R) -> V + 'static,
) -> impl Iterator<Item = (K, V)> + '_
where
    R: Rng,
{
    (0..count).map(move |_| (key(rng), value(rng)))
}

impl JsonInner {
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            user_count: rng.gen(),
            map: BTreeMap::from_iter(map_iter(
                rng.gen_range(2..4),
                rng,
                |rng| rand_string(rng, 4..8),
                |rng| rand_string(rng, 32..64),
            )),
        }
    }
}

impl Inner {
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            map: BTreeMap::from_iter(map_iter(
                rng.gen_range(16..32),
                rng,
                |rng| rand_string(rng, 8..32),
                |rng| BasicEnum::random(rng),
            )),
            map2: BTreeMap::from_iter(map_iter(
                rng.gen_range(1..4),
                rng,
                |rng| BasicEnum::random(rng),
                |rng| rand_string(rng, 8..32),
            )),
            bytes: rng.gen(),
            strings: (0..rng.gen_range(8..32))
                .map(|_| rand_string(rng, 8..32))
                .collect(),
            json: JsonInner::random(rng),
        }
    }
}

impl S {
    fn random(rng: &mut impl Rng) -> Self {
        let s1: String = rand_string(rng, 8..32);
        let s2: String = rand_string(rng, 8..32);
        let s3: String = rand_string(rng, 8..32);
        let bytes: Vec<u8> = (0..rng.gen_range(8..17)).map(|_| rng.gen()).collect();
        Self {
            i_u8: rng.gen(),
            i_u16: rng.gen(),
            i_u32: rng.gen(),
            i_u64: rng.gen(),
            i_i8: rng.gen(),
            i_i16: rng.gen(),
            i_i32: rng.gen(),
            i_i64: rng.gen(),
            unit_variant: BasicEnum::random(rng),
            advanced_enum: AdvEnum::random(rng),
            i_usize: rng.gen(),
            i_isize: rng.gen(),
            tup: (rng.gen(), rng.gen(), s1),
            boolean: rng.gen(),
            c: rng.gen(),
            // generate integers to start so that we avoid percision issues causing values to be
            // deseralised differently
            f_f32: rng.gen::<u16>() as f32 / 2.0,
            f_f64: rng.gen::<u16>() as f64 / 16.0,
            string: s2,
            bytes,
            opt: match rng.gen() {
                true => Some(s3),
                false => None,
            },
            newtype: Millimeters(rng.gen()),
            inner: Inner::random(rng),
        }
    }
}
