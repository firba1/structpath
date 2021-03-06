//! structpath is a libary which allows parsing and generating url paths in a convenient type safe way.
//!
//! structpath leverages serde to help parse values into structs.
//!
//! # Examples
//!
//! ## Basic example
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//! use structpath::Schema;
//!
//! #[derive(Deserialize)]
//! struct FooParams {
//!     foo_id: u128,
//!     bar: String,
//! }
//!
//! const foo_path = "/foo/<foo_id:u128>/bar/<bar>";
//!
//! // This is a general idea of a web request handler, not important for the demonstration
//! fn foo_bar(request: Request) -> Response {
//!     let params: FooParams = Schema::path(foo_path).parse(request.path);
//! }
//!
//! fn baz(request: Request) -> Response {
//!     let foo_path = Schema::path(foo_path).generate(FooParams{foo_id: foo_id, bar: bar});
//!     Response::Redirect(foo_path)
//! }
//! ```

extern crate serde;
extern crate thiserror;

use std::collections::HashMap;
use thiserror::Error;
use std::num::{ParseFloatError, ParseIntError};
use serde::de::Visitor;
use std::fmt::Display;

/// SegmentType is a basic enum for specifying what type a segment's value is.
#[derive(PartialEq, Debug)]
pub enum SegmentType {
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    String,
}

/// SegmentValueSchema holds the schema for a particular value segment.
///
/// See `SegmentSchema` for more details.
#[derive(PartialEq, Debug)]
pub struct SegmentValueSchema {
    name: String,
    segment_type: SegmentType,
}

/// SegmentValue holds a parsed value
///
/// Usually you should not construct one of these yourself
#[derive(PartialEq, Debug, Clone)]
pub enum SegmentValue {
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    String(String),
}


/// SegmentSchema is the schema for a particular path segment
///
/// `Literal` is a schema for an invairant string literal segment
///
/// `Value` is a schema for a segment containing a value to be parsed
#[derive(PartialEq, Debug)]
pub enum SegmentSchema {
    Literal(String),
    Value(SegmentValueSchema),
}

/// Schema hold the schema definition for a particular url path pattern.
///
/// Generally a `Schema` will map 1-to-1 to a particular request handler.
#[derive(PartialEq, Debug)]
pub struct Schema {
    segments: Vec<SegmentSchema>,
}

/// Error type for parsing Schemas from a String
#[derive(Error, Debug)]
pub enum PathSchemaParseError {
    #[error("Path schema syntax error in {segment:?}: {message}")]
    SyntaxError{
        segment: String,
        message: String,
    },
    #[error("Unrecognized type: {0}")]
    UnrecognizedType(String),
}

/// Schema for a url path
///
/// Schema objects can be used to parse or generate corresponding paths
///
/// # Examples
///
/// ## Using Schema::path
///
/// The quickest way to create a Schema is usually to use the Schema::path method.
///
/// ```
/// use structpath::Schema;
/// let foo_bar_path = Schema::path("/foo/<foo_id:u64>/bar/<bar>");
/// ```
/// This will create schema with 4 segements:
/// - literal "foo"
/// - u64 field foo_id
/// - literal "bar"
/// - String field bar
///
/// ## Using builder pattern
///
/// A more verbose, but a bit more explicit way to create the same value as above is to use the
/// builder pattern:
///
/// ```
/// use structpath::{Schema, SegmentType};
///
/// let foo_bar_path = Schema::new()
///     .literal("foo")
///     .value("foo_id", SegmentType::U64)
///     .literal("bar")
///     .value("bar", SegmentType::String);
///
/// ```
impl Schema {
    /// Create a blank Schema, typically done when using builder pattern
    pub fn new() -> Self {
        Self{segments: vec![]}
    }

