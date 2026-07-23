# 第六章: Visitor 模式详解

**源码参考**: `serde_core/src/de/mod.rs:1317`

## Visitor Trait 完整定义

```rust
// 源码: serde_core/src/de/mod.rs:1317
pub trait Visitor<'de>: Sized {
    type Value;  // 访问后产生的 Rust 类型

    // === 唯一必须实现的方法 ===
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result;

    // === 可选方法(都有默认实现,返回错误) ===

    // 基本类型
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>        { Err(Error::invalid_type(...)) }
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>            { /* 同上 */ }
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>        { /* 同上 */ }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>            { /* 同上 */ }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>        { /* 同上 */ }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>          { /* 同上 */ }
    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>        { /* 同上 */ }

    // 字符串 —— borrowed 和 owned
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>         { /* 同上 */ }
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>  { self.visit_str(v) }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>    { self.visit_str(&v) }

    // 字节 —— borrowed 和 owned
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>      { /* 同上 */ }
    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E> { self.visit_bytes(v) }
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E> { self.visit_bytes(&v) }

    // Option
    fn visit_none<E>(self) -> Result<Self::Value, E>                 { /* 同上 */ }
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de>                                       { /* 同上 */ }

    // Unit / Newtype
    fn visit_unit<E>(self) -> Result<Self::Value, E>                 { /* 同上 */ }
    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de>                                       { /* 同上 */ }

    // 复合
    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where A: SeqAccess<'de>                                          { /* 同上 */ }
    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where A: MapAccess<'de>                                          { /* 同上 */ }
    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where A: EnumAccess<'de>                                         { /* 同上 */ }
}
```

## Visitor 中的三类方法

### 1. 按值接收 (Value semantics)

不需要生命周期,直接消费值:

```rust
fn visit_bool(self, v: bool) -> Result<Self::Value, E>;
fn visit_i32(self, v: i32) -> Result<Self::Value, E>;
fn visit_char(self, v: char) -> Result<Self::Value, E>;
fn visit_string(self, v: String) -> Result<Self::Value, E>;   // 所有权转移
fn visit_byte_buf(self, v: Vec<u8>) -> Result<Self::Value, E>; // 所有权转移
```

### 2. 借用接收 (Borrowed semantics)

需要生命周期 `'de`,允许零拷贝借用 deserializer 的缓冲区:

```rust
fn visit_borrowed_str(self, v: &'de str) -> Result<Self::Value, E>;
fn visit_borrowed_bytes(self, v: &'de [u8]) -> Result<Self::Value, E>;
```

默认实现 fallback 到 `visit_str` / `visit_bytes`。

**零拷贝的关键**: 如果 Visitor::Value 是 `&'de str`, 则 `visit_borrowed_str` 可以直接返回 `v`,不需要分配。

### 3. 传递 control (Control transfer)

将控制权交还给 deserializer,让 deserializer 继续反序列化内层数据:

```rust
fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
where D: Deserializer<'de>;
// 用于 Option<T>: 拿到内层 deserializer,调用 T::deserialize(deserializer)

fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
where D: Deserializer<'de>;
// 用于 newtype struct: 拿到内层 deserializer,调用 inner::deserialize(deserializer)

fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
where A: SeqAccess<'de>;
// 拿到 SeqAccess,遍历元素

fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
where A: MapAccess<'de>;
// 拿到 MapAccess,遍历键值对

fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
where A: EnumAccess<'de>;
// 拿到 EnumAccess,获取 variant 和内容
```

## 为什么需要 borrowed 和 owned 两个版本?

```rust
// 场景 1: JSON,数据已经在内存中完整解析
// Deserializer 可以分配 String 后传递给 visit_string

// 场景 2: 零拷贝 JSON (simd-json)
// Deserializer 直接从原始字节缓冲区借用 &str
// 传递给 visit_borrowed_str

// str visitor 模式:
impl<'de> Visitor<'de> for StrVisitor {
    type Value = &'de str;  // 零拷贝!

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<&'de str, E> {
        Ok(v)  // 直接返回借用,零分配
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<&'de str, E> {
        Err(Error::custom("cannot borrow from deserializer"))
    }

    fn visit_string<E: Error>(self, v: String) -> Result<&'de str, E> {
        Err(Error::custom("cannot borrow String, need &str"))
    }
}

// String visitor 模式:
impl Visitor<'_> for StringVisitor {
    type Value = String;

    fn visit_string<E>(self, v: String) -> Result<String, E> {
        Ok(v)  // 直接取所有权
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<String, E> {
        Ok(v.to_owned())  // 分配
    }

    fn visit_borrowed_str<E: Error>(self, v: &str) -> Result<String, E> {
        Ok(v.to_owned())  // 分配
    }
}
```

