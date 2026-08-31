# 第七章: Deserializer —— 编写数据格式(反序列化侧)

**源码参考**: `serde_core/src/de/mod.rs:945`

## 为什么反序列化 JSON 必须引入 serde_json? —— serde 只是协议

**核心误解**: 以为 `#[derive(Deserialize)]` 就能解析 JSON 字节。不对 ——
derive 宏只生成了**类型侧**(`MyStruct` 怎么组装), 完全不含格式侧:

| 侧 | 职责 | 谁提供 |
|----|------|--------|
| 类型侧 | `MyStruct::deserialize` + visitor: "字段名 x/y, 值怎么转换" | `#[derive(Deserialize)]` (serde) |
| 格式侧 | 字节 → token (`{` `"` 数字 转义), 实现 `Deserializer` trait | 格式库 (serde_json / serde_yaml / ron / bincode ...) |

`serde` 自己是一个**协议层**: 它定义了 `Deserializer`/`Visitor`/`Deserialize`
三个接口, 但**不认识任何具体格式的语法**。JSON 的 `{"name": "a"}` 是怎么变成
`visit_map`/`visit_str("name")` 这些调用的? serde 不知道, 只有 serde_json 知道。

完整调用链:

```
serde_json::from_slice::<MyStruct>(bytes)
 └─ serde_json 的 Deserializer 实现        ← 格式侧: 解析 JSON 语法, 生产 token
     └─ MyStruct::deserialize(d)            ← 类型侧: derive 生成, 有 MyStruct 的知识
         └─ d.deserialize_map(MyStructVisitor)
             └─ visitor.visit_map(...)      ← 回到格式侧: 逐个读 key/value ...
                 └─ map.next_value_seed(...) ← 子字段递归...
```

关于 `serde_json::Value`: 它是 serde_json 自己的**便利目标类型**(JSON 的 AST),
你确实不需要它。但你要的是 `from_slice::<MyStruct>` 里的**解析器部分** ——
那个实现 `Deserializer` 的 `d`, 只能来自 serde_json。

验证: 仓库 `demos/examples/05_custom_deserializer.rs` 手写了一个 CSV 格式的
`Deserializer`(CsvDeserializer + SeqAccess/MapAccess/EnumAccess), 这就是"格式侧
自己写"的样子 —— serde 生态的接入口就是 `Deserializer` trait, 任何格式只要实现
它就能接入 derive 的类型侧。

**结论**: 想用 JSON 就必须引入一个实现了 `Deserializer` 的库(选哪个取决于格式);
"只依赖 serde" 可行, 但前提是你自己写格式侧(见本章练习和 17-实战-自定义Deserializer)。

## Deserializer Trait 完整定义

```rust
// 源码: serde_core/src/de/mod.rs:945
pub trait Deserializer<'de>: Sized {
    type Error: Error;

    // === 28 个 deserialize 方法 ===

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V)
        -> Result<V::Value, Self::Error>;

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V)
        -> Result<V::Value, Self::Error>;

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, visitor: V)
        -> Result<V::Value, Self::Error>;

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_struct<V: Visitor<'de>>(
        self, name: &'static str, fields: &'static [&'static str], visitor: V)
        -> Result<V::Value, Self::Error>;

    fn deserialize_enum<V: Visitor<'de>>(
        self, name: &'static str, variants: &'static [&'static str], visitor: V)
        -> Result<V::Value, Self::Error>;

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    // === 带默认的方法 ===
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { /* 错误 */ }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { /* 错误 */ }
    fn is_human_readable(&self) -> bool { true }
}
```

## 实现策略: type-driven vs content-driven

### Type-driven 格式 (如 Bincode, Postcard)

type-driven 格式依赖调用方告知期望的类型,不自己推断。

```
struct BincodeDeserializer { bytes: &[u8], pos: usize }

// 每个 deserialize_* 方法直接从字节流读取数据
impl<'de> Deserializer<'de> for BincodeDeserializer {
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = i32::from_le_bytes(/* read 4 bytes */);
        visitor.visit_i32(val)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let len: u64 = /* read length prefix */;
        let bytes = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        visitor.visit_borrowed_str(std::str::from_utf8(bytes)?)
    }

    // 所有其他方法 forward 到 deserialize_any
    forward_to_deserialize_any! {
        bool i8 i16 i64 u8 u16 u32 u64 f32 f64 char
        string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}
```

### Content-driven 格式 (如 JSON)

content-driven 格式的数据是自描述的,JSON parser 自己知道当前 token 的类型:

```
struct JsonDeserializer { /* 持有 serde_json::Value 或 streaming parser */ }

impl<'de> Deserializer<'de> for JsonDeserializer {
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.peek()? {
            Token::Null => visitor.visit_unit(),
            Token::Bool(b) => visitor.visit_bool(b),
            Token::Number(n) => { /* 按需转换为 i32/u64/f64 */ },
            Token::Str(s) => visitor.visit_borrowed_str(s),
            Token::Array { .. } => visitor.visit_seq(self),
            Token::Object { .. } => visitor.visit_map(self),
        }
    }

    // 大多数其他方法 forward 到 deserialize_any
    // 但可以特殊优化:
    // deserialize_i32 可以直接尝试 from_str, 失败则报错
}
```