    /// Create a Schema from a path schema string, see above example.
    pub fn path<S: Into<String>>(path: S) -> Result<Self, PathSchemaParseError> {
        let mut schema = Schema::new();
        for segment in path.into().split("/").skip(1) {
            if &segment[0..1] == "<" {
                let no_brackets: String = segment.chars().skip(1).take_while(|c| c != &'>').collect();
                let chunks: Vec<&str> = no_brackets.split(":").collect();
                if chunks.len() > 2 {
                    return Err(PathSchemaParseError::SyntaxError{
                        segment: segment.to_owned(),
                        message: "Expected at most one ':' in path segment".to_owned(),
                    });
                } else if chunks.len() == 2 {
                    let name = chunks[0];
                    let segment_type = match chunks[1] {
                        "f32" => SegmentType::F32,
                        "f64" => SegmentType::F64,
                        "u8" => SegmentType::U8,
                        "u16" => SegmentType::U16,
                        "u32" => SegmentType::U32,
                        "u64" => SegmentType::U64,
                        "u128" => SegmentType::U128,
                        "i8" => SegmentType::I8,
                        "i16" => SegmentType::I16,
                        "i32" => SegmentType::I32,
                        "i64" => SegmentType::I64,
                        "i128" => SegmentType::I128,
                        "String" => SegmentType::String,
                        _ => {
                            return Err(PathSchemaParseError::UnrecognizedType(chunks[1].to_owned()))
                        },
                    };
                    schema.segments.push(SegmentSchema::Value(SegmentValueSchema{
                        name: name.to_owned(),
                        segment_type: segment_type,
                    }))
                } else { // chunks.len() == 1
                    schema.segments.push(SegmentSchema::Value(SegmentValueSchema{
                        name: chunks[0].to_owned(),
                        segment_type: SegmentType::String,
                    }));
                }
            } else {
                schema.segments.push(SegmentSchema::Literal(segment.to_owned()));
            }
        }
        Ok(schema)
    }

    /// Append a literal to the `Schema`
    ///
    /// e.g. `Schema::new().literal("foo")` would match the path `"/foo"`
    pub fn literal<S: Into<String>>(mut self, segment_literal: S) -> Self {
        self.segments.push(SegmentSchema::Literal(segment_literal.into()));
        self
    }

    /// Append a value to the `Schema`
    ///
    /// e.g. `Schema::new().value("foo", SegmentType::I64)` is equivalent to
    /// `Schema::path("/<foo:i64>")`
    pub fn value<S: Into<String>>(mut self, name: S, segment_type: SegmentType) -> Self {
        self.segments.push(SegmentSchema::Value(SegmentValueSchema{name: name.into(), segment_type: segment_type}));
        self
    }

    /// Parse a concrete path into a value, using this `Schema`
    pub fn parse<'a, S, T>(&self, path: S) -> Result<T, StructPathError> where S: Into<String>, T: serde::Deserialize<'a> {
        parse_path(path, self)
    }

    /// Create a path String from parameters and this `Schema`
    pub fn generate<T>(&self, parameters: &T) -> Result<String, StructPathError> where T: serde::Serialize {
        generate_path(parameters, self)
    }
}

/// General error type for errors when parsing or generating urls
#[derive(Error, Debug)]
pub enum StructPathError {
    #[error("Incorrect path segment (expected {expected:?}, got {got:?})")]
    IncorrectSegment{
        got: String,
        expected: String,
    },
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error("Error from serde: {0}")]
    SerdeInternalError(String),
    #[error("Error is impossible, but reqired structurrally")]
    Impossible,
    #[error("Expected {0}, but got {1:?}")]
    ExpectedType(String, SegmentValue),
    #[error("Not supported: {0}")]
    NotSupported(String),
    #[error("Expected field {0:?} missing from input")]
    MissingField(String),
    #[error("Expected state(s): {expected}, got {got:?}")]
    InvalidSerializerState{
        expected: String,
        got: SerializerState,
    },
    #[error("Expected state(s): {expected}, got {got:?}")]
    InvalidDeserializerState{
        expected: String,
        got: DeserializerState,
    },
}

impl serde::de::Error for StructPathError {
    fn custom<T>(msg: T) -> Self where T: Display {
        StructPathError::SerdeInternalError(msg.to_string())
    }
}

impl serde::ser::Error for StructPathError {
    fn custom<T>(msg: T) -> Self where T: Display {
        StructPathError::SerdeInternalError(msg.to_string())
    }
}

