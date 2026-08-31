#![feature(structural_match, core_intrinsics, print_internals, fmt_helpers_for_derive)]
#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
/// 第五章/第六章 demo: 实现自定义 Deserializer
///
/// 运行: cargo run --example 05_custom_deserializer
///
/// 实现一个能将逗号分隔的字符串反序列化为各种 Rust 类型的 Deserializer
use serde::de::{
    self, DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::{Deserialize, Deserializer};
use std::fmt;
struct CsvError(String);
#[automatically_derived]
impl ::core::fmt::Debug for CsvError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_tuple_field1_finish(f, "CsvError", &&self.0)
    }
}
impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{0}", self.0))
    }
}
impl std::error::Error for CsvError {}
impl de::Error for CsvError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        CsvError(msg.to_string())
    }
}
struct CsvDeserializer<'de> {
    input: &'de str,
}
impl<'de> CsvDeserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self { input }
    }
    /// 解析下一个逗号字段,返回字段值 + 剩余部分
    fn next_field(&self) -> (&'de str, &'de str) {
        match self.input.find(',') {
            Some(pos) => (&self.input[..pos], &self.input[pos + 1..]),
            None => (self.input, ""),
        }
    }
}
impl<'de> Deserializer<'de> for CsvDeserializer<'de> {
    type Error = CsvError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let field = self.input.trim();
        if field == "true" {
            return visitor.visit_bool(true);
        }
        if field == "false" {
            return visitor.visit_bool(false);
        }
        if let Ok(v) = field.parse::<i64>() {
            return visitor.visit_i64(v);
        }
        if let Ok(v) = field.parse::<u64>() {
            return visitor.visit_u64(v);
        }
        if let Ok(v) = field.parse::<f64>() {
            return visitor.visit_f64(v);
        }
        visitor.visit_borrowed_str(field)
    }
    fn deserialize_bool<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        match self.input.trim() {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            other => {
                Err(Error::invalid_value(de::Unexpected::Str(other), &"true or false"))
            }
        }
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: i32 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i32"))?;
        visitor.visit_i32(v)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: i64 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i64"))?;
        visitor.visit_i64(v)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: u64 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u64"))?;
        visitor.visit_u64(v)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: f64 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"f64"))?;
        visitor.visit_f64(v)
    }
    fn deserialize_char<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        let s = self.input.trim();
        let mut chars = s.chars();
        match chars.next() {
            Some(c) if chars.next().is_none() => visitor.visit_char(c),
            _ => Err(Error::invalid_value(de::Unexpected::Str(s), &"a single character")),
        }
    }
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        visitor.visit_borrowed_str(self.input.trim())
    }
    fn deserialize_string<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_string(self.input.trim().to_owned())
    }
    fn deserialize_bytes<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_borrowed_bytes(self.input.trim().as_bytes())
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_byte_buf(self.input.trim().as_bytes().to_vec())
    }
    fn deserialize_option<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        match self.input.trim() {
            "" | "null" | "none" | "None" => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let fields: Vec<&str> = self.input.split(',').collect();
        visitor.visit_seq(CsvSeqAccess { fields, index: 0 })
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let pairs: Vec<(&str, &str)> = self
            .input
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                s.split_once('=').or_else(|| s.split_once(':'))
            })
            .collect();
        visitor.visit_map(CsvMapAccess { pairs, index: 0 })
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        let (variant, rest) = self.next_field();
        visitor
            .visit_enum(CsvEnumAccess {
                variant: variant.trim(),
                data: rest.trim(),
                _marker: std::marker::PhantomData,
            })
    }
    fn deserialize_identifier<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        self.deserialize_str(visitor)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_unit()
    }
    fn deserialize_i8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: i8 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i8"))?;
        v.visit_i8(n)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: i16 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i16"))?;
        v.visit_i16(n)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: u8 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u8"))?;
        v.visit_u8(n)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: u16 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u16"))?;
        v.visit_u16(n)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: u32 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u32"))?;
        v.visit_u32(n)
    }
    fn deserialize_f32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: f32 = self
            .input
            .trim()
            .parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"f32"))?;
        v.visit_f32(n)
    }
}
struct CsvSeqAccess<'de> {
    fields: Vec<&'de str>,
    index: usize,
}
impl<'de> SeqAccess<'de> for CsvSeqAccess<'de> {
    type Error = CsvError;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, CsvError> {
        if self.index >= self.fields.len() {
            return Ok(None);
        }
        let field = self.fields[self.index].trim();
        self.index += 1;
        seed.deserialize(CsvDeserializer::new(field)).map(Some)
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}
struct CsvMapAccess<'de> {
    pairs: Vec<(&'de str, &'de str)>,
    index: usize,
}
impl<'de> MapAccess<'de> for CsvMapAccess<'de> {
    type Error = CsvError;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, CsvError> {
        if self.index >= self.pairs.len() {
            return Ok(None);
        }
        let (key, _) = self.pairs[self.index];
        seed.deserialize(CsvDeserializer::new(key.trim())).map(Some)
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, CsvError> {
        let (_, value) = self.pairs[self.index];
        self.index += 1;
        seed.deserialize(CsvDeserializer::new(value.trim()))
    }
}
struct CsvEnumAccess<'de> {
    variant: &'de str,
    data: &'de str,
    _marker: std::marker::PhantomData<&'de ()>,
}
impl<'de> EnumAccess<'de> for CsvEnumAccess<'de> {
    type Error = CsvError;
    type Variant = CsvVariantAccess<'de>;
    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), CsvError> {
        let variant = seed.deserialize(CsvDeserializer::new(self.variant))?;
        Ok((
            variant,
            CsvVariantAccess {
                data: self.data,
            },
        ))
    }
}
struct CsvVariantAccess<'de> {
    data: &'de str,
}
impl<'de> VariantAccess<'de> for CsvVariantAccess<'de> {
    type Error = CsvError;
    fn unit_variant(self) -> Result<(), CsvError> {
        Ok(())
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, CsvError> {
        seed.deserialize(CsvDeserializer::new(self.data))
    }
    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        CsvDeserializer::new(self.data).deserialize_seq(visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, CsvError> {
        CsvDeserializer::new(self.data).deserialize_seq(visitor)
    }
}
fn from_csv<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T, CsvError> {
    T::deserialize(CsvDeserializer::new(s))
}
struct User {
    name: String,
    age: u8,
}
#[automatically_derived]
impl ::core::fmt::Debug for User {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "User",
            "name",
            &self.name,
            "age",
            &&self.age,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for User {}
#[automatically_derived]
impl ::core::cmp::PartialEq for User {
    #[inline]
    fn eq(&self, other: &User) -> bool {
        self.age == other.age && self.name == other.name
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for User {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private229::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private229::Formatter,
                ) -> _serde::__private229::fmt::Result {
                    _serde::__private229::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private229::Ok(__Field::__field0),
                        1u64 => _serde::__private229::Ok(__Field::__field1),
                        _ => _serde::__private229::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "name" => _serde::__private229::Ok(__Field::__field0),
                        "age" => _serde::__private229::Ok(__Field::__field1),
                        _ => _serde::__private229::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"name" => _serde::__private229::Ok(__Field::__field0),
                        b"age" => _serde::__private229::Ok(__Field::__field1),
                        _ => _serde::__private229::Ok(__Field::__ignore),
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private229::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private229::PhantomData<User>,
                lifetime: _serde::__private229::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = User;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private229::Formatter,
                ) -> _serde::__private229::fmt::Result {
                    _serde::__private229::Formatter::write_str(
                        __formatter,
                        "struct User",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private229::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match _serde::de::SeqAccess::next_element::<
                        String,
                    >(&mut __seq)? {
                        _serde::__private229::Some(__value) => __value,
                        _serde::__private229::None => {
                            return _serde::__private229::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct User with 2 elements",
                                ),
                            );
                        }
                    };
                    let __field1 = match _serde::de::SeqAccess::next_element::<
                        u8,
                    >(&mut __seq)? {
                        _serde::__private229::Some(__value) => __value,
                        _serde::__private229::None => {
                            return _serde::__private229::Err(
                                _serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct User with 2 elements",
                                ),
                            );
                        }
                    };
                    _serde::__private229::Ok(User {
                        name: __field0,
                        age: __field1,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private229::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private229::Option<String> = _serde::__private229::None;
                    let mut __field1: _serde::__private229::Option<u8> = _serde::__private229::None;
                    while let _serde::__private229::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private229::Option::is_some(&__field0) {
                                    return _serde::__private229::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("name"),
                                    );
                                }
                                __field0 = _serde::__private229::Some(
                                    _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private229::Option::is_some(&__field1) {
                                    return _serde::__private229::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("age"),
                                    );
                                }
                                __field1 = _serde::__private229::Some(
                                    _serde::de::MapAccess::next_value::<u8>(&mut __map)?,
                                );
                            }
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private229::Some(__field0) => __field0,
                        _serde::__private229::None => {
                            _serde::__private229::de::missing_field("name")?
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private229::Some(__field1) => __field1,
                        _serde::__private229::None => {
                            _serde::__private229::de::missing_field("age")?
                        }
                    };
                    _serde::__private229::Ok(User {
                        name: __field0,
                        age: __field1,
                    })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &["name", "age"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "User",
                FIELDS,
                __Visitor {
                    marker: _serde::__private229::PhantomData::<User>,
                    lifetime: _serde::__private229::PhantomData,
                },
            )
        }
    }
};
enum Status {
    Active,
    Inactive,
    Banned,
}
#[automatically_derived]
impl ::core::fmt::Debug for Status {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                Status::Active => "Active",
                Status::Inactive => "Inactive",
                Status::Banned => "Banned",
            },
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Status {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Status {
    #[inline]
    fn eq(&self, other: &Status) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Status {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private229::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __field2,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private229::Formatter,
                ) -> _serde::__private229::fmt::Result {
                    _serde::__private229::Formatter::write_str(
                        __formatter,
                        "variant identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private229::Ok(__Field::__field0),
                        1u64 => _serde::__private229::Ok(__Field::__field1),
                        2u64 => _serde::__private229::Ok(__Field::__field2),
                        _ => {
                            _serde::__private229::Err(
                                _serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 3",
                                ),
                            )
                        }
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "Active" => _serde::__private229::Ok(__Field::__field0),
                        "Inactive" => _serde::__private229::Ok(__Field::__field1),
                        "Banned" => _serde::__private229::Ok(__Field::__field2),
                        _ => {
                            _serde::__private229::Err(
                                _serde::de::Error::unknown_variant(__value, VARIANTS),
                            )
                        }
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"Active" => _serde::__private229::Ok(__Field::__field0),
                        b"Inactive" => _serde::__private229::Ok(__Field::__field1),
                        b"Banned" => _serde::__private229::Ok(__Field::__field2),
                        _ => {
                            let __value = &_serde::__private229::from_utf8_lossy(
                                __value,
                            );
                            _serde::__private229::Err(
                                _serde::de::Error::unknown_variant(__value, VARIANTS),
                            )
                        }
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private229::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private229::PhantomData<Status>,
                lifetime: _serde::__private229::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = Status;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private229::Formatter,
                ) -> _serde::__private229::fmt::Result {
                    _serde::__private229::Formatter::write_str(
                        __formatter,
                        "enum Status",
                    )
                }
                fn visit_enum<__A>(
                    self,
                    __data: __A,
                ) -> _serde::__private229::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::EnumAccess<'de>,
                {
                    match _serde::de::EnumAccess::variant(__data) {
                        _serde::__private229::Ok((__Field::__field0, __variant)) => {
                            _serde::de::VariantAccess::unit_variant(__variant)?;
                            _serde::__private229::Ok(Status::Active)
                        }
                        _serde::__private229::Ok((__Field::__field1, __variant)) => {
                            _serde::de::VariantAccess::unit_variant(__variant)?;
                            _serde::__private229::Ok(Status::Inactive)
                        }
                        _serde::__private229::Ok((__Field::__field2, __variant)) => {
                            _serde::de::VariantAccess::unit_variant(__variant)?;
                            _serde::__private229::Ok(Status::Banned)
                        }
                        _serde::__private229::Err(__err) => {
                            _serde::__private229::Err(__err)
                        }
                    }
                }
            }
            #[doc(hidden)]
            const VARIANTS: &'static [&'static str] = &["Active", "Inactive", "Banned"];
            _serde::Deserializer::deserialize_enum(
                __deserializer,
                "Status",
                VARIANTS,
                __Visitor {
                    marker: _serde::__private229::PhantomData::<Status>,
                    lifetime: _serde::__private229::PhantomData,
                },
            )
        }
    }
};
enum Command {
    Move { x: i32, y: i32 },
    Say(String),
    Quit,
}
#[automatically_derived]
impl ::core::fmt::Debug for Command {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Command::Move { x: __self_0, y: __self_1 } => {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Move",
                    "x",
                    __self_0,
                    "y",
                    &__self_1,
                )
            }
            Command::Say(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Say", &__self_0)
            }
            Command::Quit => ::core::fmt::Formatter::write_str(f, "Quit"),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Command {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Command {
    #[inline]
    fn eq(&self, other: &Command) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (
                    Command::Move { x: __self_0, y: __self_1 },
                    Command::Move { x: __arg1_0, y: __arg1_1 },
                ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                (Command::Say(__self_0), Command::Say(__arg1_0)) => __self_0 == __arg1_0,
                _ => true,
            }
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Command {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private229::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __field2,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private229::Formatter,
                ) -> _serde::__private229::fmt::Result {
                    _serde::__private229::Formatter::write_str(
                        __formatter,
                        "variant identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private229::Ok(__Field::__field0),
                        1u64 => _serde::__private229::Ok(__Field::__field1),
                        2u64 => _serde::__private229::Ok(__Field::__field2),
                        _ => {
                            _serde::__private229::Err(
                                _serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 3",
                                ),
                            )
                        }
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "Move" => _serde::__private229::Ok(__Field::__field0),
                        "Say" => _serde::__private229::Ok(__Field::__field1),
                        "Quit" => _serde::__private229::Ok(__Field::__field2),
                        _ => {
                            _serde::__private229::Err(
                                _serde::de::Error::unknown_variant(__value, VARIANTS),
                            )
                        }
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private229::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"Move" => _serde::__private229::Ok(__Field::__field0),
                        b"Say" => _serde::__private229::Ok(__Field::__field1),
                        b"Quit" => _serde::__private229::Ok(__Field::__field2),
                        _ => {
                            let __value = &_serde::__private229::from_utf8_lossy(
                                __value,
                            );
                            _serde::__private229::Err(
                                _serde::de::Error::unknown_variant(__value, VARIANTS),
                            )
                        }
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private229::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private229::PhantomData<Command>,
                lifetime: _serde::__private229::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = Command;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private229::Formatter,
                ) -> _serde::__private229::fmt::Result {
                    _serde::__private229::Formatter::write_str(
                        __formatter,
                        "enum Command",
                    )
                }
                fn visit_enum<__A>(
                    self,
                    __data: __A,
                ) -> _serde::__private229::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::EnumAccess<'de>,
                {
                    match _serde::de::EnumAccess::variant(__data) {
                        _serde::__private229::Ok((__Field::__field0, __variant)) => {
                            #[allow(non_camel_case_types)]
                            #[doc(hidden)]
                            enum __Field {
                                __field0,
                                __field1,
                                __ignore,
                            }
                            #[doc(hidden)]
                            struct __FieldVisitor;
                            #[automatically_derived]
                            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                                type Value = __Field;
                                fn expecting(
                                    &self,
                                    __formatter: &mut _serde::__private229::Formatter,
                                ) -> _serde::__private229::fmt::Result {
                                    _serde::__private229::Formatter::write_str(
                                        __formatter,
                                        "field identifier",
                                    )
                                }
                                fn visit_u64<__E>(
                                    self,
                                    __value: u64,
                                ) -> _serde::__private229::Result<Self::Value, __E>
                                where
                                    __E: _serde::de::Error,
                                {
                                    match __value {
                                        0u64 => _serde::__private229::Ok(__Field::__field0),
                                        1u64 => _serde::__private229::Ok(__Field::__field1),
                                        _ => _serde::__private229::Ok(__Field::__ignore),
                                    }
                                }
                                fn visit_str<__E>(
                                    self,
                                    __value: &str,
                                ) -> _serde::__private229::Result<Self::Value, __E>
                                where
                                    __E: _serde::de::Error,
                                {
                                    match __value {
                                        "x" => _serde::__private229::Ok(__Field::__field0),
                                        "y" => _serde::__private229::Ok(__Field::__field1),
                                        _ => _serde::__private229::Ok(__Field::__ignore),
                                    }
                                }
                                fn visit_bytes<__E>(
                                    self,
                                    __value: &[u8],
                                ) -> _serde::__private229::Result<Self::Value, __E>
                                where
                                    __E: _serde::de::Error,
                                {
                                    match __value {
                                        b"x" => _serde::__private229::Ok(__Field::__field0),
                                        b"y" => _serde::__private229::Ok(__Field::__field1),
                                        _ => _serde::__private229::Ok(__Field::__ignore),
                                    }
                                }
                            }
                            #[automatically_derived]
                            impl<'de> _serde::Deserialize<'de> for __Field {
                                #[inline]
                                fn deserialize<__D>(
                                    __deserializer: __D,
                                ) -> _serde::__private229::Result<Self, __D::Error>
                                where
                                    __D: _serde::Deserializer<'de>,
                                {
                                    _serde::Deserializer::deserialize_identifier(
                                        __deserializer,
                                        __FieldVisitor,
                                    )
                                }
                            }
                            #[doc(hidden)]
                            struct __Visitor<'de> {
                                marker: _serde::__private229::PhantomData<Command>,
                                lifetime: _serde::__private229::PhantomData<&'de ()>,
                            }
                            #[automatically_derived]
                            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                                type Value = Command;
                                fn expecting(
                                    &self,
                                    __formatter: &mut _serde::__private229::Formatter,
                                ) -> _serde::__private229::fmt::Result {
                                    _serde::__private229::Formatter::write_str(
                                        __formatter,
                                        "struct variant Command::Move",
                                    )
                                }
                                #[inline]
                                fn visit_seq<__A>(
                                    self,
                                    mut __seq: __A,
                                ) -> _serde::__private229::Result<Self::Value, __A::Error>
                                where
                                    __A: _serde::de::SeqAccess<'de>,
                                {
                                    let __field0 = match _serde::de::SeqAccess::next_element::<
                                        i32,
                                    >(&mut __seq)? {
                                        _serde::__private229::Some(__value) => __value,
                                        _serde::__private229::None => {
                                            return _serde::__private229::Err(
                                                _serde::de::Error::invalid_length(
                                                    0usize,
                                                    &"struct variant Command::Move with 2 elements",
                                                ),
                                            );
                                        }
                                    };
                                    let __field1 = match _serde::de::SeqAccess::next_element::<
                                        i32,
                                    >(&mut __seq)? {
                                        _serde::__private229::Some(__value) => __value,
                                        _serde::__private229::None => {
                                            return _serde::__private229::Err(
                                                _serde::de::Error::invalid_length(
                                                    1usize,
                                                    &"struct variant Command::Move with 2 elements",
                                                ),
                                            );
                                        }
                                    };
                                    _serde::__private229::Ok(Command::Move {
                                        x: __field0,
                                        y: __field1,
                                    })
                                }
                                #[inline]
                                fn visit_map<__A>(
                                    self,
                                    mut __map: __A,
                                ) -> _serde::__private229::Result<Self::Value, __A::Error>
                                where
                                    __A: _serde::de::MapAccess<'de>,
                                {
                                    let mut __field0: _serde::__private229::Option<i32> = _serde::__private229::None;
                                    let mut __field1: _serde::__private229::Option<i32> = _serde::__private229::None;
                                    while let _serde::__private229::Some(__key) = _serde::de::MapAccess::next_key::<
                                        __Field,
                                    >(&mut __map)? {
                                        match __key {
                                            __Field::__field0 => {
                                                if _serde::__private229::Option::is_some(&__field0) {
                                                    return _serde::__private229::Err(
                                                        <__A::Error as _serde::de::Error>::duplicate_field("x"),
                                                    );
                                                }
                                                __field0 = _serde::__private229::Some(
                                                    _serde::de::MapAccess::next_value::<i32>(&mut __map)?,
                                                );
                                            }
                                            __Field::__field1 => {
                                                if _serde::__private229::Option::is_some(&__field1) {
                                                    return _serde::__private229::Err(
                                                        <__A::Error as _serde::de::Error>::duplicate_field("y"),
                                                    );
                                                }
                                                __field1 = _serde::__private229::Some(
                                                    _serde::de::MapAccess::next_value::<i32>(&mut __map)?,
                                                );
                                            }
                                            _ => {
                                                let _ = _serde::de::MapAccess::next_value::<
                                                    _serde::de::IgnoredAny,
                                                >(&mut __map)?;
                                            }
                                        }
                                    }
                                    let __field0 = match __field0 {
                                        _serde::__private229::Some(__field0) => __field0,
                                        _serde::__private229::None => {
                                            _serde::__private229::de::missing_field("x")?
                                        }
                                    };
                                    let __field1 = match __field1 {
                                        _serde::__private229::Some(__field1) => __field1,
                                        _serde::__private229::None => {
                                            _serde::__private229::de::missing_field("y")?
                                        }
                                    };
                                    _serde::__private229::Ok(Command::Move {
                                        x: __field0,
                                        y: __field1,
                                    })
                                }
                            }
                            #[doc(hidden)]
                            const FIELDS: &'static [&'static str] = &["x", "y"];
                            _serde::de::VariantAccess::struct_variant(
                                __variant,
                                FIELDS,
                                __Visitor {
                                    marker: _serde::__private229::PhantomData::<Command>,
                                    lifetime: _serde::__private229::PhantomData,
                                },
                            )
                        }
                        _serde::__private229::Ok((__Field::__field1, __variant)) => {
                            _serde::__private229::Result::map(
                                _serde::de::VariantAccess::newtype_variant::<
                                    String,
                                >(__variant),
                                Command::Say,
                            )
                        }
                        _serde::__private229::Ok((__Field::__field2, __variant)) => {
                            _serde::de::VariantAccess::unit_variant(__variant)?;
                            _serde::__private229::Ok(Command::Quit)
                        }
                        _serde::__private229::Err(__err) => {
                            _serde::__private229::Err(__err)
                        }
                    }
                }
            }
            #[doc(hidden)]
            const VARIANTS: &'static [&'static str] = &["Move", "Say", "Quit"];
            _serde::Deserializer::deserialize_enum(
                __deserializer,
                "Command",
                VARIANTS,
                __Visitor {
                    marker: _serde::__private229::PhantomData::<Command>,
                    lifetime: _serde::__private229::PhantomData,
                },
            )
        }
    }
};
fn main() -> Result<(), CsvError> {
    {
        ::std::io::_print(format_args!("=== Custom CsvDeserializer Demo ===\n\n"));
    };
    {
        ::std::io::_print(format_args!("--- Primitives ---\n"));
    };
    let v: i32 = from_csv("42")?;
    {
        ::std::io::_print(format_args!("  i32: {0}  ←  \'42\'\n", v));
    };
    let v: bool = from_csv("true")?;
    {
        ::std::io::_print(format_args!("  bool: {0}  ←  \'true\'\n", v));
    };
    let v: String = from_csv("hello world")?;
    {
        ::std::io::_print(format_args!("  str:  \'{0}\'\n", v));
    };
    let v: f64 = from_csv("3.14")?;
    {
        ::std::io::_print(format_args!("  f64:  {0}\n", v));
    };
    {
        ::std::io::_print(format_args!("\n--- Option ---\n"));
    };
    let v: Option<i32> = from_csv("42")?;
    {
        ::std::io::_print(format_args!("  Some: {0:?}\n", v));
    };
    let v: Option<i32> = from_csv("")?;
    {
        ::std::io::_print(format_args!("  None: {0:?}\n", v));
    };
    let v: Option<i32> = from_csv("null")?;
    {
        ::std::io::_print(format_args!("  Null: {0:?}\n", v));
    };
    {
        ::std::io::_print(format_args!("\n--- Vec ---\n"));
    };
    let v: Vec<i32> = from_csv("1,2,3,4,5")?;
    {
        ::std::io::_print(format_args!("  Vec: {0:?}\n", v));
    };
    {
        ::std::io::_print(format_args!("\n--- Struct (positional) ---\n"));
    };
    let user: User = from_csv("Alice,30")?;
    {
        ::std::io::_print(format_args!("  User: {0:?}\n", user));
    };
    {
        ::std::io::_print(format_args!("\n--- Enum ---\n"));
    };
    let st: Status = from_csv("Active")?;
    {
        ::std::io::_print(format_args!("  Status: {0:?}\n", st));
    };
    let st: Status = from_csv("Banned")?;
    {
        ::std::io::_print(format_args!("  Status: {0:?}\n", st));
    };
    {
        ::std::io::_print(format_args!("\n--- Enum with data ---\n"));
    };
    let cmd: Command = from_csv("Say,Hello World")?;
    {
        ::std::io::_print(format_args!("  Command: {0:?}\n", cmd));
    };
    let cmd: Command = from_csv("Move,10,20")?;
    {
        ::std::io::_print(format_args!("  Command: {0:?}\n", cmd));
    };
    let cmd: Command = from_csv("Quit")?;
    {
        ::std::io::_print(format_args!("  Command: {0:?}\n", cmd));
    };
    {
        ::std::io::_print(format_args!("\n--- Map format (key=val) ---\n"));
    };
    let map_str = "name=Bob,age=25";
    {
        ::std::io::_print(format_args!("  Input: \'{0}\'\n", map_str));
    };
    {
        ::std::io::_print(
            format_args!(
                "  Use CsvDeserializer::deserialize_map for key-value parsing\n",
            ),
        );
    };
    {
        ::std::io::_print(format_args!("\n=== All CsvDeserializer demos passed! ===\n"));
    };
    Ok(())
}
