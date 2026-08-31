/// 第二十一章专题 demo ①: struct —— deserialize_struct → visit_map / visit_seq
///
/// 运行: cargo run --example 09_access_struct
///
/// 这是「复合类型访问器」系列的第一个示例 (struct / map / enum / 嵌套):
///   - 09_access_struct: struct 两种入口 (MapAccess / SeqAccess)
///   - 10_access_map:    map 任意键 (MapAccess)
///   - 11_access_enum:   enum 四种 variant 形状 (EnumAccess / VariantAccess)
///   - 12_access_nested: 复合类型嵌套的递归调用链
///
/// 核心问题: MapAccess / SeqAccess 怎么用?
/// 一句话答案: 它们是「格式方实现、类型方消费」的协议对象 ——
///   手写 Deserialize 的 Visitor 方法收到它们, 按约定方法轮询数据,
///   这套代码正是 derive 帮你生成的。
///
/// 本 demo 专注「类型侧怎么用」(即 derive 生成的代码长什么样)。
/// 「格式侧怎么实现」见 04_custom_serializer / 05_custom_deserializer / 08_why_visitor。

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeStruct, SerializeTupleStruct};
use serde::{Serialize, Serializer};
use std::fmt;

// ============================================================
// 1. struct 反序列化调用链
// ============================================================
// JSON 对象 {"x": 1.5, "y": -2.0}:
//
//   Point::deserialize(d)
//     └─ d.deserialize_struct("Point", FIELDS, PointVisitor)  格式方知道"这是一个结构体"
//         └─ visitor.visit_map(MapAccess)                    JSON 对象 → MapAccess
//             ├─ next_key::<PointField>()  消费 "x" 键       ← 键是标识符!
//             ├─ next_value::<f64>()       消费 1.5
//             ├─ next_key::<PointField>()  消费 "y" 键
//             ├─ next_value::<f64>()       消费 -2.0
//             └─ next_key() → None                           ← 返回 None 即结束
// 或者 JSON 数组 [1.5, -2.0]:
//     └─ visitor.visit_seq(SeqAccess)                        JSON 数组 → SeqAccess
//         ├─ next_element()? 消费 1.5
//         ├─ next_element()? 消费 -2.0
//         └─ next_element()? → None
//
// 注意: visit_map / visit_seq 是同一个 Visitor 的两个入口,
//       格式方根据输入形状选一个调用, 你只需要两个都实现。

#[derive(Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

const FIELDS: &[&str] = &["x", "y"];

impl<'de> Deserialize<'de> for Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize_struct 的三个参数: 结构体名、字段名列表、Visitor
        // 字段名列表会出现在错误信息里, 也给格式方一个提前检查的机会
        deserializer.deserialize_struct("Point", FIELDS, PointVisitor)
    }
}

// ---- 字段标识符枚举 (derive 生成的 __Field 就是这种东西) ----
// 为什么要它? struct 的键是编译期固定的几个字符串,
// 预先把它枚举出来, 可以:
//   1. 用枚举匹配代替字符串比较 (编译器帮你写 match 分支)
//   2. 未知字段直接生成 unknown_field 错误
//   3. 让 bincode 等格式可以用数字下标代替字段名 (visit_u64)
#[derive(Debug)]
enum PointField {
    X,
    Y,
}

impl<'de> Deserialize<'de> for PointField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PointFieldVisitor;

        impl<'de> Visitor<'de> for PointFieldVisitor {
            type Value = PointField;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("field identifier `x` or `y`")
            }

            // JSON 里字段名一定是字符串 → visit_str
            // (derive 还会生成 visit_bytes / visit_u64, 服务其他格式)
            fn visit_str<E: de::Error>(self, v: &str) -> Result<PointField, E> {
                match v {
                    "x" => Ok(PointField::X),
                    "y" => Ok(PointField::Y),
                    other => Err(de::Error::unknown_field(other, FIELDS)),
                }
            }
        }

        // 键是用 deserialize_identifier 反序列化的 —— 给"字段名/变体名"
        // 预留的专门入口, 有些格式(如 JSON)把它当普通字符串处理
        deserializer.deserialize_identifier(PointFieldVisitor)
    }
}

struct PointVisitor;