fn parse_path_generic(path: String, schema: &Schema) -> Result<HashMap<String, SegmentValue>, StructPathError> {
    let mut path_values = HashMap::new();
    for (segment, segment_schema) in path.split("/").skip(1).zip(schema.segments.iter()) {
        match segment_schema {
            SegmentSchema::Literal(literal) => {
                if segment != literal {
                    return Err(StructPathError::IncorrectSegment{got: segment.to_owned(), expected: literal.clone()});
                }
            }
            SegmentSchema::Value(segment_value_schema) => {
                match segment_value_schema.segment_type {
                    SegmentType::F32 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::F32(segment.parse()?));
                    },
                    SegmentType::F64 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::F64(segment.parse()?));
                    },
                    SegmentType::I8 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::I8(segment.parse()?));
                    },
                    SegmentType::I16 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::I16(segment.parse()?));
                    },
                    SegmentType::I32 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::I32(segment.parse()?));
                    },
                    SegmentType::I64 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::I64(segment.parse()?));
                    },
                    SegmentType::I128 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::I128(segment.parse()?));
                    },
                    SegmentType::U8 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::U8(segment.parse()?));
                    },
                    SegmentType::U16 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::U16(segment.parse()?));
                    },
                    SegmentType::U32 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::U32(segment.parse()?));
                    },
                    SegmentType::U64 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::U64(segment.parse()?));
                    },
                    SegmentType::U128 => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::U128(segment.parse()?));
                    },
                    SegmentType::String => {
                        path_values.insert(segment_value_schema.name.clone(), SegmentValue::String(segment.to_owned()));
                    },
                }
            },
        }
    }
    Ok(path_values)
}

/// Internal state for Deserializer, usually only useful for debugging.
#[derive(Clone, Debug)]
pub enum DeserializerState {
    Start,
    Map,
    MapKey(String),
    MapValue(SegmentValue),
    End,
}

struct Deserializer {
    generic_parsed_path: HashMap<String, SegmentValue>,
    state: DeserializerState,
}

