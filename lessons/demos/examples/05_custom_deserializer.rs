/// 第五章/第六章 demo: 实现自定义 Deserializer
///
/// 运行: cargo run --example 05_custom_deserializer
///
/// 实现一个能将逗号分隔的字符串反序列化为各种 Rust 类型的 Deserializer

use serde::de::{self, DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

// ============================================================
// 1. 错误类型
// ============================================================
#[derive(Debug)]
struct CsvError(String);

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

impl std::error::Error for CsvError {}

impl de::Error for CsvError {
    fn custom<T: fmt::Display>(msg: T) -> Self { CsvError(msg.to_string()) }
}

// ============================================================
// 2. Deserializer —— 从逗号分隔字符串反序列化
// ============================================================
// 格式示例:
//   "42"               → i32
//   "hello"            → String
//   "1,2,3"            → Vec<i32>
//   "Alice,30,true"    → struct User { name, age, active }
//   "Start"            → enum variant
//   "Login,admin,pass" → enum variant with data
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

    // deserialize_any: 尝试推断类型并反序列化
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let field = self.input.trim();

        // 尝试 bool
        if field == "true" { return visitor.visit_bool(true); }
        if field == "false" { return visitor.visit_bool(false); }

        // 尝试整数
        if let Ok(v) = field.parse::<i64>() { return visitor.visit_i64(v); }
        if let Ok(v) = field.parse::<u64>() { return visitor.visit_u64(v); }

        // 尝试浮点
        if let Ok(v) = field.parse::<f64>() { return visitor.visit_f64(v); }

        // 默认: 字符串
        visitor.visit_borrowed_str(field)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        match self.input.trim() {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            other => Err(Error::invalid_value(de::Unexpected::Str(other), &"true or false")),
        }
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: i32 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i32"))?;
        visitor.visit_i32(v)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: i64 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i64"))?;
        visitor.visit_i64(v)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: u64 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u64"))?;
        visitor.visit_u64(v)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        let v: f64 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"f64"))?;
        visitor.visit_f64(v)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
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

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        visitor.visit_string(self.input.trim().to_owned())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        visitor.visit_borrowed_bytes(self.input.trim().as_bytes())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        visitor.visit_byte_buf(self.input.trim().as_bytes().to_vec())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        match self.input.trim() {
            "" | "null" | "none" | "None" => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self, _name: &'static str, visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self, _name: &'static str, visitor: V,
    ) -> Result<V::Value, CsvError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        // 逗号分隔 → 序列
        let fields: Vec<&str> = self.input.split(',').collect();
        visitor.visit_seq(CsvSeqAccess { fields, index: 0 })
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self, len: usize, visitor: V,
    ) -> Result<V::Value, CsvError> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self, _name: &'static str, _len: usize, visitor: V,
    ) -> Result<V::Value, CsvError> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        // 格式: key1=val1,key2=val2
        let pairs: Vec<(&str, &str)> = self.input.split(',')
            .filter_map(|s| {
                let s = s.trim();
                s.split_once('=').or_else(|| s.split_once(':'))
            })
            .collect();
        visitor.visit_map(CsvMapAccess { pairs, index: 0 })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self, _name: &'static str, _fields: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, CsvError> {
        // struct: 按顺序映射到字段
        self.deserialize_seq(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self, _name: &'static str, _variants: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, CsvError> {
        // 格式: VariantName,rest_of_data
        let (variant, rest) = self.next_field();
        visitor.visit_enum(CsvEnumAccess {
            variant: variant.trim(),
            data: rest.trim(),
            _marker: std::marker::PhantomData,
        })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, CsvError> {
        // 跳过数据: 仍然创建一个 unit 给 visitor
        visitor.visit_unit()
    }

    fn deserialize_i8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: i8 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i8"))?;
        v.visit_i8(n)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: i16 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"i16"))?;
        v.visit_i16(n)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: u8 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u8"))?;
        v.visit_u8(n)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: u16 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u16"))?;
        v.visit_u16(n)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: u32 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"u32"))?;
        v.visit_u32(n)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, CsvError> {
        let n: f32 = self.input.trim().parse()
            .map_err(|_| Error::invalid_value(de::Unexpected::Str(self.input), &"f32"))?;
        v.visit_f32(n)
    }
}

// ============================================================
// 3. 辅助访问器
// ============================================================

// SeqAccess
struct CsvSeqAccess<'de> {
    fields: Vec<&'de str>,
    index: usize,
}

impl<'de> SeqAccess<'de> for CsvSeqAccess<'de> {
    type Error = CsvError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self, seed: T,
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