impl<'de> Visitor<'de> for PointVisitor {
    type Value = Point;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Point")
    }

    // ---- 入口 A: 对象形状 {"x": ..., "y": ...} ----
    fn visit_map<A>(self, mut map: A) -> Result<Point, A::Error>
    where
        A: MapAccess<'de>,
    {
        // 先收集到 Option, 最后统一检查缺失 —— 因为键的出现顺序不确定
        let mut x = None;
        let mut y = None;

        // 协议: next_key 和 next_value 必须严格交替调用!
        //   next_key 消费一个键 → 返回 Some(键) 或 None(耗尽)
        //   next_value 消费这个键对应的值 (key 未消费时不能调用!)
        while let Some(key) = map.next_key::<PointField>()? {
            match key {
                PointField::X => {
                    if x.is_some() {
                        return Err(de::Error::duplicate_field("x"));
                    }
                    x = Some(map.next_value()?);
                }
                PointField::Y => {
                    if y.is_some() {
                        return Err(de::Error::duplicate_field("y"));
                    }
                    y = Some(map.next_value()?);
                }
            }
        }

        let x = x.ok_or_else(|| de::Error::missing_field("x"))?;
        let y = y.ok_or_else(|| de::Error::missing_field("y"))?;
        Ok(Point { x, y })
    }

    // ---- 入口 B: 数组形状 [x, y] (bincode/CSV 等无键格式走这里) ----
    fn visit_seq<A>(self, mut seq: A) -> Result<Point, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // next_element 返回 None 表示序列结束;
        // 用 invalid_length 指明"第几个位置缺了"
        let x = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let y = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        Ok(Point { x, y })
    }
}

// ============================================================
// 2. struct 序列化: serialize_struct → SerializeStruct
// ============================================================
// 协议: serialize_field(key, value) 任意次数 → end()
impl Serialize for Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Point", 2)?;
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        state.end()
    }
}

// ============================================================
// 3. 顺带: 命名元组 struct Point2(f64, f64)
//    序列化走 serialize_tuple_struct → SerializeTupleStruct (无字段名)
// ============================================================
#[derive(Debug, PartialEq)]
struct Point2(f64, f64);

impl Serialize for Point2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_tuple_struct("Point2", 2)?;
        state.serialize_field(&self.0)?;
        state.serialize_field(&self.1)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Point2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 无键的 tuple struct 只能走 visit_seq —— 格式方也只知道按位置给
        deserializer.deserialize_tuple_struct("Point2", 2, Point2Visitor)
    }
}

struct Point2Visitor;

impl<'de> Visitor<'de> for Point2Visitor {
    type Value = Point2;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("tuple struct Point2")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Point2, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let a = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let b = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        Ok(Point2(a, b))
    }
}

// ============================================================
// 4. 运行
// ============================================================
fn main() {
    println!("=== 1. struct Point: visit_map / visit_seq 双入口 ===");
    let p: Point = serde_json::from_str(r#"{"x": 1.5, "y": -2.0}"#).unwrap();
    println!("  对象 {{x,y}}   → {p:?}");
    let p2: Point = serde_json::from_str("[1.5, -2.0]").unwrap();
    println!("  数组 [x,y]    → {p2:?}");
    println!("  序列化         → {}", serde_json::to_string(&p).unwrap());

    println!("\n  --- 三种错误场景 ---");
    println!("  缺字段 y       → {}", serde_json::from_str::<Point>(r#"{"x": 1.5}"#).unwrap_err());
    println!("  重复字段 x     → {}", serde_json::from_str::<Point>(r#"{"x": 1.0, "x": 2.0, "y": 3.0}"#).unwrap_err());
    println!("  未知字段 z     → {}", serde_json::from_str::<Point>(r#"{"x": 1.0, "z": 9.0, "y": 3.0}"#).unwrap_err());

    println!("\n=== 2. tuple struct Point2: 只有 visit_seq ===");
    let t: Point2 = serde_json::from_str("[1.5, -2.0]").unwrap();
    println!("  [x, y]        → {t:?}");
    println!("  序列化         → {}", serde_json::to_string(&t).unwrap());
    println!("  对象形状会失败  → {}", serde_json::from_str::<Point2>(r#"{"a": 1.0, "b": 2.0}"#).unwrap_err());
}
