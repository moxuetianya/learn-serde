/// 第四章/第五章 demo: 手工实现 Deserialize
///
/// 运行: cargo run --example 03_hand_impl_deserialize
///
/// 手工实现 Deserialize 需要:
/// 1. 实现 Deserialize trait
/// 2. 创建一个 Visitor(实现 Visitor trait)
/// 3. 在 deserialize 中调用 Deserializer 的恰好一个方法,传入 visitor

use serde::de::{self, Error, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

// ============================================================
// 示例 1: Duration 的反序列化
// ============================================================
#[derive(Debug, PartialEq)]
struct Duration {
    secs: u64,
    nanos: u32,
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["secs", "nanos"];

        // deserialize_struct 传入字段名列表和 Visitor
        deserializer.deserialize_struct("Duration", FIELDS, DurationVisitor)
    }
}

struct DurationVisitor;

impl<'de> Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Duration")
    }

    // 从 map 反序列化: {"secs": 3600, "nanos": 500000000}
    fn visit_map<A>(self, mut map: A) -> Result<Duration, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut secs = None;
        let mut nanos = None;

        // 遍历所有 key-value 对
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "secs" => {
                    if secs.is_some() {
                        return Err(Error::duplicate_field("secs"));
                    }
                    secs = Some(map.next_value()?);
                }
                "nanos" => {
                    if nanos.is_some() {
                        return Err(Error::duplicate_field("nanos"));
                    }
                    nanos = Some(map.next_value()?);
                }
                other => {
                    // 忽略未知字段(或用 IgnoredAny 跳过)
                    println!("  Warning: ignoring unknown field '{}'", other);
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let secs = secs.ok_or_else(|| Error::missing_field("secs"))?;
        let nanos = nanos.ok_or_else(|| Error::missing_field("nanos"))?;
        Ok(Duration { secs, nanos })
    }

    // 从序列反序列化: [3600, 500000000]
    fn visit_seq<A>(self, mut seq: A) -> Result<Duration, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let secs = seq.next_element()?
            .ok_or_else(|| Error::invalid_length(0, &self))?;
        let nanos = seq.next_element()?
            .ok_or_else(|| Error::invalid_length(1, &self))?;
        Ok(Duration { secs, nanos })
    }
}

// ============================================================
// 示例 2: 选项类型(手工实现类似 Option 的反序列化)
// ============================================================
#[derive(Debug, PartialEq)]
struct Maybe<T>(Option<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Maybe<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 使用 deserialize_option
        deserializer.deserialize_option(MaybeVisitor::<T>(std::marker::PhantomData))
    }
}

// PhantomData 允许我们的 visitor 携带类型信息而不持有值
struct MaybeVisitor<T>(std::marker::PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for MaybeVisitor<T> {
    type Value = Maybe<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an optional value")
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Maybe<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| Maybe(Some(v)))
    }

    fn visit_none<E>(self) -> Result<Maybe<T>, E> {
        Ok(Maybe(None))
    }

    // 将 JSON null 也视为 None
    fn visit_unit<E>(self) -> Result<Maybe<T>, E> {
        Ok(Maybe(None))
    }
}

// ============================================================
// 示例 3: 灵活接受多种输入的类型
// ============================================================
#[derive(Debug, PartialEq)]
struct FlexibleBool(bool);

impl<'de> Deserialize<'de> for FlexibleBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FlexibleBoolVisitor)
    }
}

struct FlexibleBoolVisitor;

impl<'de> Visitor<'de> for FlexibleBoolVisitor {
    type Value = FlexibleBool;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a boolean, or 0/1, or 'true'/'false'")
    }

    fn visit_bool<E>(self, v: bool) -> Result<FlexibleBool, E> {
        Ok(FlexibleBool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<FlexibleBool, E> {
        Ok(FlexibleBool(v != 0))
    }

    fn visit_u64<E>(self, v: u64) -> Result<FlexibleBool, E> {
        Ok(FlexibleBool(v != 0))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<FlexibleBool, E> {
        match v.to_lowercase().as_str() {
            "true" | "yes" | "on" | "1" => Ok(FlexibleBool(true)),
            "false" | "no" | "off" | "0" => Ok(FlexibleBool(false)),
            _ => Err(E::custom(format!("cannot parse '{}' as bool", v))),
        }
    }
}

// ============================================================
// 示例 4: 实现 EnumAccess 的 Visitor(枚举反序列化)
// ============================================================
#[derive(Debug, PartialEq)]
enum SimpleCommand {
    Start,
    Stop { reason: String },
}

impl<'de> Deserialize<'de> for SimpleCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &["Start", "Stop"];
        deserializer.deserialize_enum("SimpleCommand", VARIANTS, SimpleCommandVisitor)
    }
}

struct SimpleCommandVisitor;

impl<'de> Visitor<'de> for SimpleCommandVisitor {
    type Value = SimpleCommand;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("enum SimpleCommand")
    }

    fn visit_enum<A>(self, data: A) -> Result<SimpleCommand, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        // variant() 返回 (variant_name, variant_data)
        // 变体名可以是 &str 也可以是其他类型
        let (variant_name, variant_data) = data.variant::<String>()?;

        match variant_name.as_str() {
            "Start" => {
                // unit variant: 无数据
                variant_data.unit_variant()?;
                Ok(SimpleCommand::Start)
            }
            "Stop" => {
                // struct variant: 有命名字段
                let reason = variant_data.struct_variant(
                    &["reason"],
                    StopReasonVisitor,
                )?;
                Ok(SimpleCommand::Stop { reason })
            }
            unknown => Err(Error::unknown_variant(unknown, &["Start", "Stop"])),
        }
    }
}

// Stop variant 的字段 visitor
struct StopReasonVisitor;

impl<'de> Visitor<'de> for StopReasonVisitor {
    type Value = String;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct with 'reason' field")
    }

    fn visit_map<A>(self, mut map: A) -> Result<String, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut reason = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == "reason" {
                reason = Some(map.next_value()?);
            } else {
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }
        reason.ok_or_else(|| Error::missing_field("reason"))
    }
}

// ============================================================
// 测试
// ============================================================
fn main() {
    println!("=== 1. Duration deserialize ===");
    let json = r#"{"secs": 3600, "nanos": 500000000}"#;
    let d: Duration = serde_json::from_str(json).unwrap();
    println!("  from map: {:?}", d);

    let json = r#"[7200, 250000000]"#;
    let d2: Duration = serde_json::from_str(json).unwrap();
    println!("  from seq: {:?}", d2);

    println!("\n=== 2. Maybe<T> (option) ===");
    let m: Maybe<i32> = serde_json::from_str("42").unwrap();
    println!("  Some: {:?}", m);
    let m: Maybe<i32> = serde_json::from_str("null").unwrap();
    println!("  None: {:?}", m);

    println!("\n=== 3. FlexibleBool ===");
    for input in &["true", "false", "1", "0", "yes", "no", "on", "off"] {
        let fb: FlexibleBool = serde_json::from_str(input).unwrap();
        println!("  '{}' → {:?}", input, fb);
    }

    println!("\n=== 4. SimpleCommand enum ===");
    let json = r#"{"Start": null}"#;
    let cmd: SimpleCommand = serde_json::from_str(json).unwrap();
    println!("  Start: {:?}", cmd);

    let json = r#"{"Stop": {"reason": "timeout"}}"#;
    let cmd: SimpleCommand = serde_json::from_str(json).unwrap();
    println!("  Stop: {:?}", cmd);

    println!("\n=== All demos passed! ===");
}
