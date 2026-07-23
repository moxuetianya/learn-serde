/// 综合 demo: 用自定义 Serializer + Deserializer 完成 roundtrip
///
/// 运行: cargo run --example 07_roundtrip
///
/// 实现一个简单的键值对文本格式:
///   key1=value1;key2=value2;key3=value3
///
/// 然后对一个 struct 做 serialization → deserialization roundtrip

use serde::de::{self, DeserializeSeed, Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// 复用 serde 自带的 forward_to_deserialize_any! 宏
use serde::forward_to_deserialize_any;
use std::fmt;

// ============================================================
// 格式定义
// ============================================================
// 序列化: key1=value1;key2=value2;...
// 反序列化: 同上
//
// 每个值使用 JSON 格式的子集来序列化:
//   bool → true/false
//   int  → 123
//   str  → "string"  (如果包含 ; 或 = 则用引号包裹)

// ============================================================
// 序列化侧
// ============================================================
#[derive(Debug)]
struct KvpError(String);

impl fmt::Display for KvpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

impl std::error::Error for KvpError {}

impl serde::ser::Error for KvpError {
    fn custom<T: fmt::Display>(msg: T) -> Self { KvpError(msg.to_string()) }
}

impl de::Error for KvpError {
    fn custom<T: fmt::Display>(msg: T) -> Self { KvpError(msg.to_string()) }
}

struct KvpSerializer {
    output: String,
    first: bool,
}

impl KvpSerializer {
    fn new() -> Self {
        Self { output: String::new(), first: true }
    }
    fn into_string(self) -> String { self.output }
}

struct KvpStructSerializer<'a> {
    ser: &'a mut KvpSerializer,
}

impl<'a> Serializer for &'a mut KvpSerializer {
    type Ok = ();
    type Error = KvpError;

    type SerializeSeq = serde::ser::Impossible<(), KvpError>;
    type SerializeTuple = serde::ser::Impossible<(), KvpError>;
    type SerializeTupleStruct = serde::ser::Impossible<(), KvpError>;
    type SerializeTupleVariant = serde::ser::Impossible<(), KvpError>;
    type SerializeMap = serde::ser::Impossible<(), KvpError>;
    type SerializeStruct = KvpStructSerializer<'a>;
    type SerializeStructVariant = serde::ser::Impossible<(), KvpError>;

    fn serialize_bool(self, v: bool) -> Result<(), KvpError> {
        self.output.push_str(if v { "true" } else { "false" });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<(), KvpError> {
        self.output.push_str(&format!("{}", v));
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<(), KvpError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<(), KvpError> {
        if v.contains(';') || v.contains('=') || v.is_empty() {
            self.output.push('"');
            self.output.push_str(v);
            self.output.push('"');
        } else {
            self.output.push_str(v);
        }
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), KvpError> {
        // 格式为 [byte1,byte2,...]
        self.output.push('[');
        for (i, byte) in v.iter().enumerate() {
            if i > 0 { self.output.push(','); }
            self.output.push_str(&format!("{}", byte));
        }
        self.output.push(']');
        Ok(())
    }
    fn serialize_none(self) -> Result<(), KvpError> { Ok(()) }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), KvpError> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), KvpError> { Ok(()) }
    fn serialize_unit_struct(self, _n: &'static str) -> Result<(), KvpError> { Ok(()) }
    fn serialize_unit_variant(self, _n: &'static str, _i: u32, _v: &'static str) -> Result<(), KvpError> { Ok(()) }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _n: &'static str, v: &T) -> Result<(), KvpError> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(self, _n: &'static str, _i: u32, _v: &'static str, value: &T) -> Result<(), KvpError> {
        value.serialize(self)
    }
    fn serialize_seq(self, _l: Option<usize>) -> Result<serde::ser::Impossible<(), KvpError>, KvpError> {
        Err(serde::ser::Error::custom("seq not supported in kvp format"))
    }
    fn serialize_tuple(self, _l: usize) -> Result<serde::ser::Impossible<(), KvpError>, KvpError> {
        Err(serde::ser::Error::custom("tuple not supported"))
    }
    fn serialize_tuple_struct(self, _n: &'static str, _l: usize) -> Result<serde::ser::Impossible<(), KvpError>, KvpError> {
        Err(serde::ser::Error::custom("tuple_struct not supported"))
    }
    fn serialize_tuple_variant(self, _n: &'static str, _i: u32, _v: &'static str, _l: usize) -> Result<serde::ser::Impossible<(), KvpError>, KvpError> {
        Err(serde::ser::Error::custom("tuple_variant not supported"))
    }
    fn serialize_map(self, _l: Option<usize>) -> Result<serde::ser::Impossible<(), KvpError>, KvpError> {
        Err(serde::ser::Error::custom("map not supported"))
    }
    fn serialize_struct(self, _n: &'static str, _l: usize) -> Result<KvpStructSerializer<'a>, KvpError> {
        Ok(KvpStructSerializer { ser: self })
    }
    fn serialize_struct_variant(self, _n: &'static str, _i: u32, _v: &'static str, _l: usize) -> Result<serde::ser::Impossible<(), KvpError>, KvpError> {
        Err(serde::ser::Error::custom("struct_variant not supported"))
    }
}

impl<'a> serde::ser::SerializeStruct for KvpStructSerializer<'a> {
    type Ok = ();
    type Error = KvpError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &str, value: &T) -> Result<(), KvpError> {
        if !self.ser.first {
            self.ser.output.push(';');
        }
        self.ser.first = false;
        self.ser.output.push_str(key);
        self.ser.output.push('=');
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), KvpError> {
        Ok(())
    }
}

