/// 第二十一章专题 demo ③: enum —— deserialize_enum → visit_enum(EnumAccess) → VariantAccess
///
/// 运行: cargo run --example 11_access_enum
///
/// 这是「复合类型访问器」系列示例之三 (struct / map / enum / 嵌套):
///   - 09_access_struct: struct 两种入口 (MapAccess / SeqAccess)
///   - 10_access_map:    map 任意键 (MapAccess)
///   - 11_access_enum:   enum 四种 variant 形状 (EnumAccess / VariantAccess)
///   - 12_access_nested: 复合类型嵌套的递归调用链
///
/// 枚举反序列化分两步:
///   ① 读变体名:     data.variant::<EventField>()  → (变体标识, VariantAccess)
///      —— 变体名 token 在这里被消费一次, 之后无法回头 (匹配即提交!)
///   ② 读载荷:       根据变体形状选 VariantAccess 的四个方法之一
///
/// 四种 variant 形状 ↔ VariantAccess 四个方法:
///   Quit                     → unit_variant()
///   Write(String)            → newtype_variant()
///   ChangeColor(i32,i32,i32) → tuple_variant(len, TupleVisitor)
///   Move { x, y }            → struct_variant(&["x","y"], StructVisitor)
///
/// 序列化侧镜像:
///   Quit                     → serialize_unit_variant(name, idx, variant)
///   Write(String)            → serialize_newtype_variant(name, idx, variant, value)
///   ChangeColor(...)         → serialize_tuple_variant(name, idx, variant, len, ...)
///   Move { x, y }            → serialize_struct_variant(name, idx, variant, len, ...)
///
/// 外部标签策略的 token 流 (JSON {"Move": {"x": 10, "y": 20}}):
///   MapStart → Str("Move") → MapStart → Str("x") → I64(10) → Str("y") → I64(20)
///   → MapEnd → MapEnd
///   visit_enum → variant() 消费 Str("Move")
///              → struct_variant 消费内层 MapStart..MapEnd (又是 visit_map 交替)

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeStructVariant, SerializeTupleVariant};
use serde::{Serialize, Serializer};
use std::fmt;

// ============================================================
// 1. 类型定义 + 变体标识符枚举
// ============================================================

#[derive(Debug, PartialEq)]
enum Event {
    Quit,
    Write(String),
    ChangeColor(i32, i32, i32),
    Move { x: i32, y: i32 },
}

const VARIANTS: &[&str] = &["Quit", "Write", "ChangeColor", "Move"];

// 变体标识符枚举 —— 和 struct 的 __Field 同一套路 (见 09_access_struct),
// 只是未知名字报 unknown_variant 而不是 unknown_field
#[derive(Debug)]
enum EventField {
    Quit,
    Write,
    ChangeColor,
    Move,
}

impl<'de> Deserialize<'de> for EventField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EventFieldVisitor;

        impl<'de> Visitor<'de> for EventFieldVisitor {
            type Value = EventField;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("variant identifier")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<EventField, E> {
                match v {
                    "Quit" => Ok(EventField::Quit),
                    "Write" => Ok(EventField::Write),
                    "ChangeColor" => Ok(EventField::ChangeColor),
                    "Move" => Ok(EventField::Move),
                    other => Err(de::Error::unknown_variant(other, VARIANTS)),
                }
            }
        }

        deserializer.deserialize_identifier(EventFieldVisitor)
    }
}

// ============================================================
// 2. 反序列化: deserialize_enum → visit_enum
// ============================================================

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Event", VARIANTS, EventVisitor)
    }
}

struct EventVisitor;

impl<'de> Visitor<'de> for EventVisitor {
    type Value = Event;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("enum Event")
    }

    fn visit_enum<A>(self, data: A) -> Result<Event, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        use de::VariantAccess;

        // ① 消费变体名 token —— 整个枚举反序列化只有这一次"读名字"的机会
        let (variant, access) = data.variant::<EventField>()?;
        println!("    variant() 消费变体名 → {variant:?}");

        // ② 按形状分发载荷。注意: 一旦分支匹配, 载荷失败也不会回退
        match variant {
            EventField::Quit => {
                access.unit_variant()?;
                Ok(Event::Quit)
            }
            EventField::Write => {
                println!("    newtype_variant() 读载荷");
                Ok(Event::Write(access.newtype_variant()?))
            }
            EventField::ChangeColor => {
                // tuple_variant 要求格式方按长度依次吐出元素,
                // 元素形状由 ChangeColorVisitor 描述
                let (r, g, b) = access.tuple_variant(3, ChangeColorVisitor)?;
                Ok(Event::ChangeColor(r, g, b))
            }
            EventField::Move => {
                // struct_variant 的载荷又是一个 map, 需要再套一个 Visitor
                // (这里字段直接按字符串匹配; derive 会再生成一个 __Field 枚举)
                let (x, y) = access.struct_variant(&["x", "y"], MoveVisitor)?;
                Ok(Event::Move { x, y })
            }
        }
    }
}