impl <'de, 'a> serde::de::Deserializer<'de> for &'a mut Deserializer {
    type Error = StructPathError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("deserialize_any".to_owned()))
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("bool".to_owned()))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::I8(value) => (DeserializerState::Map, visitor.visit_i8(*value)),
                _ => return Err(StructPathError::ExpectedType("i8".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::I16(value) => (DeserializerState::Map, visitor.visit_i16(*value)),
                _ => return Err(StructPathError::ExpectedType("i16".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::I32(value) => (DeserializerState::Map, visitor.visit_i32(*value)),
                _ => return Err(StructPathError::ExpectedType("i32".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::I64(value) => (DeserializerState::Map, visitor.visit_i64(*value)),
                _ => return Err(StructPathError::ExpectedType("i64".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::I128(value) => (DeserializerState::Map, visitor.visit_i128(*value)),
                _ => return Err(StructPathError::ExpectedType("i128".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::U8(value) => (DeserializerState::Map, visitor.visit_u8(*value)),
                _ => return Err(StructPathError::ExpectedType("u8".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::U64(value) => (DeserializerState::Map, visitor.visit_u64(*value)),
                _ => return Err(StructPathError::ExpectedType("u64".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::U64(value) => (DeserializerState::Map, visitor.visit_u64(*value)),
                _ => return Err(StructPathError::ExpectedType("u64".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::U64(value) => (DeserializerState::Map, visitor.visit_u64(*value)),
                _ => return Err(StructPathError::ExpectedType("u64".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::U128(value) => (DeserializerState::Map, visitor.visit_u128(*value)),
                _ => return Err(StructPathError::ExpectedType("u128".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::F32(value) => (DeserializerState::Map, visitor.visit_f32(*value)),
                _ => return Err(StructPathError::ExpectedType("f32".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::F64(value) => (DeserializerState::Map, visitor.visit_f64(*value)),
                _ => return Err(StructPathError::ExpectedType("f64".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("char".to_owned()))
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let (next_state, result) = match &self.state {
            DeserializerState::MapValue(segment_value) => match segment_value {
                SegmentValue::String(value) => (DeserializerState::Map, visitor.visit_string(value.clone())),
                _ => return Err(StructPathError::ExpectedType("String".to_owned(), segment_value.clone())),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_owned(),
                got: self.state.clone(),
            })
        };
        self.state = next_state;
        result
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("bytes".to_owned()))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("bytes_buf".to_owned()))
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("Option".to_owned()))
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("()".to_owned()))
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("unit struct".to_owned()))
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("newtype struct".to_owned()))
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("sequence".to_owned()))
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("tuple".to_owned()))
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("tuple struct".to_owned()))
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.state = match self.state {
            DeserializerState::Start => DeserializerState::Map,
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "Start".to_owned(),
                got: self.state.clone(),
            }),
        };
        visitor.visit_map(self)
    }

    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("enum".to_owned()))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match &self.state {
            DeserializerState::MapKey(key) => visitor.visit_string(key.clone()),
            _ => Err(StructPathError::InvalidDeserializerState{
                expected: "MapKey".to_string(),
                got: self.state.clone(),
            }),
        }
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(StructPathError::NotSupported("deserialize_ignored_any".to_owned()))
    }
}

impl<'de, 'a> serde::de::MapAccess<'de> for &'a mut Deserializer {
    type Error = StructPathError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: serde::de::DeserializeSeed<'de> {
        let (has_next_key, next_state) = match self.state {
            DeserializerState::Map => {
                match self.generic_parsed_path.keys().nth(0) {
                    Some(key) => (true, DeserializerState::MapKey(key.clone())),
                    None => (false, DeserializerState::End),
                }
            }
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "Map".to_string(),
                got: self.state.clone(),
            }),
        };
        self.state = next_state;
        if !has_next_key {
            return Ok(None)
        }
        match &self.state {
            DeserializerState::MapKey(_) => seed.deserialize(&mut **self).map(Some),
            _ => Err(StructPathError::InvalidDeserializerState{
                expected: "MapKey".to_string(),
                got: self.state.clone(),
            }),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>  where V: serde::de::DeserializeSeed<'de> {
        let value = match &self.state {
            DeserializerState::MapKey(key) => match self.generic_parsed_path.remove(key) {
                Some(value) => value.clone(),
                None => return Err(StructPathError::Impossible),
            },
            _ => return Err(StructPathError::InvalidDeserializerState{
                expected: "MapValue".to_string(),
                got: self.state.clone(),
            }),
        };
        self.state = DeserializerState::MapValue(value.clone());
        seed.deserialize(&mut **self)
    }
}

/// Parse a particular path using a `Schema`
///
/// Typical errors will include when the Schema doesn't match T's structure.
pub fn parse_path<'a, S, T>(path: S, schema: &Schema) -> Result<T, StructPathError> where S: Into<String>, T: serde::Deserialize<'a> {
    let generic_parsed_path_value = parse_path_generic(path.into(), schema)?;
    let mut deserializer = Deserializer{
        generic_parsed_path: generic_parsed_path_value,
        state: DeserializerState::Start,
    };
    T::deserialize(&mut deserializer)
}

/// Internal state used by the Serializer, typically only used for debugging.
#[derive(Debug, Clone)]
pub enum SerializerState {
    Start, // starting, expecting a struct
    StructKey,  // in a struct, about to parse next key
    StructValue(String),  // about to serialize a struct value, this holds the key
    End,  // ending, not expecting any other states
}

struct Serializer{
    serialized_values: HashMap<String, String>,
    state: SerializerState,
}

impl<'a> serde::ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, _v: bool) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("bool".to_owned()))
    }

    fn serialize_i8(self, v: i8) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_string());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("char".to_owned()))
    }

    fn serialize_str(self, v: &str) -> Result<(), StructPathError> {
        self.state = match &self.state {
            SerializerState::StructValue(key) => {
                self.serialized_values.insert(key.clone(), v.to_owned());
                SerializerState::StructKey
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructValue".to_owned(),
                got: self.state.clone(),
            }),
        };
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("bytes".to_owned()))
    }

    fn serialize_none(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("None".to_owned()))
    }

    fn serialize_some<T>(self, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("Some".to_owned()))
    }

    fn serialize_unit(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("unit".to_owned()))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("unit struct".to_owned()))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("unit variant".to_owned()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
        ) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize, {
        Err(StructPathError::NotSupported("newtype struct".to_owned()))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
        ) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize, {
        Err(StructPathError::NotSupported("newtype variant".to_owned()))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, StructPathError> {
        Err(StructPathError::NotSupported("sequence".to_owned()))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, StructPathError> {
        Err(StructPathError::NotSupported("tuple".to_owned()))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
        ) -> Result<Self::SerializeTupleStruct, StructPathError> {
        Err(StructPathError::NotSupported("tuple struct".to_owned()))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
        ) -> Result<Self::SerializeTupleVariant, StructPathError> {
        Err(StructPathError::NotSupported("tuple variant".to_owned()))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, StructPathError> {
        // TODO should probaby support this
        Err(StructPathError::NotSupported("map".to_owned()))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
        ) -> Result<Self::SerializeStruct, StructPathError> {
        self.state = SerializerState::StructKey;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
        ) -> Result<Self::SerializeStructVariant, StructPathError> {
        Err(StructPathError::NotSupported("struct variant".to_owned()))
    }

}

impl<'a> serde::ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_element<T>(&mut self, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("sequence".to_owned()))
    }

    fn end(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("sequence".to_owned()))
    }
}