## 复合类型的 Visitor 实现

### visit_seq: 逐个读入序列元素

```rust
fn visit_seq<A>(self, mut seq: A) -> Result<Vec<T>, A::Error>
where
    A: SeqAccess<'de>,
    T: Deserialize<'de>,
{
    let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or(0));
    while let Some(elem) = seq.next_element()? {
        vec.push(elem);
    }
    Ok(vec)
}
```

### visit_map: 逐个读入键值对

```rust
fn visit_map<A>(self, mut map: A) -> Result<HashMap<K, V>, A::Error>
where
    A: MapAccess<'de>,
    K: Deserialize<'de> + Eq + Hash,
    V: Deserialize<'de>,
{
    let mut h = HashMap::with_capacity(map.size_hint().unwrap_or(0));
    while let Some((key, value)) = map.next_entry()? {
        h.insert(key, value);
    }
    Ok(h)
}
```

## 辅助 trait 详解

### SeqAccess

```rust
pub trait SeqAccess<'de> {
    type Error: Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where T: DeserializeSeed<'de>;

    fn next_element<T: Deserialize<'de>>(&mut self) -> Result<Option<T>, Self::Error> {
        self.next_element_seed(PhantomData)
    }

    fn size_hint(&self) -> Option<usize> { None }
}
```

### MapAccess

```rust
pub trait MapAccess<'de> {
    type Error: Error;
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where K: DeserializeSeed<'de>;
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where V: DeserializeSeed<'de>;

    fn next_entry<K, V>(&mut self) -> Result<Option<(K, V)>, Self::Error>
    where K: Deserialize<'de>, V: Deserialize<'de> { /* ... */ }

    fn size_hint(&self) -> Option<usize> { None }
}
```

### EnumAccess

```rust
pub trait EnumAccess<'de>: Sized {
    type Error: Error;
    type Variant: VariantAccess<'de, Error = Self::Error>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where V: DeserializeSeed<'de>;
}
```

### VariantAccess

```rust
pub trait VariantAccess<'de>: Sized {
    type Error: Error;
    fn unit_variant(self) -> Result<(), Self::Error>;
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where T: DeserializeSeed<'de>;
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where V: Visitor<'de>;
    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V)
        -> Result<V::Value, Self::Error>
    where V: Visitor<'de>;
}
```

## Unexpected 和 Error 类型

```rust
// 源码: serde_core/src/de/mod.rs:338
// 用于错误消息,描述"实际收到了什么类型"
pub enum Unexpected<'a> {
    Bool(bool),
    Unsigned(u64), Signed(i64), Float(f64),
    Char(char), Str(&'a str), Bytes(&'a [u8]),
    Unit, Option, NewtypeStruct,
    Seq, Map, Enum,
    UnitVariant, NewtypeVariant, TupleVariant, StructVariant,
    Other(&'a dyn fmt::Display),
}

// de::Error 方法
pub trait Error: Debug + Display {
    fn custom<T: Display>(msg: T) -> Self;
    fn invalid_type(unexp: Unexpected, exp: &dyn Expected) -> Self;
    fn invalid_value(unexp: Unexpected, exp: &dyn Expected) -> Self;
    fn invalid_length(len: usize, exp: &dyn Expected) -> Self;
    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self;
    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self;
    fn missing_field(field: &'static str) -> Self;
    fn duplicate_field(field: &'static str) -> Self;
}
```

---

**练习**:
1. 为自定义类型实现 `Visitor`(选择 `visit_seq` + `visit_map` 两种策略)
2. 阅读 `serde_core/src/de/value.rs`,理解 `SeqDeserializer` 如何将 `Iterator` 包装为 `Deserializer`
3. 研究 `IgnoredAny` 的实现 (`serde_core/src/de/ignored_any.rs`),理解如何高效丢弃数据
