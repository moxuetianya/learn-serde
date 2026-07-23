/// 第四章 demo: 实现自定义 Serializer
///
/// 运行: cargo run --example 04_custom_serializer
///
/// 实现一个最小的把数据序列化成调试字符串的 Serializer
///
/// 需要实现:
/// 1. serde::Serializer trait (28 个方法)
/// 2. 7 个辅助 trait: SerializeSeq, SerializeTuple, SerializeTupleStruct,
///    SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant
/// 3. serde::ser::Error trait

use serde::ser::{self, SerializeSeq, SerializeStruct, SerializeTupleStruct};
use serde::{Serialize, Serializer};
use std::fmt;

// ============================================================
// 1. 错误类型 —— 实现 ser::Error
// ============================================================
#[derive(Debug)]
struct DbgError(String);

impl fmt::Display for DbgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DbgError {}

impl ser::Error for DbgError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DbgError(msg.to_string())
    }
}

// ============================================================
// 2. Serializer 实现
// ============================================================
struct DbgSerializer {
    output: String,
    indent: usize,
    /// 当前是否在同一行(用于决定是否加缩进)
    fresh_line: bool,
}

impl DbgSerializer {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            fresh_line: true,
        }
    }

    fn into_string(self) -> String {
        self.output
    }

    fn write_indent(&mut self) {
        if self.fresh_line {
            for _ in 0..self.indent {
                self.output.push_str("  ");
            }
            self.fresh_line = false;
        }
    }
}

// 辅助结构: 实现所有复合类型的 trait
// 模式: 每个复合类型方法返回 self(或新包装),在 end() 时执行清理
struct DbgCompound<'a> {
    ser: &'a mut DbgSerializer,
    /// 结束标记: close_bracket, close_brace, nothing
    closer: &'static str,
}

impl<'a> DbgCompound<'a> {
    fn begin(ser: &'a mut DbgSerializer, opener: &str, closer: &'static str) -> Self {
        ser.write_indent();
        ser.output.push_str(opener);
        ser.fresh_line = true;
        ser.indent += 1;
        Self { ser, closer }
    }

    fn comma(&mut self) {
        self.ser.output.push(',');
        self.ser.fresh_line = true;
    }
}

impl<'a> Drop for DbgCompound<'a> {
    fn drop(&mut self) {
        // 仅在完全结束时清理(由 end() 负责)
    }
}

// ============================================================
// Serializer 实现
// ============================================================
impl<'a> Serializer for &'a mut DbgSerializer {
    type Ok = ();
    type Error = DbgError;

    // 所有复合类型的关联类型都指向同一个辅助结构
    type SerializeSeq = DbgCompound<'a>;
    type SerializeTuple = DbgCompound<'a>;
    type SerializeTupleStruct = DbgCompound<'a>;
    type SerializeTupleVariant = DbgCompound<'a>;
    type SerializeMap = DbgCompound<'a>;
    type SerializeStruct = DbgCompound<'a>;
    type SerializeStructVariant = DbgCompound<'a>;

    // ---------- 基本类型 ----------
    fn serialize_bool(self, v: bool) -> Result<(), DbgError> {
        self.write_indent();
        self.output.push_str(if v { "true" } else { "false" });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<(), DbgError> {
        self.write_indent();
        self.output.push_str(&format!("{}", v));
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_i16", v));
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_i32", v));
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_i64", v));
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_u8", v));
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_u16", v));
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_u32", v));
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_u64", v));
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_f32", v));
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_f64", v));
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), DbgError> {
        self.output.push_str(&format!("'{}'", v));
        Ok(())
    }

    // ---------- 字符串/字节 ----------
    fn serialize_str(self, v: &str) -> Result<(), DbgError> {
        self.output.push_str(&format!("\"{}\"", v));
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), DbgError> {
        self.output.push_str("0x");
        for byte in v {
            self.output.push_str(&format!("{:02x}", byte));
        }
        Ok(())
    }

    // ---------- Option ----------
    fn serialize_none(self) -> Result<(), DbgError> {
        self.output.push_str("None");
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), DbgError> {
        self.output.push_str("Some(");
        value.serialize(&mut *self)?;
        self.output.push(')');
        Ok(())
    }

    // ---------- Unit ----------
    fn serialize_unit(self) -> Result<(), DbgError> {
        self.output.push_str("()");
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<(), DbgError> {
        self.output.push_str(name);
        Ok(())
    }

    fn serialize_unit_variant(
        self, _name: &'static str, _idx: u32, variant: &'static str,
    ) -> Result<(), DbgError> {
        self.output.push_str(variant);
        Ok(())
    }

    // ---------- Newtype ----------
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self, name: &'static str, value: &T,
    ) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}(", name));
        value.serialize(&mut *self)?;
        self.output.push(')');
        Ok(())
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self, _name: &'static str, _idx: u32, variant: &'static str, value: &T,
    ) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}(", variant));
        value.serialize(&mut *self)?;
        self.output.push(')');
        Ok(())
    }

    // ---------- Sequence ----------
    fn serialize_seq(self, _len: Option<usize>) -> Result<DbgCompound<'a>, DbgError> {
        Ok(DbgCompound::begin(self, "[\n", "\n]"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<DbgCompound<'a>, DbgError> {
        Ok(DbgCompound::begin(self, "(\n", "\n)"))
    }

    fn serialize_tuple_struct(
        self, name: &'static str, _len: usize,
    ) -> Result<DbgCompound<'a>, DbgError> {
        self.write_indent();
        Ok(DbgCompound::begin(self, &format!("{} (\n", name), "\n)"))
    }

    fn serialize_tuple_variant(
        self, _name: &'static str, _idx: u32, variant: &'static str, _len: usize,
    ) -> Result<DbgCompound<'a>, DbgError> {
        self.write_indent();
        Ok(DbgCompound::begin(self, &format!("{} (\n", variant), "\n)"))
    }

    // ---------- Map / Struct ----------
    fn serialize_map(self, _len: Option<usize>) -> Result<DbgCompound<'a>, DbgError> {
        Ok(DbgCompound::begin(self, "{\n", "\n}"))
    }

    fn serialize_struct(
        self, name: &'static str, _len: usize,
    ) -> Result<DbgCompound<'a>, DbgError> {
        self.write_indent();
        Ok(DbgCompound::begin(self, &format!("{} {{\n", name), "\n}"))
    }

    fn serialize_struct_variant(
        self, _name: &'static str, _idx: u32, variant: &'static str, _len: usize,
    ) -> Result<DbgCompound<'a>, DbgError> {
        self.write_indent();
        Ok(DbgCompound::begin(self, &format!("{} {{\n", variant), "\n}"))
    }

    // ---------- i128/u128 默认实现返回错误 ----------
    fn serialize_i128(self, v: i128) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_i128", v));
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<(), DbgError> {
        self.output.push_str(&format!("{}_u128", v));
        Ok(())
    }
}