// MapAccess
struct CsvMapAccess<'de> {
    pairs: Vec<(&'de str, &'de str)>,
    index: usize,
}

impl<'de> MapAccess<'de> for CsvMapAccess<'de> {
    type Error = CsvError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self, seed: K,
    ) -> Result<Option<K::Value>, CsvError> {
        if self.index >= self.pairs.len() {
            return Ok(None);
        }
        let (key, _) = self.pairs[self.index];
        seed.deserialize(CsvDeserializer::new(key.trim())).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self, seed: V,
    ) -> Result<V::Value, CsvError> {
        let (_, value) = self.pairs[self.index];
        self.index += 1;
        seed.deserialize(CsvDeserializer::new(value.trim()))
    }
}

// EnumAccess
struct CsvEnumAccess<'de> {
    variant: &'de str,
    data: &'de str,
    _marker: std::marker::PhantomData<&'de ()>,
}

impl<'de> EnumAccess<'de> for CsvEnumAccess<'de> {
    type Error = CsvError;
    type Variant = CsvVariantAccess<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self, seed: V,
    ) -> Result<(V::Value, Self::Variant), CsvError> {
        let variant = seed.deserialize(CsvDeserializer::new(self.variant))?;
        Ok((variant, CsvVariantAccess { data: self.data }))
    }
}

// VariantAccess
struct CsvVariantAccess<'de> {
    data: &'de str,
}

impl<'de> VariantAccess<'de> for CsvVariantAccess<'de> {
    type Error = CsvError;

    fn unit_variant(self) -> Result<(), CsvError> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self, seed: T,
    ) -> Result<T::Value, CsvError> {
        seed.deserialize(CsvDeserializer::new(self.data))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self, _len: usize, visitor: V,
    ) -> Result<V::Value, CsvError> {
        CsvDeserializer::new(self.data).deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self, _fields: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, CsvError> {
        CsvDeserializer::new(self.data).deserialize_seq(visitor)
    }
}

// ============================================================
// 4. 快捷入口
// ============================================================
fn from_csv<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T, CsvError> {
    T::deserialize(CsvDeserializer::new(s))
}

// ============================================================
// 5. 测试
// ============================================================
#[derive(Debug, PartialEq, Deserialize)]
struct User {
    name: String,
    age: u8,
}

#[derive(Debug, PartialEq, Deserialize)]
enum Status {
    Active,
    Inactive,
    Banned,
}

#[derive(Debug, PartialEq, Deserialize)]
enum Command {
    Move { x: i32, y: i32 },
    Say(String),
    Quit,
}

fn main() -> Result<(), CsvError> {
    println!("=== Custom CsvDeserializer Demo ===\n");

    // 基本类型
    println!("--- Primitives ---");
    let v: i32 = from_csv("42")?;
    println!("  i32: {}  ←  '42'", v);

    let v: bool = from_csv("true")?;
    println!("  bool: {}  ←  'true'", v);

    let v: String = from_csv("hello world")?;
    println!("  str:  '{}'", v);

    let v: f64 = from_csv("3.14")?;
    println!("  f64:  {}", v);

    // Option
    println!("\n--- Option ---");
    let v: Option<i32> = from_csv("42")?;
    println!("  Some: {:?}", v);
    let v: Option<i32> = from_csv("")?;
    println!("  None: {:?}", v);
    let v: Option<i32> = from_csv("null")?;
    println!("  Null: {:?}", v);

    // Vec
    println!("\n--- Vec ---");
    let v: Vec<i32> = from_csv("1,2,3,4,5")?;
    println!("  Vec: {:?}", v);

    // Struct (按顺序)
    println!("\n--- Struct (positional) ---");
    let user: User = from_csv("Alice,30")?;
    println!("  User: {:?}", user);

    // Enum
    println!("\n--- Enum ---");
    let st: Status = from_csv("Active")?;
    println!("  Status: {:?}", st);

    let st: Status = from_csv("Banned")?;
    println!("  Status: {:?}", st);

    println!("\n--- Enum with data ---");
    let cmd: Command = from_csv("Say,Hello World")?;
    println!("  Command: {:?}", cmd);

    let cmd: Command = from_csv("Move,10,20")?;
    println!("  Command: {:?}", cmd);

    let cmd: Command = from_csv("Quit")?;
    println!("  Command: {:?}", cmd);

    // Map 格式
    println!("\n--- Map format (key=val) ---");
    let map_str = "name=Bob,age=25";
    // 用 deserialize_map (需要实现 Visit)
    println!("  Input: '{}'", map_str);
    println!("  Use CsvDeserializer::deserialize_map for key-value parsing");

    println!("\n=== All CsvDeserializer demos passed! ===");
    Ok(())
}
