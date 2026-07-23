# 第十六章: 实战 —— 完整编写自定义 Serializer

本章通过实现一个**自定义二进制序列化器**来巩固 Serialize / Serializer 的理解。

## 目标: 实现一个类似 Postcard 的紧凑二进制格式

### 格式规范

```
bool:       1 byte (0 = false, 1 = true)
u8:         1 byte
u16:        2 bytes LE
u32:        4 bytes LE
u64:        8 bytes LE
i8:         1 byte (zigzag 编码: sign + magnitude)
i16-i64:    zigzag 编码 + 对应长度的 LE
f32:        4 bytes LE
f64:        8 bytes LE
str:        4 bytes LE len + UTF-8 data
bytes:      4 bytes LE len + raw data
unit:       0 bytes
none:       1 byte (0x00)
some:       1 byte (0x01) + value
option:     同 some/none
seq:        4 bytes LE len + elements
tuple:      len 不重要(隐式) + elements
map:        4 bytes LE len + (key, value)*
struct:     同 map (fields = (name_str, value))
enum:
  external: 1 byte variant index + value
  internal: 同 struct, tag 是第一个 field
  unit_variant: 1 byte index
```

## 实现

### 1. 错误类型

```rust
#[derive(Debug)]
pub enum Error {
    Message(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
```

### 2. Serializer

```rust
pub struct Serializer {
    output: Vec<u8>,
}

impl Serializer {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }
    pub fn into_vec(self) -> Vec<u8> {
        self.output
    }
}

// 便捷入口
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_vec())
}
```

### 3. 实现 serde::Serializer

```rust
impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    // 所有复合类型都用 Compound 结构
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = Compound<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;

    // === 基本类型 ===

    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        self.output.push(v as u8);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<(), Error> {
        // zigzag encoding: 将负数转为正数
        // -1 → 1, 1 → 2, -128 → 255
        let encoded = ((v << 1) ^ (v >> 7)) as u8;
        self.output.push(encoded);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<(), Error> {
        let encoded = ((v << 1) ^ (v >> 15)) as u16;
        self.output.extend_from_slice(&encoded.to_le_bytes());
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<(), Error> {
        let encoded = ((v << 1) ^ (v >> 31)) as u32;
        self.output.extend_from_slice(&encoded.to_le_bytes());
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<(), Error> {
        let encoded = ((v << 1) ^ (v >> 63)) as u64;
        self.output.extend_from_slice(&encoded.to_le_bytes());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), Error> {
        self.output.push(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<(), Error> {
        self.output.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<(), Error> {
        self.output.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<(), Error> {
        self.output.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<(), Error> {
        self.output.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<(), Error> {
        self.output.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), Error> {
        self.serialize_u32(v as u32)
    }

    // === 字符串和字节 ===

    fn serialize_str(self, v: &str) -> Result<(), Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), Error> {
        // 先写长度(4 bytes LE),再写数据
        let len = v.len() as u32;
        self.output.extend_from_slice(&len.to_le_bytes());
        self.output.extend_from_slice(v);
        Ok(())
    }

    // === Option ===

    fn serialize_none(self) -> Result<(), Error> {
        self.output.push(0);
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), Error> {
        self.output.push(1);
        v.serialize(self)?;
        Ok(())
    }

    // === Unit ===

    fn serialize_unit(self) -> Result<(), Error> {
        Ok(())  // unit = 0 bytes
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self, _name: &'static str, _idx: u32, _variant: &'static str
    ) -> Result<(), Error> {
        // 只写 variant index
        self.serialize_u32(_idx)
    }

    // === Newtype ===

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self, _name: &'static str, value: &T
    ) -> Result<(), Error> {
        value.serialize(self)  // 透明: 直接序列化内层
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self, _name: &'static str, idx: u32, _variant: &'static str, value: &T
    ) -> Result<(), Error> {
        self.serialize_u32(idx)?;  // variant index
        value.serialize(self)      // content
    }

    // === Sequence ===

    fn serialize_seq(self, len: Option<usize>) -> Result<Compound<'a>, Error> {
        // 写长度(或 0)
        self.serialize_u32(len.unwrap_or(0) as u32)?;
        Ok(Compound { ser: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Compound<'a>, Error> {
        Ok(Compound { ser: self })  // 无长度前缀,隐式由类型确定
    }

    fn serialize_tuple_struct(
        self, _name: &'static str, _len: usize
    ) -> Result<Compound<'a>, Error> {
        Ok(Compound { ser: self })
    }

    fn serialize_tuple_variant(
        self, _name: &'static str, idx: u32, _variant: &'static str, _len: usize
    ) -> Result<Compound<'a>, Error> {
        self.serialize_u32(idx)?;
        Ok(Compound { ser: self })
    }

    // === Map ===

    fn serialize_map(self, len: Option<usize>) -> Result<Compound<'a>, Error> {
        self.serialize_u32(len.unwrap_or(0) as u32)?;
        Ok(Compound { ser: self })
    }

    fn serialize_struct(
        self, _name: &'static str, _len: usize
    ) -> Result<Compound<'a>, Error> {
        // struct 不写长度(隐式),只写 fields
        // 但我们的格式中 struct = map,字段名作为 key
        Ok(Compound { ser: self })
    }

    fn serialize_struct_variant(
        self, _name: &'static str, idx: u32, _variant: &'static str, _len: usize
    ) -> Result<Compound<'a>, Error> {
        self.serialize_u32(idx)?;
        Ok(Compound { ser: self })
    }
}
```