impl<'a> serde::ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_element<T>(&mut self, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("tuple".to_owned()))
    }

    fn end(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("tuple".to_owned()))
    }
}

impl<'a> serde::ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("tuple struct".to_owned()))
    }

    fn end(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("tuple struct".to_owned()))
    }
}

impl<'a> serde::ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("tuple variant".to_owned()))
    }

    fn end(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("tuple variant".to_owned()))
    }
}

impl<'a> serde::ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize, {
        Err(StructPathError::NotSupported("map".to_owned()))
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("map".to_owned()))
    }

    fn end(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("map".to_owned()))
    }
}

impl<'a> serde::ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        self.state = match self.state {
            SerializerState::StructKey => {
                SerializerState::StructValue(key.to_owned())
            },
            _ => return Err(StructPathError::InvalidSerializerState{
                expected: "StructKey".to_owned(),
                got: self.state.clone(),
            }),
        };
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<(), StructPathError> {
        self.state = SerializerState::End;
        Ok(())
    }
}

impl<'a> serde::ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = StructPathError;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<(), StructPathError> where T: ?Sized + serde::Serialize {
        Err(StructPathError::NotSupported("struct variant".to_owned()))
    }

    fn end(self) -> Result<(), StructPathError> {
        Err(StructPathError::NotSupported("struct variant".to_owned()))
    }
}