## 完整示例: 从字符串反序列化的 Deserializer

```rust
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, Error};

struct StrDeserializer<'de> {
    input: &'de str,
}

impl<'de> Deserializer<'de> for StrDeserializer<'de> {
    type Error = de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // 尝试解析为基本类型
        if self.input == "true" { return visitor.visit_bool(true); }
        if self.input == "false" { return visitor.visit_bool(false); }
        if self.input == "null" { return visitor.visit_unit(); }

        if let Ok(n) = self.input.parse::<i64>() {
            return visitor.visit_i64(n);
        }
        if let Ok(n) = self.input.parse::<u64>() {
            return visitor.visit_u64(n);
        }
        if let Ok(n) = self.input.parse::<f64>() {
            return visitor.visit_f64(n);
        }

        // 默认当字符串
        visitor.visit_borrowed_str(self.input)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            _ => Err(de::Error::invalid_value(
                de::Unexpected::Str(self.input), &"true or false"
            )),
        }
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let n: i32 = self.input.parse().map_err(|_| {
            de::Error::invalid_value(de::Unexpected::Str(self.input), &"i32")
        })?;
        visitor.visit_i32(n)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_str(self.input)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            "null" | "()" | "" => visitor.visit_unit(),
            _ => Err(de::Error::invalid_value(de::Unexpected::Str(self.input), &"unit")),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // 对于 Option, 空字符串 = None
        if self.input.is_empty() || self.input == "null" {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // 按逗号分割
        let elements: Vec<&str> = self.input.split(',').collect();
        visitor.visit_seq(CommaSeparated { elements, index: 0 })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self, _name: &'static str, _fields: &'static [&'static str], visitor: V
    ) -> Result<V::Value, Self::Error> {
        // 假设格式为 "field1=val1,field2=val2"
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // 解析 "key1=val1,key2=val2"
        let pairs: Vec<(&str, &str)> = self.input.split(',')
            .filter_map(|s| s.split_once('='))
            .collect();
        visitor.visit_map(KeyValuePairs { pairs, index: 0 })
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self, _name: &'static str, _variants: &'static [&'static str], visitor: V
    ) -> Result<V::Value, Self::Error> {
        // 格式: "VariantName:data" 或 "VariantName"
        match self.input.split_once(':') {
            Some((variant, data)) => {
                visitor.visit_enum(Enum {
                    variant: variant.trim(),
                    data: Some(data.trim()),
                    _marker: std::marker::PhantomData,
                })
            }
            None => {
                visitor.visit_enum(Enum {
                    variant: self.input.trim(),
                    data: None,
                    _marker: std::marker::PhantomData,
                })
            }
        }
    }

    // 其余方法用宏 forward
    forward_to_deserialize_any! {
        i8 i16 i64 u8 u16 u32 u64 f32 f64 char
        string bytes byte_buf
        unit_struct newtype_struct tuple tuple_struct
        identifier ignored_any
    }
}

// SeqAccess 实现: 逗号分隔的元素
struct CommaSeparated<'de> {
    elements: Vec<&'de str>,
    index: usize,
}

impl<'de> SeqAccess<'de> for CommaSeparated<'de> {
    type Error = de::value::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where T: serde::de::DeserializeSeed<'de>
    {
        if self.index >= self.elements.len() {
            return Ok(None);
        }
        let elem = self.elements[self.index];
        self.index += 1;
        seed.deserialize(StrDeserializer { input: elem.trim() }).map(Some)
    }
}
```

## 源码: serde_core/src/de/value.rs

这个文件提供了**最小化的 Deserializer 实现**,用于从已有的 Rust 值构造 Deserializer:

```
BoolDeserializer<E>      -- 总是返回 true/false
I32Deserializer<E>       -- 总是返回一个 i32
StrDeserializer<'a, E>    -- 总是返回一个 &str
SeqDeserializer<I, E>    -- 从迭代器构造序列 deserializer
MapDeserializer<'de, I, E> -- 从键值对迭代器构造 map deserializer
```

这些"值 deserializer"主要用于:
1. **测试**: 在单元测试中模拟反序列化
2. **兜底**: 当需要将现有的 Rust 值当作反序列化的源时
3. **derive 生成代码**: 用于处理 `#[serde(default = "path")]`

```rust
// 示例: 将 Vec<i32> 作为序列 deserializer
use serde::de::value::{SeqDeserializer, Error};

let values = vec![1, 2, 3];
let deserializer = SeqDeserializer::<_, Error>::new(values.into_iter());
let result: Vec<i32> = Vec::deserialize(deserializer)?;
// result == vec![1, 2, 3]
```

---

**练习**:
1. 完成上面的 `StrDeserializer` 实现,处理更多类型
2. 为一个简单的二进制协议实现 `Deserializer`:
   - 前 1 字节: 类型标记 (0=unit, 1=bool, 2=i32, 3=str)
   - 对于 str: 接下来 4 字节 LE u32 = 长度,然后 UTF-8 数据
3. 阅读 `serde_core/src/de/value.rs`,理解单元 deserializer 的设计