### 4. Compound 类型(辅助 trait 实现)

```rust
pub struct Compound<'a> {
    ser: &'a mut Serializer,
}

// SerializeSeq 用于 seq 和 tuple
impl<'a> serde::ser::SerializeSeq for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTuple for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleStruct for Compound<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<(), Error> { Ok(()) }
}

impl<'a> serde::ser::SerializeTupleVariant for Compound<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<(), Error> { Ok(()) }
}

// SerializeMap 用于 map 和 struct
impl<'a> serde::ser::SerializeMap for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a> serde::ser::SerializeStruct for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self, key: &str, value: &T
    ) -> Result<(), Error> {
        // struct field: key(字段名) + value
        key.serialize(&mut *self.ser)?;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a> serde::ser::SerializeStructVariant for Compound<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self, key: &str, value: &T
    ) -> Result<(), Error> {
        key.serialize(&mut *self.ser)?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<(), Error> { Ok(()) }
}
```

### 5. 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool() {
        let v = to_vec(&true).unwrap();
        assert_eq!(v, vec![1]);
    }

    #[test]
    fn test_u32() {
        let v = to_vec(&42u32).unwrap();
        assert_eq!(v, vec![42, 0, 0, 0]);
    }

    #[test]
    fn test_string() {
        let v = to_vec(&"hello").unwrap();
        // len = 5(LE) + "hello"
        assert_eq!(v, vec![5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_roundtrip() {
        // 这个 BinarySerializer 不是自描述格式,
        // 需要配合 BinaryDeserializer 才能做 roundtrip
        // 正常测试用 serde_test
    }
}
```

## 设计要点回顾

1. **Serializer 必须实现所有 28 个方法**,即使某些返回错误
2. **复合类型返回状态机对象**,元素通过 `&mut self` 逐个序列化
3. **关联类型的绑定**: 所有 `Serialize*` 关联类型必须实现对应的 trait
4. **生命周期**: `&'a mut Serializer` 的 `'a` 来自调用上下文
5. **i128/u128**: 默认返回错误,可以实现但非必须
6. **错误传播**: `?` 操作符天然支持,因为 `Error` 实现了 `From<E>` (通过 custom)

---

**练习**:
1. 为这个 Serializer 添加 `serialize_i128` 支持
2. 修改格式: struct 不序列化字段名(按顺序),节省空间
3. 实现 `collect_str` 方法,避免先创建临时 String