/// Generate a string url path given parameters and a `Schema`
pub fn generate_path<T>(parameters: &T, schema: &Schema) -> Result<String, StructPathError> where T: serde::Serialize {
    let mut serializer = Serializer{
        serialized_values: HashMap::new(),
        state: SerializerState::Start,
    };
    parameters.serialize(&mut serializer)?;
    let mut generated_path = String::new();
    for segment_schema in &schema.segments {
        match segment_schema {
            SegmentSchema::Literal(literal) => generated_path = format!("{}/{}", generated_path, literal),
            SegmentSchema::Value(segment_value_schema) => match serializer.serialized_values.get(&segment_value_schema.name) {
                Some(value) => generated_path = format!("{}/{}", generated_path, value),
                None => return Err(StructPathError::MissingField(segment_value_schema.name.clone())),
            }
        }
    }
    Ok(generated_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[test]
    fn test_parse_path_generic() {
        assert_eq!(
            parse_path_generic(
                "/foo/1/bar/thing".to_owned(),
                &Schema{
                    segments: vec![
                        SegmentSchema::Literal("foo".to_owned()),
                        SegmentSchema::Value(SegmentValueSchema{
                            name: "foo".to_owned(),
                            segment_type: SegmentType::U64,
                        }),
                        SegmentSchema::Literal("bar".to_owned()),
                        SegmentSchema::Value(SegmentValueSchema{
                            name: "bar".to_owned(),
                            segment_type: SegmentType::String,
                        }),
                    ],
                }
            ).unwrap(),
            {
                let mut map = HashMap::new();
                map.insert("foo".to_owned(), SegmentValue::U64(1));
                map.insert("bar".to_owned(), SegmentValue::String("thing".to_owned()));
                map
            },
            );
    }

    #[test]
    fn test_parse_path_generic_float() {
        assert_eq!(
            parse_path_generic(
                "/foo/1.2".to_owned(),
                &Schema{
                    segments: vec![
                        SegmentSchema::Literal("foo".to_owned()),
                        SegmentSchema::Value(SegmentValueSchema{
                            name: "foo".to_owned(),
                            segment_type: SegmentType::F64,
                        }),
                    ],
                },
                ).unwrap(),
            {
                let mut map = HashMap::new();
                map.insert("foo".to_owned(), SegmentValue::F64(1.2));
                map
            },
            );
    }

    #[test]
    fn test_parse_path_generic_signed_integer() {
        assert_eq!(
            parse_path_generic(
                "/foo/-1".to_owned(),
                &Schema{
                    segments: vec![
                        SegmentSchema::Literal("foo".to_owned()),
                        SegmentSchema::Value(SegmentValueSchema{
                            name: "foo".to_owned(),
                            segment_type: SegmentType::I128,
                        }),
                    ],
                },
                ).unwrap(),
            {
                let mut map = HashMap::new();
                map.insert("foo".to_owned(), SegmentValue::I128(-1));
                map
            },
            );
    }

    #[test]
    fn test_schema_building() {
        let schema = Schema::new()
            .literal("foo")
            .value("foo", SegmentType::U64)
            .literal("bar")
            .value("bar", SegmentType::String);
        assert_eq!(
            schema,
                Schema{
                    segments: vec![
                        SegmentSchema::Literal("foo".to_owned()),
                        SegmentSchema::Value(SegmentValueSchema{
                            name: "foo".to_owned(),
                            segment_type: SegmentType::U64,
                        }),
                        SegmentSchema::Literal("bar".to_owned()),
                        SegmentSchema::Value(SegmentValueSchema{
                            name: "bar".to_owned(),
                            segment_type: SegmentType::String,
                        }),
                    ],
                },
            );
    }

    #[test]
    fn test_schema_path() {
        assert_eq!(
            Schema::path("/foo/<foo_id:u128>/bar/<bar_thing:String>").unwrap(),
            Schema{
                segments: vec![
                    SegmentSchema::Literal("foo".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "foo_id".to_owned(),
                        segment_type: SegmentType::U128,
                    }),
                    SegmentSchema::Literal("bar".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "bar_thing".to_owned(),
                        segment_type: SegmentType::String,
                    }),
                ],
            }
            );
    }

    #[test]
    fn test_schema_path_string_default() {
        assert_eq!(
            Schema::path("/foo/<bar>").unwrap(),
            Schema{
                segments: vec![
                    SegmentSchema::Literal("foo".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "bar".to_owned(),
                        segment_type: SegmentType::String,
                    }),
                ],
            }
            );
    }


    #[test]
    fn test_parse_path_basic() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Value{
            foo: u64,
            bar: String,
        }

        let value: Value = parse_path(
            "/foo/1/bar/thing".to_owned(),
            &Schema{
                segments: vec![
                    SegmentSchema::Literal("foo".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "foo".to_owned(),
                        segment_type: SegmentType::U64,
                    }),
                    SegmentSchema::Literal("bar".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "bar".to_owned(),
                        segment_type: SegmentType::String,
                    }),
                ],
            }
        ).unwrap();
        assert_eq!(value, Value{foo: 1, bar: "thing".to_owned()});
    }

    #[test]
    fn test_parse_path_i64() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Value{
            foo: i128
        }

        let value: Value = parse_path(
            "/foo/-1".to_owned(),
            &Schema{
                segments: vec![
                    SegmentSchema::Literal("foo".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "foo".to_owned(),
                        segment_type: SegmentType::I128,
                    }),
                ],
            },
            ).unwrap();
        assert_eq!(value, Value{foo: -1});
    }

    #[test]
    fn test_parse_path_f64() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Value{
            foo: f64
        }

        let value: Value = parse_path(
            "/foo/1.2".to_owned(),
            &Schema{
                segments: vec![
                    SegmentSchema::Literal("foo".to_owned()),
                    SegmentSchema::Value(SegmentValueSchema{
                        name: "foo".to_owned(),
                        segment_type: SegmentType::F64,
                    }),
                ],
            },
            ).unwrap();
        assert_eq!(value, Value{foo: 1.2});
    }

    #[test]
    fn test_parse_path_idiomatic() {

        #[derive(Deserialize, PartialEq, Debug)]
        struct Parameters{
            foo: u64,
            bar: String,
        }

        let path_schema = Schema::path("/foo/<foo:u64>/bar/<bar>").unwrap();
        let parameters: Parameters = path_schema.parse("/foo/1/bar/thing").unwrap();
        assert_eq!(parameters, Parameters{foo: 1, bar: "thing".to_owned()});

    }

    #[test]
    fn test_generate_path() {
        #[derive(Serialize, PartialEq, Debug)]
        struct Parameters{
            foo: u64,
            bar: String,
        }

        let schema = Schema::path("/foo/<foo:u64>/bar/<bar>").unwrap();
        assert_eq!(schema.generate(&Parameters{foo: 1, bar: "thing".to_owned()}).unwrap(), "/foo/1/bar/thing");
    }

    #[test]
    fn test_roundtrip() {

        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Parameters{
            foo: u64,
            bar: String,
        }

        let test_path = "/foo/1/bar/thing";
        let path_schema = Schema::path("/foo/<foo:u64>/bar/<bar>").unwrap();
        let parameters: Parameters = path_schema.parse(test_path).unwrap();
        assert_eq!(parameters, Parameters{foo: 1, bar: "thing".to_owned()});
        assert_eq!(path_schema.generate(&parameters).unwrap(), test_path);

    }
}
