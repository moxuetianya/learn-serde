# 第五章: Deserialize —— 从数据模型重建 Rust 类型

**源码参考**: `serde_core/src/de/mod.rs:554` 和 `serde_core/src/de/impls.rs`

## Deserialize Trait

```rust
// 源码: serde_core/src/de/mod.rs:554
pub trait Deserialize<'de>: Sized {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;

    // 可选: 原地反序列化(用于优化,如 Vec::deserialize_in_place)
    fn deserialize_in_place<D>(
        deserializer: D,
        place: &mut Self,
    ) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        // 默认: 正常反序列化,然后赋值
        *place = Deserialize::deserialize(deserializer)?;
        Ok(())
    }
}
```

**核心原理**: `Deserialize` 需要构造一个 `Visitor`,然后调用 `Deserializer` 上的**恰好一个**方法,将 visitor 传递给 deserializer。

```
数据 → Deserializer.deserialize_* → Visitor.visit_* → Rust 值
```

### 反序列化的两步过程:

```
1. Deserialize::deserialize 选择一个 deserializer 方法
   (告诉 deserializer "我想要什么类型的数据")

2. Deserializer 调用 Visitor 的对应 visit_* 方法
   (告诉 visitor "这里是你想要的数据")
```

> **注意**: 这里的 `D: Deserializer` 不来自 serde 本身 —— derive 只生成类型侧。
> 用 JSON 时 `D` 是 serde_json 的解析器, 用 YAML 是 serde_yaml 的……格式侧必须
> 由格式库提供 (详见第 7 章开头「为什么必须引入 serde_json」)。

### 为什么需要 Visitor?

反序列化比序列化复杂,因为:
1. **数据格式差异**: JSON 的 `42` 可能是 `i8`, `u64`, `f64` 之一 —— Visitor 接受所有可能性
2. **零拷贝**: `&str` 可能需要借用 deserializer 内部缓冲区 —— Visitor 带生命周期
3. **错误处理**: 期望 `u32` 却得到负数 —— Visitor 的 `expecting()` 提供友好错误

## 手工实现 Deserialize

### 示例 1: 基本类型

```rust
// 源码: serde_core/src/de/impls.rs:380
impl<'de> Deserialize<'de> for bool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 创建一个 Visitor 并告诉 deserializer "我要 bool"
        deserializer.deserialize_bool(BoolVisitor)
    }
}

// Visitor 定义
struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a boolean")
    }

    fn visit_bool(self, v: bool) -> Result<bool, E> {
        Ok(v)
    }
}
```

### 示例 2: 结构体(通过 StructVisitor)

```rust
struct Duration {
    secs: u64,
    nanos: u32,
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // FIELDS 是字段名列表
        const FIELDS: &[&str] = &["secs", "nanos"];
        deserializer.deserialize_struct("Duration", FIELDS, DurationVisitor)
    }
}

struct DurationVisitor;

impl<'de> Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Duration")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Duration, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut secs = None;
        let mut nanos = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "secs" => {
                    if secs.is_some() {
                        return Err(de::Error::duplicate_field("secs"));
                    }
                    secs = Some(map.next_value()?);
                }
                "nanos" => {
                    if nanos.is_some() {
                        return Err(de::Error::duplicate_field("nanos"));
                    }
                    nanos = Some(map.next_value()?);
                }
                _ => {
                    // 忽略未知字段(或返回错误)
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let secs = secs.ok_or_else(|| de::Error::missing_field("secs"))?;
        let nanos = nanos.ok_or_else(|| de::Error::missing_field("nanos"))?;
        Ok(Duration { secs, nanos })
    }
}
```

### 示例 3: 枚举

```rust
enum Command {
    Login { user: String, pass: String },
    Logout,
    Message(String),
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &["Login", "Logout", "Message"];
        deserializer.deserialize_enum("Command", VARIANTS, CommandVisitor)
    }
}

struct CommandVisitor;

impl<'de> Visitor<'de> for CommandVisitor {
    type Value = Command;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("enum Command")
    }

    fn visit_enum<A>(self, data: A) -> Result<Command, A::Error>
    where
        A: EnumAccess<'de>,
    {
        // data 提供 variant() 方法,返回 (variant_name, VariantAccess)
        use serde::de::VariantAccess;

        match data.variant()? {
            ("Login", variant) => {
                // struct variant: 用 struct_variant() 拿到 MapAccess
                let (user, pass) = variant.struct_variant(
                    &["user", "pass"],
                    LoginVisitor
                )?;
                Ok(Command::Login { user, pass })
            }
            ("Logout", variant) => {
                // unit variant
                variant.unit_variant()?;
                Ok(Command::Logout)
            }
            ("Message", variant) => {
                // newtype variant
                Ok(Command::Message(variant.newtype_variant()?))
            }
            (unknown, _) => {
                Err(de::Error::unknown_variant(unknown, &["Login", "Logout", "Message"]))
            }
        }
    }
}
```

## Deserialize 实现策略

### 对于简单类型

```
impl Deserialize for i32
  → deserializer.deserialize_i32(I32Visitor)

impl Deserialize for String
  → deserializer.deserialize_string(StringVisitor)
  // StringVisitor 处理 visit_string(owned) 和 visit_str(borrowed → to_owned)
```

### 对于"灵活接受"的类型

```rust
// Option<T> 的实现
// 源码: serde_core/src/de/impls.rs ~670
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Option<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionVisitor::<T>(PhantomData))
    }
}

// PhantomData<T> 允许不持有 T 的情况下实现带有 T 的 Visitor
struct OptionVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for OptionVisitor<T> {
    type Value = Option<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("option")
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Some)
    }

    fn visit_none<E>(self) -> Result<Option<T>, E> {
        Ok(None)
    }

    // Optional: visit_unit 也产生 None (JSON null)
    fn visit_unit<E>(self) -> Result<Option<T>, E> {
        Ok(None)
    }
}
```

## DeserializeSeed —— 带状态的 Deserialize

```rust
// 源码: serde_core/src/de/mod.rs:803
pub trait DeserializeSeed<'de>: Sized {
    type Value;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>;
}

// PhantomData 的桥接实现: 将 DeserializeSeed 转换为 Deserialize
impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for PhantomData<T> {
    type Value = T;
    fn deserialize<D>(self, deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer)
    }
}
```

`DeserializeSeed` 用于需要**运行时上下文**的场景:
- `MapAccess::next_key_seed` —— key 类型在运行时确定
- `EnumAccess::variant_seed` —— variant 选择影响后续的类型

---

**练习**:
1. 阅读 `serde_core/src/de/impls.rs` 中 `Option<T>` 和 `Vec<T>` 的 `Deserialize` 实现
2. 手工为 `enum Status { Online, Offline, Custom(u16) }` 实现 `Deserialize`
3. 追踪: `serde_json::from_str::<i32>("42")` 的完整调用链