// ---- tuple variant 的载荷 Visitor: visit_seq ----
struct ChangeColorVisitor;

impl<'de> Visitor<'de> for ChangeColorVisitor {
    type Value = (i32, i32, i32);

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("3-tuple of (r, g, b)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<(i32, i32, i32), A::Error>
    where
        A: SeqAccess<'de>,
    {
        let r = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let g = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let b = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
        Ok((r, g, b))
    }
}

// ---- struct variant 的载荷 Visitor: visit_map ----
struct MoveVisitor;

impl<'de> Visitor<'de> for MoveVisitor {
    type Value = (i32, i32);

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Move { x, y }")
    }

    fn visit_map<A>(self, mut map: A) -> Result<(i32, i32), A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut x = None;
        let mut y = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "x" => {
                    if x.is_some() {
                        return Err(de::Error::duplicate_field("x"));
                    }
                    x = Some(map.next_value()?);
                }
                "y" => {
                    if y.is_some() {
                        return Err(de::Error::duplicate_field("y"));
                    }
                    y = Some(map.next_value()?);
                }
                // 未知字段必须先消费值再跳过, 否则键值对会错位
                other => {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                    eprintln!("  (ignored unknown field {other:?})");
                }
            }
        }
        let x = x.ok_or_else(|| de::Error::missing_field("x"))?;
        let y = y.ok_or_else(|| de::Error::missing_field("y"))?;
        Ok((x, y))
    }
}

// ============================================================
// 3. 序列化: 四个 serialize_*_variant
// ============================================================

impl Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 注意每个方法的参数: (类型名, 变体下标, 变体名, [长度/载荷])
        match self {
            Event::Quit => serializer.serialize_unit_variant("Event", 0, "Quit"),
            Event::Write(s) => serializer.serialize_newtype_variant("Event", 1, "Write", s),
            Event::ChangeColor(r, g, b) => {
                let mut tv = serializer.serialize_tuple_variant("Event", 2, "ChangeColor", 3)?;
                tv.serialize_field(r)?;
                tv.serialize_field(g)?;
                tv.serialize_field(b)?;
                tv.end()
            }
            Event::Move { x, y } => {
                let mut sv = serializer.serialize_struct_variant("Event", 3, "Move", 2)?;
                sv.serialize_field("x", x)?;
                sv.serialize_field("y", y)?;
                sv.end()
            }
        }
    }
}

// ============================================================
// 4. 运行
// ============================================================
fn main() {
    println!("=== enum Event: 四种 variant 形状 ===");
    let cases = [
        (r#"{"Quit": null}"#, "unit"),
        (r#"{"Write": "hello"}"#, "newtype"),
        (r#"{"ChangeColor": [1, 2, 3]}"#, "tuple"),
        (r#"{"Move": {"x": 10, "y": 20}}"#, "struct"),
    ];
    for (json, shape) in cases {
        let e: Event = serde_json::from_str(json).unwrap();
        println!("  {json:<30} {shape:<8} → {e:?}");
        println!("  {:>30}        → {}", "", serde_json::to_string(&e).unwrap());
    }

    println!("\n  --- 错误场景 ---");
    println!("  未知变体 Boom    → {}", serde_json::from_str::<Event>(r#"{"Boom": null}"#).unwrap_err());
    println!("  载荷错型(Write 收数字) → {}", serde_json::from_str::<Event>(r#"{"Write": 42}"#).unwrap_err());
    println!("  Move 缺字段 y    → {}", serde_json::from_str::<Event>(r#"{"Move": {"x": 1}}"#).unwrap_err());

    println!("\n  --- 调用链追踪 (看 variant() 只被调用一次) ---");
    let _: Event = serde_json::from_str(r#"{"Move": {"x": 10, "y": 20}}"#).unwrap();
}