fn to_kvp<T: Serialize>(value: &T) -> Result<String, KvpError> {
    let mut ser = KvpSerializer::new();
    value.serialize(&mut ser)?;
    Ok(ser.into_string())
}

// ============================================================
// 反序列化侧
// ============================================================
struct KvpDeserializer<'de> {
    input: &'de str,
    pos: usize,
}

impl<'de> KvpDeserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_key_value(&mut self) -> Option<(&'de str, &'de str)> {
        let remaining = &self.input[self.pos..];
        if remaining.is_empty() { return None; }

        if let Some(eq_pos) = remaining.find('=') {
            let key = remaining[..eq_pos].trim();
            let after_eq = &remaining[eq_pos + 1..];

            let (val, next_pos) = if after_eq.starts_with('"') {
                // quoted value
                if let Some(end_quote) = after_eq[1..].find('"') {
                    (&after_eq[1..end_quote + 1], end_quote + 2)
                } else {
                    (&after_eq[1..], after_eq.len())
                }
            } else if let Some(semi_pos) = after_eq.find(';') {
                (&after_eq[..semi_pos], semi_pos + 1)
            } else {
                (after_eq, after_eq.len())
            };

            self.pos += eq_pos + 1 + next_pos;
            Some((key, val.trim()))
        } else {
            None
        }
    }
}

struct KvpMapAccess<'de> {
    de: &'de str,
    pos: usize,
    value: Option<(&'de str, &'de str)>,
}

impl<'de> MapAccess<'de> for KvpMapAccess<'de> {
    type Error = KvpError;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, KvpError> {
        let remaining = &self.de[self.pos..];
        if remaining.is_empty() { return Ok(None); }

        if let Some(eq_pos) = remaining.find('=') {
            let key = remaining[..eq_pos].trim();
            let after_eq = &remaining[eq_pos + 1..];

            let (val, next_pos) = if after_eq.starts_with('"') {
                if let Some(end_quote) = after_eq[1..].find('"') {
                    (&after_eq[1..end_quote + 1], end_quote + 2)
                } else {
                    (&after_eq[1..], after_eq.len())
                }
            } else if let Some(semi_pos) = after_eq.find(';') {
                (&after_eq[..semi_pos], semi_pos + 1)
            } else {
                (after_eq, after_eq.len())
            };

            self.pos += eq_pos + 1 + next_pos;
            self.value = Some((key, val.trim()));
            seed.deserialize(ValueDeserializer { value: key }).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, KvpError> {
        let (_, val) = self.value.take().unwrap();
        seed.deserialize(ValueDeserializer { value: val })
    }
}

struct ValueDeserializer<'de> {
    value: &'de str,
}

impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = KvpError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, KvpError> {
        if self.value == "true" { return visitor.visit_bool(true); }
        if self.value == "false" { return visitor.visit_bool(false); }
        if let Ok(v) = self.value.parse::<i64>() { return visitor.visit_i64(v); }
        if let Ok(v) = self.value.parse::<u64>() { return visitor.visit_u64(v); }
        if let Ok(v) = self.value.parse::<f64>() { return visitor.visit_f64(v); }
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        match self.value {
            "true" => v.visit_bool(true),
            "false" => v.visit_bool(false),
            _ => Err(Error::invalid_value(de::Unexpected::Str(self.value), &"true/false")),
        }
    }

    fn deserialize_i32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        let n: i32 = self.value.parse().map_err(|_| Error::invalid_value(de::Unexpected::Str(self.value), &"i32"))?;
        v.visit_i32(n)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        let n: i64 = self.value.parse().map_err(|_| Error::invalid_value(de::Unexpected::Str(self.value), &"i64"))?;
        v.visit_i64(n)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        let n: u64 = self.value.parse().map_err(|_| Error::invalid_value(de::Unexpected::Str(self.value), &"u64"))?;
        v.visit_u64(n)
    }

    fn deserialize_str<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        v.visit_borrowed_str(self.value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        v.visit_string(self.value.to_owned())
    }

    fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        if self.value.is_empty() { v.visit_none() } else { v.visit_some(self) }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        v.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, v: V) -> Result<V::Value, KvpError> {
        v.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, v: V) -> Result<V::Value, KvpError> {
        v.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        Err(Error::custom("seq not supported"))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, v: V) -> Result<V::Value, KvpError> {
        Err(Error::custom("tuple not supported"))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _: &'static str, _: usize, v: V) -> Result<V::Value, KvpError> {
        Err(Error::custom("tuple_struct not supported"))
    }

    fn deserialize_map<V: Visitor<'de>>(self, _: V) -> Result<V::Value, KvpError> {
        Err(Error::custom("map not supported"))
    }

    fn deserialize_struct<V: Visitor<'de>>(self, _: &'static str, _: &'static [&str], visitor: V) -> Result<V::Value, KvpError> {
        // 将整个 kvp 字符串作为 map 解析
        let access = KvpMapAccess { de: self.value, pos: 0, value: None };
        // 简化: 使用整个 input
        visitor.visit_map(access)
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _: &'static str, _: &'static [&str], _: V) -> Result<V::Value, KvpError> {
        Err(Error::custom("enum not supported"))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        self.deserialize_str(v)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, KvpError> {
        v.visit_unit()
    }

    // 其余通过 forward_to_deserialize_any
    forward_to_deserialize_any! {
        i8 i16 u8 u16 u32 f32 f64 char bytes byte_buf
    }
}

fn from_kvp<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T, KvpError> {
    // 需要实现 deserialize_struct,用 MapAccess 解析 kvp
    // 简化为使用 ValueDeserializer
    T::deserialize(ValueDeserializer { value: s })
}

// ============================================================
// 测试类型
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ServerConfig {
    host: String,
    port: u16,
    debug: bool,
    max_connections: u32,
}

// ============================================================
// Main
// ============================================================
fn main() -> Result<(), KvpError> {
    println!("=== KVP Format Roundtrip Demo ===\n");

    let config = ServerConfig {
        host: "127.0.0.1".into(),
        port: 8080,
        debug: true,
        max_connections: 1000,
    };

    // 序列化
    let kvp_str = to_kvp(&config)?;
    println!("1. Serialized struct:");
    println!("   {}", kvp_str);
    println!("   (format: key=value;key=value;...)");

    // 验证序列化结果
    assert!(kvp_str.contains("host=127.0.0.1"));
    assert!(kvp_str.contains("port=8080"));
    assert!(kvp_str.contains("debug=true"));
    assert!(kvp_str.contains("max_connections=1000"));

    // 反序列化
    println!("\n2. Deserialized back:");
    let decoded: ServerConfig = from_kvp(&kvp_str)?;
    println!("   {:?}", decoded);

    assert_eq!(config, decoded);
    println!("\n3. Roundtrip successful!");

    // 其他类型测试
    println!("\n4. Individual value serialization:");
    let mut ser = KvpSerializer::new();
    42u32.serialize(&mut ser)?;
    println!("   u32 42 → '{}'", ser.into_string());

    let mut ser = KvpSerializer::new();
    true.serialize(&mut ser)?;
    println!("   bool true → '{}'", ser.into_string());

    let mut ser = KvpSerializer::new();
    "hello".serialize(&mut ser)?;
    println!("   str hello → '{}'", ser.into_string());

    println!("\n===== KVP format demo complete! =====");
    Ok(())
}