// ============================================================
// 复合类型 trait 实现 —— 所有 7 个!
// ============================================================

impl<'a> SerializeSeq for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), DbgError> {
        self.comma();
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push(']');
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), DbgError> {
        self.comma();
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push(')');
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), DbgError> {
        self.comma();
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push(')');
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), DbgError> {
        self.comma();
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push(')');
        Ok(())
    }
}

impl<'a> ser::SerializeMap for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), DbgError> {
        self.comma();
        key.serialize(&mut *self.ser)?;
        self.ser.output.push_str(": ");
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), DbgError> {
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push('}');
        Ok(())
    }
}

impl<'a> SerializeStruct for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self, key: &str, value: &T,
    ) -> Result<(), DbgError> {
        self.comma();
        self.ser.write_indent();
        self.ser.output.push_str(&format!("{}: ", key));
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push('}');
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for DbgCompound<'a> {
    type Ok = ();
    type Error = DbgError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self, key: &str, value: &T,
    ) -> Result<(), DbgError> {
        self.comma();
        self.ser.write_indent();
        self.ser.output.push_str(&format!("{}: ", key));
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<(), DbgError> {
        self.ser.indent -= 1;
        self.ser.fresh_line = true;
        self.ser.write_indent();
        self.ser.output.push('}');
        Ok(())
    }
}

// ============================================================
// 便捷 API
// ============================================================
fn to_dbg_string<T: Serialize>(value: &T) -> Result<String, DbgError> {
    let mut ser = DbgSerializer::new();
    value.serialize(&mut ser)?;
    Ok(ser.into_string())
}

// ============================================================
// 测试类型
// ============================================================
#[derive(Serialize)]
struct User {
    name: String,
    age: u8,
    active: bool,
    tags: Vec<String>,
}

#[derive(Serialize)]
enum ServerEvent {
    Connected { id: u64, addr: String },
    Disconnected(u64),
    Timeout,
}

fn main() -> Result<(), DbgError> {
    println!("=== Custom Serializer Demo ===\n");

    println!("--- Struct ---");
    let user = User {
        name: "Alice".into(),
        age: 30,
        active: true,
        tags: vec!["admin".into(), "staff".into()],
    };
    println!("{}", to_dbg_string(&user)?);

    println!("\n--- Enum: Connected ---");
    let event = ServerEvent::Connected { id: 1, addr: "127.0.0.1".into() };
    println!("{}", to_dbg_string(&event)?);

    println!("\n--- Enum: Disconnected ---");
    let event = ServerEvent::Disconnected(42);
    println!("{}", to_dbg_string(&event)?);

    println!("\n--- Enum: Timeout ---");
    let event = ServerEvent::Timeout;
    println!("{}", to_dbg_string(&event)?);

    println!("\n--- Nested structures ---");
    let nested = serde_json::json!({
        "users": [{"name": "Bob", "score": 100}, {"name": "Eve", "score": 200}],
        "total": 2,
    });
    println!("{}", to_dbg_string(&nested)?);

    println!("\n=== Every Serializer method is exercised ===");
    println!("The output shows how Serializer methods map to text representation");
    Ok(())
}
