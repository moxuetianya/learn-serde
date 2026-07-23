# 第四章: Serializer —— 编写数据格式(序列化侧)

**源码参考**: `serde_core/src/ser/mod.rs:355` 和 `serde_core/src/ser/fmt.rs`

## Serializer Trait

```rust
// 源码: serde_core/src/ser/mod.rs:355
pub trait Serializer: Sized {
    // 关联类型
    type Ok;  // 序列化成功后的输出类型
    type Error: Error;  // 错误类型

    // 7 个复合类型状态机关联类型
    type SerializeSeq: SerializeSeq<Ok = Self::Ok, Error = Self::Error>;
    type SerializeTuple: SerializeTuple<Ok = Self::Ok, Error = Self::Error>;
    type SerializeTupleStruct: SerializeTupleStruct<Ok = Self::Ok, Error = Self::Error>;
    type SerializeTupleVariant: SerializeTupleVariant<Ok = Self::Ok, Error = Self::Error>;
    type SerializeMap: SerializeMap<Ok = Self::Ok, Error = Self::Error>;
    type SerializeStruct: SerializeStruct<Ok = Self::Ok, Error = Self::Error>;
    type SerializeStructVariant: SerializeStructVariant<Ok = Self::Ok, Error = Self::Error>;

    // === 28 个序列化方法 ===

    // 基本类型
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error>;
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error>;
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error>;
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error>;
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error>;
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error>;
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error>;
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error>;
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error>;
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error>;

    // 字符串和字节
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error>;
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error>;

    // Option
    fn serialize_none(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>;

    // Unit 类型
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_variant(self, name: &'static str, idx: u32, variant: &'static str)
        -> Result<Self::Ok, Self::Error>;

    // Newtype
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>;
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self, name: &'static str, idx: u32, variant: &'static str, value: &T)
        -> Result<Self::Ok, Self::Error>;

    // Sequence
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error>;
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error>;
    fn serialize_tuple_struct(self, name: &'static str, len: usize)
        -> Result<Self::SerializeTupleStruct, Self::Error>;
    fn serialize_tuple_variant(
        self, name: &'static str, idx: u32, variant: &'static str, len: usize)
        -> Result<Self::SerializeTupleVariant, Self::Error>;

    // Map
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error>;
    fn serialize_struct(self, name: &'static str, len: usize)
        -> Result<Self::SerializeStruct, Self::Error>;
    fn serialize_struct_variant(
        self, name: &'static str, idx: u32, variant: &'static str, len: usize)
        -> Result<Self::SerializeStructVariant, Self::Error>;

    // === 带默认实现的方法 ===

    // i128/u128 默认返回错误(很多格式不支持)
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        let _ = v;
        Err(Error::custom("i128 is not supported"))
    }
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> { /* 同上 */ }

    // 便利方法(有默认实现,但可覆盖以优化)
    fn collect_str<T: Display + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&value.to_string())
    }
    fn collect_seq<I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where I: IntoIterator, I::Item: Serialize { /* 默认遍历实现 */ }

    fn is_human_readable(&self) -> bool { true }
}
```

## 实现一个简单的 Serializer: 将值格式化为字符串

让我们实现一个最小的 Serializer,将 serde 数据模型转换为字符串:

```rust
use serde::ser::{self, Serialize};

struct StringSerializer {
    output: String,
}

// 简单起见,错误用 &str
impl ser::Serializer for StringSerializer {  /* 见下文 */ }
impl ser::SerializeSeq for StringSerializer { /* ... */ }
// ... 其他辅助 trait

// 使用示例
// let mut ser = StringSerializer { output: String::new() };
// true.serialize(&mut ser)?;
// assert_eq!(ser.output, "true");
```

完整实现需要:

1. 基本类型 → 直接格式化到 output
2. 复合类型 → 返回实现了对应 trait 的 self (或新类型)

```rust
impl ser::Serializer for StringSerializer {
    // Ok = &str, Error = &str (简化)
    type Ok = ();
    type Error = &'static str;

    // 所有复合类型都用 Self(自己实现了所有辅助 trait)
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), &'static str> {
        write!(self.output, "{}", v).unwrap();
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<(), &'static str> {
        write!(self.output, "{}", v).unwrap();
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<(), &'static str> {
        self.output.push_str(v);
        Ok(())
    }

    fn serialize_unit(self) -> Result<(), &'static str> {
        self.output.push_str("()");
        Ok(())
    }

    fn serialize_none(self) -> Result<(), &'static str> {
        self.output.push_str("None");
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), &'static str> {
        self.output.push_str("Some(");
        v.serialize(self)?;
        self.output.push(')');
        Ok(())
    }

    // serialize_seq 返回 self(因为 SerializeSeq 对应 Self)
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self, &'static str> {
        self.output.push('[');
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self, &'static str> {
        self.output.push('{');
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self, &'static str> {
        self.output.push_str(&format!("{} {{ ", _name));
        Ok(self)
    }

    // ... 其他方法
}

// 实现 SerializeSeq(因为 SerializeSeq = Self)
impl ser::SerializeSeq for StringSerializer {
    type Ok = ();
    type Error = &'static str;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), &'static str> {
        if !self.output.ends_with('[') {
            self.output.push_str(", ");
        }
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn end(self) -> Result<(), &'static str> {
        self.output.push(']');
        Ok(())
    }
}
```

## 序列化错误: ser::Error

```rust
// 源码: serde_core/src/ser/mod.rs:148
pub trait Error: Debug + Display {
    fn custom<T: Display>(msg: T) -> Self;
    // 也可以是 std::error::Error (如果启用 std feature)
}
```

## 源码研读: fmt::Formatter 的 Serializer 实现

`serde_core/src/ser/fmt.rs` 是 serde 内置的一个 Serializer 实现,可以将基础类型直接写入 `fmt::Formatter`:

```rust
// 源码: serde_core/src/ser/fmt.rs ~10-40
impl<'a, 'b> Serializer for &'b mut Formatter<'a> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_bool(self, v: bool) -> fmt::Result {
        Display::fmt(&v, self)
    }
    fn serialize_str(self, v: &str) -> fmt::Result {
        Display::fmt(&v, self)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self, _name: &'static str, value: &T
    ) -> fmt::Result {
        value.serialize(self)  // newtype 透明序列化
    }

    // 复合类型都返回 Impossible —— Formatter 不支持复合类型
    type SerializeSeq = Impossible<(), fmt::Error>;
    // ...
    fn serialize_seq(self, _len: Option<usize>) -> fmt::Result {
        Err(fmt::Error)  // 不支持!
    }
}
```

## 辅助宏: __serialize_unimplemented!

在 `serde_core/src/private/doc.rs` 中,serde 为文档演示提供了一组宏,可以快速"stub out"所有 Serializer 方法:

```rust
// 源码: serde_core/src/private/doc.rs
macro_rules! __serialize_unimplemented {
    ($($t:ident)*) => {
        $(
            __serialize_unimplemented_method!($t);
        )*
    };
}

// 在文档演示 Serialize 实现时,使用这个宏可以省略
// 不相关的 Serializer 方法实现
```

---

**练习**:
1. 阅读 `serde_core/src/ser/fmt.rs`,理解 `Formatter` Serializer 的完整实现
2. 实现一个 `CsvSerializer`,将 struct 序列化为 CSV 行
3. 追踪: 当调用 `42i32.serialize(&mut json_serializer)` 时,代码执行路径是什么?
