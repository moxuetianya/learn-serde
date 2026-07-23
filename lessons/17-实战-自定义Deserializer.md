# 第十七章: 实战 —— 完整编写自定义 Deserializer

**本教程配合作业**: 针对第十六章的二进制格式,实现对应的反序列化器

## 与 Serializer 的对称性

Serializer 和 Deserializer 的设计是镜像对称的:

| Serializer (输出) | Deserializer (输入) |
|---|---|
| `serialize_i32(v)` → 写 bytes | `deserialize_i32(visitor)` → 读 bytes → `visitor.visit_i32(v)` |
| `serialize_struct(name, len)` → 返回 SerializeStruct | `deserialize_struct(name, fields, visitor)` → 调用 visitor |
| `SerializeSeq` 写元素 | `SeqAccess` 读元素 |
| `SerializeMap` 写键值对 | `MapAccess` 读键值对 |

核心区别: Deserializer 需要 `Visitor`,因为输入数据的类型可能不精确匹配期望。

## 完整示例: BinaryDeserializer

对应第十六章的二进制格式的反序列化器:

```rust
pub struct Deserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

impl<'de> Deserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self { input, pos: 0 }
    }

    // 辅助: 读取 N 个字节
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        if self.pos + N > self.input.len() {
            return Err(Error::custom("unexpected EOF"));
        }
        let mut buf = [0u8; N];
        buf.copy_from_slice(&self.input[self.pos..self.pos + N]);
        self.pos += N;
        Ok(buf)
    }

    // 辅助: 读取 u32 LE 长度 + data
    fn read_len_prefixed(&mut self) -> Result<&'de [u8], Error> {
        let len = u32::from_le_bytes(self.read_bytes::<4>()?) as usize;
        if self.pos + len > self.input.len() {
            return Err(Error::custom("unexpected EOF"));
        }
        let data = &self.input[self.pos..self.pos + len];
        self.pos += len;
        Ok(data)
    }
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Error> {
        // 非自描述格式,不支持
        Err(Error::custom("deserialize_any not supported"))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let byte = self.read_bytes::<1>()?[0];
        visitor.visit_bool(byte != 0)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let bytes = self.read_bytes::<4>()?;
        visitor.visit_u32(u32::from_le_bytes(bytes))
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let bytes = self.read_bytes::<8>()?;
        visitor.visit_u64(u64::from_le_bytes(bytes))
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let encoded = u32::from_le_bytes(self.read_bytes::<4>()?);
        // zigzag decode: 将正数还原为有符号数
        let decoded = ((encoded >> 1) as i32) ^ -((encoded & 1) as i32);
        visitor.visit_i32(decoded)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let encoded = u64::from_le_bytes(self.read_bytes::<8>()?);
        let decoded = ((encoded >> 1) as i64) ^ -((encoded & 1) as i64);
        visitor.visit_i64(decoded)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let bytes = self.read_bytes::<8>()?;
        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let data = self.read_len_prefixed()?;
        let s = std::str::from_utf8(data).map_err(Error::custom)?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let data = self.read_len_prefixed()?;
        let s = std::str::from_utf8(data).map_err(Error::custom)?;
        visitor.visit_string(s.to_owned())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let data = self.read_len_prefixed()?;
        visitor.visit_borrowed_bytes(data)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let data = self.read_len_prefixed()?;
        visitor.visit_byte_buf(data.to_vec())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let tag = self.read_bytes::<1>()?[0];
        if tag == 0 {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self, _name: &'static str, visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self, _name: &'static str, visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let len = u32::from_le_bytes(self.read_bytes::<4>()?) as usize;
        visitor.visit_seq(BinarySeqAccess { de: self, len, index: 0 })
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self, len: usize, visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_seq(BinarySeqAccess { de: self, len, index: 0 })
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self, _name: &'static str, len: usize, visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_seq(BinarySeqAccess { de: self, len, index: 0 })
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let len = u32::from_le_bytes(self.read_bytes::<4>()?) as usize;
        visitor.visit_map(BinaryMapAccess { de: self, len, index: 0 })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self, _name: &'static str, _fields: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, Error> {
        // 对于 type-driven 格式,struct 直接按顺序读字段
        // (不包含字段名)
        visitor.visit_seq(BinarySeqAccess { de: self, len: _fields.len(), index: 0 })
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self, _name: &'static str, variants: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, Error> {
        // external: 读取 variant index,然后根据 variant 反序列化数据
        let idx = u32::from_le_bytes(self.read_bytes::<4>()?);
        visitor.visit_enum(BinaryEnumAccess {
            de: self,
            idx,
            variants,
        })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        // type-driven 格式: 字段名不会出现在数据中
        // 返回 error —— 因为标识符不是从输入中读取的
        Err(Error::custom("identifiers not in binary format"))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        // 跳过任何类型的数据 —— 简单实现: 读一个字节
        // 更好的做法: 根据已知类型信息跳过
        let _ = self.read_bytes::<1>()?;
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        i8 i16 u8 u16 f32 char
    }
}

// SeqAccess 实现
struct BinarySeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
    index: usize,
}

impl<'de, 'a> SeqAccess<'de> for BinarySeqAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self, seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if self.index >= self.len {
            return Ok(None);
        }
        self.index += 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// MapAccess 实现(同 seq,但读 key+value 对)
struct BinaryMapAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
    index: usize,
}

impl<'de, 'a> MapAccess<'de> for BinaryMapAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self, seed: K,
    ) -> Result<Option<K::Value>, Error> {
        if self.index >= self.len {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self, seed: V,
    ) -> Result<V::Value, Error> {
        self.index += 1;
        seed.deserialize(&mut *self.de)
    }
}

// EnumAccess 实现
struct BinaryEnumAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    idx: u32,
    variants: &'static [&'static str],
}

impl<'de, 'a> EnumAccess<'de> for BinaryEnumAccess<'a, 'de> {
    type Error = Error;
    type Variant = BinaryVariantAccess<'a, 'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self, seed: V,
    ) -> Result<(V::Value, Self::Variant), Error> {
        let idx = self.idx as u32;
        // 将 idx 转为 variant 名
        let variant = seed.deserialize(BinaryVariantDeserializer { idx })?;
        Ok((variant, BinaryVariantAccess { de: self.de }))
    }
}

struct BinaryVariantAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'de, 'a> VariantAccess<'de> for BinaryVariantAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> { Ok(()) }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(&mut *self.de)
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        self.de.deserialize_tuple(len, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self, fields: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, Error> {
        self.de.deserialize_struct("", fields, visitor)
    }
}
```

