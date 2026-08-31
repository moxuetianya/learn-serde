/// 第二十一章专题 demo ②: map —— deserialize_map → visit_map (任意键)
///
/// 运行: cargo run --example 10_access_map
///
/// 这是「复合类型访问器」系列示例之二 (struct / map / enum / 嵌套):
///   - 09_access_struct: struct 两种入口 (MapAccess / SeqAccess)
///   - 10_access_map:    map 任意键 (MapAccess)
///   - 11_access_enum:   enum 四种 variant 形状 (EnumAccess / VariantAccess)
///   - 12_access_nested: 复合类型嵌套的递归调用链
///
/// 与 struct 的区别:
///   struct: 键是编译期固定的字段名 → 用 PointField 标识符枚举匹配
///   map:    键是任意类型! (这里用 i32 做键, 强调"任意")
///   但格式侧 JSON 都叫对象, 所以两者都落到 visit_map ——
///   visit_map 是同一个 MapAccess, 只是 next_key 的类型不同!
///
/// {"10": "a", "20": "b"} 的 token 流:
///   MapStart, I64(10), Str("a"), I64(20), Str("b"), MapEnd
///   next_key()   消费 I64(10)  → Some(10)
///   next_value() 消费 Str("a") → "a"
///   next_key()   消费 I64(20)  → Some(20)
///   next_value() 消费 Str("b") → "b"
///   next_key()   已耗尽        → None
///
/// 协议: next_key / next_value 必须严格交替; 没消费的值留在 token 流里,
///       会把后面的键值对全部挤错位。

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

// ============================================================
// 1. map 反序列化: deserialize_map → visit_map → next_entry
// ============================================================
// 注意: 这里用 deserialize_map (不是 deserialize_struct),
// 区别只在键的"身份": map 的键是数据, struct 的键是字段名。

#[derive(Debug, PartialEq)]
struct ScoreTable(BTreeMap<i32, String>);

impl<'de> Deserialize<'de> for ScoreTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ScoreTableVisitor)
    }
}

struct ScoreTableVisitor;

impl<'de> Visitor<'de> for ScoreTableVisitor {
    type Value = ScoreTable;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map of integer scores to names")
    }

    fn visit_map<A>(self, mut map: A) -> Result<ScoreTable, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut scores = BTreeMap::new();

        // next_entry = next_key + next_value 的组合便捷方法,
        // 键和值各写各的类型 (键 i32 值 String)
        while let Some((score, name)) = map.next_entry()? {
            // 注意: JSON 里键是字符串 "10", 这里被解析成 i32 10 ——
            // 这就是"键是任意类型"的含义
            println!("    next_entry → key {score:?}, value {name:?}");
            scores.insert(score, name);
        }

        Ok(ScoreTable(scores))
    }
}

// ============================================================
// 2. map 序列化: serialize_map → SerializeMap
// ============================================================
// 协议: serialize_key → serialize_value → ... → end()
//       键值必须成对! (serialize_entry 是两者组合)
impl Serialize for ScoreTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 第一个参数是元素个数提示 (Some(len)); 不知道就传 None
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (score, name) in &self.0 {
            map.serialize_entry(score, name)?;
        }
        map.end()
    }
}

// ============================================================
// 3. 顺带: Vec<T> 用的其实是同一套 MapAccess 家族的邻居 SeqAccess
//    这里手写一个 Vec 的序列化/反序列化, 展示 seq 家族
// ============================================================
#[derive(Debug, PartialEq)]
struct RankList(Vec<i32>);

impl Serialize for RankList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for v in &self.0 {
            seq.serialize_element(v)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for RankList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(RankListVisitor)
    }
}

struct RankListVisitor;

impl<'de> Visitor<'de> for RankListVisitor {
    type Value = RankList;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a sequence of integers")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<RankList, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut v = Vec::new();
        // next_element 返回 None 即序列耗尽
        while let Some(x) = seq.next_element()? {
            v.push(x);
        }
        Ok(RankList(v))
    }
}

// ============================================================
// 4. 运行
// ============================================================
fn main() {
    println!("=== 1. map ScoreTable: 任意键类型 (i32 键) ===");
    let s: ScoreTable = serde_json::from_str(r#"{"10": "alice", "20": "bob"}"#).unwrap();
    println!("  {{10:a,20:b}}  → {s:?}");
    println!("  序列化         → {}", serde_json::to_string(&s).unwrap());
    println!("  键不是整数会失败 → {}", serde_json::from_str::<ScoreTable>(r#"{"x": "a"}"#).unwrap_err());

    println!("\n=== 2. seq RankList: 序列家族 (SeqAccess) ===");
    let r: RankList = serde_json::from_str("[3, 1, 4, 1, 5]").unwrap();
    println!("  [3,1,4,1,5]   → {r:?}");
    println!("  序列化         → {}", serde_json::to_string(&r).unwrap());
    println!("  对象形状会失败  → {}", serde_json::from_str::<RankList>(r#"{"a": 1}"#).unwrap_err());
}