## 关键设计决策

### 1. 生命周期

```rust
impl<'de> Deserializer<'de> for &mut Deserializer<'de>
                        // ^^^ 引用,因为需要消耗输入
```

Deserializer 通常实现为 `&mut self` 的引用,这样可以在 `SeqAccess` 中共享:

```rust
struct BinarySeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,  // 共享 deserializer
    // ...
}
```

### 2. type-driven 格式的特性

- `deserialize_any` 不支持 → 返回错误
- `deserialize_identifier` 不需要 → 返回错误
- struct 的反序列化不关心字段名 → 用 visit_seq 按顺序读

### 3. Visitor 的生命周期条件

```rust
// visitor.visit_borrowed_str(s) 要求 s: &'de str
fn deserialize_str<V: Visitor<'de>>(self, visitor: V) {
    let data = self.read_len_prefixed()?;     // data: &'de [u8]
    let s = std::str::from_utf8(data)?;       // s: &'de str ← 零拷贝!
    visitor.visit_borrowed_str(s)
}
```

---

**练习**:
1. 实现完整的 BinaryDeserializer,支持 enum 的 external, internal, adjacent 三种策略
2. 实现 roundtrip 测试: 同一数据 序列化 → 反序列化 → 相等
3. 阅读 serde_json 的 Deserializer 源码,对比 type-driven vs content-driven 的实现差异
