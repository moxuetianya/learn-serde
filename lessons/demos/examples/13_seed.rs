/// 第二十二章专题 demo: DeserializeSeed —— 「种子」(带上下文的 Deserialize)
///
/// 运行: cargo run --example 13_seed
///
/// 一句话: Deserialize 只是 DeserializeSeed<PhantomData<T>> 的特例
/// (源码 serde_core/src/de/mod.rs:1897 next_key → next_key_seed(PhantomData);
///   814 行 PhantomData<T> 在 T: Deserialize 时实现 DeserializeSeed)。
/// 需要往反序列化过程里夹带「上下文」时, 从 Deserialize 升级到 seed:
///
///   A. 版本号驱动    — data 的形状随已读到的 version 变化  → next_value_seed
///   B. 字符串驻留    — 多次解析共享字符串池, 相同键只分配一次
///                       → next_key_seed + 顶层 DeserializeSeed 入口
///   C. 递归深度限制  — 每层嵌套携带 depth, 超限报错
///                       → next_value_seed + next_element_seed (Copy 顺着递归传值)

use serde::de::{self, Deserialize, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

// ============================================================
// 场景 A: 版本号驱动 (next_value_seed)
// ============================================================
// 输入:
//   {"version": 1, "data": {"list": [10, 20]}}   ← v1: data 是对象
//   {"version": 2, "data": [1, 2, 3]}            ← v2: data 是数组
//
// 问题: 「data 是什么形状」要读到 version 之后才知道 ——
// 而 visit_map 循环里 next_value::<Vec<u32>>() 的入口是编译期定死的。
// 解法: 把 version 夹进 seed, 用 next_value_seed 把「选哪个入口」推迟到运行时。

#[derive(Debug, PartialEq)]
struct Config {
    version: u32,
    data: Vec<u32>,
}

const CONFIG_FIELDS: &[&str] = &["version", "data"];

#[derive(Debug)]
enum ConfigField {
    Version,
    Data,
}

impl<'de> Deserialize<'de> for ConfigField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Fv;
        impl<'de> Visitor<'de> for Fv {
            type Value = ConfigField;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("field `version` or `data`")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<ConfigField, E> {
                match v {
                    "version" => Ok(ConfigField::Version),
                    "data" => Ok(ConfigField::Data),
                    other => Err(de::Error::unknown_field(other, CONFIG_FIELDS)),
                }
            }
        }
        deserializer.deserialize_identifier(Fv)
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ConfigVisitor;
        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct Config")
            }
            fn visit_map<A>(self, mut map: A) -> Result<Config, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut version = None;
                let mut data = None;
                while let Some(key) = map.next_key::<ConfigField>()? {
                    match key {
                        ConfigField::Version => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            version = Some(map.next_value()?);
                        }
                        ConfigField::Data => {
                            if data.is_some() {
                                return Err(de::Error::duplicate_field("data"));
                            }
                            // ★ seed: 把已读到的 version 夹带进去 (没读到就用默认 1)
                            let seed = DataSeed {
                                version: version.unwrap_or(1),
                            };
                            data = Some(map.next_value_seed(seed)?);
                        }
                    }
                }
                let version = version.ok_or_else(|| de::Error::missing_field("version"))?;
                let data = data.ok_or_else(|| de::Error::missing_field("data"))?;
                Ok(Config { version, data })
            }
        }
        deserializer.deserialize_struct("Config", CONFIG_FIELDS, ConfigVisitor)
    }
}

// 种子本体: 只携带一个 version, 却决定了 data 的整个形状
struct DataSeed {
    version: u32,
}

impl<'de> DeserializeSeed<'de> for DataSeed {
    type Value = Vec<u32>;

    fn deserialize<D>(self, deserializer: D) -> Result<Vec<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.version >= 2 {
            // v2: data 直接是数组
            deserializer.deserialize_seq(PlainListVisitor)
        } else {
            // v1: data 是 {"list": [...]}
            deserializer.deserialize_struct("data", &["list"], WrappedListVisitor)
        }
    }
}

struct PlainListVisitor;

impl<'de> Visitor<'de> for PlainListVisitor {
    type Value = Vec<u32>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a plain array of numbers")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Vec<u32>, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element()? {
            out.push(v);
        }
        Ok(out)
    }
}

struct WrappedListVisitor;

impl<'de> Visitor<'de> for WrappedListVisitor {
    type Value = Vec<u32>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an object {\"list\": [...]}")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Vec<u32>, A::Error>
    where
        A: MapAccess<'de>,
    {
        // data 里唯一关心的字段是 "list" (其它键随它去)
        match map.next_key::<String>()? {
            Some(key) if key == "list" => map.next_value(),
            Some(other) => Err(de::Error::unknown_field(&other, &["list"])),
            None => Err(de::Error::missing_field("list")),
        }
    }
}

// ============================================================
// 场景 B: 字符串驻留 (next_key_seed + 顶层 seed)
// ============================================================
// 输入:
//   {"apple": 3, "banana": 1, "apple": 2}
//
// 问题: 键是 Rc<str>, 相同键只希望分配一次 —— 但普通
// next_key::<Rc<str>>() 每解析一个键就新建一份字符串。
// 解法: seed 携带共享池 (Rc<RefCell<HashSet<Rc<str>>>>):
//   命中池子 → clone 已驻留的 Rc (只 bump 引用计数, 不重新分配);
//   未命中   → 驻留一份, 池子和返回值共享同一份。
// 顺带演示「顶层 seed」入口: 不经过 serde_json::from_str::<T>(),
// 直接 seed.deserialize(deserializer) —— 连入口类型都让 seed 说了算。

type Pool = Rc<RefCell<HashSet<Rc<str>>>>;

#[derive(Debug)]
struct Counts {
    items: Vec<(Rc<str>, u32)>,
}

// 顶层 seed: 携带池子, 替代 T: Deserialize 的入口
#[derive(Clone)]
struct CountsSeed {
    pool: Pool,
}

impl<'de> DeserializeSeed<'de> for CountsSeed {
    type Value = Counts;

    fn deserialize<D>(self, deserializer: D) -> Result<Counts, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(CountsVisitor { pool: self.pool })
    }
}

struct CountsVisitor {
    pool: Pool,
}

impl<'de> Visitor<'de> for CountsVisitor {
    type Value = Counts;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map of counts")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Counts, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut items = Vec::new();
        // ★ 键的解析走 seed: 池子作为上下文传进去
        while let Some(key) = map.next_key_seed(InternedKeySeed { pool: self.pool.clone() })? {
            let value = map.next_value()?;
            items.push((key, value));
        }
        Ok(Counts { items })
    }
}

// 键的 seed: 唯一职责是「查池子 / 进池子」
struct InternedKeySeed {
    pool: Pool,
}

impl<'de> DeserializeSeed<'de> for InternedKeySeed {
    type Value = Rc<str>;

    fn deserialize<D>(self, deserializer: D) -> Result<Rc<str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor {
            pool: Pool,
        }

        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Rc<str>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string key")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Rc<str>, E> {
                let mut pool = self.pool.borrow_mut();
                if let Some(existing) = pool.get(v) {
                    // 命中: 复用已驻留的 Rc (引用计数 +1, 不重新分配)
                    Ok(existing.clone())
                } else {
                    // 未命中: 驻留一份, 池子与返回值共享同一份
                    let rc: Rc<str> = Rc::from(v.to_owned());
                    pool.insert(rc.clone());
                    Ok(rc)
                }
            }
        }

        deserializer.deserialize_identifier(KeyVisitor { pool: self.pool })
    }
}

// ============================================================
// 场景 C: 递归深度限制 (Copy 的 seed 顺着递归传值)
// ============================================================
// 输入: {"value": 1, "children": [{"value": 2, "children": [{"value": 3}]}]}
//
// 问题: 嵌套深度有限制, 每层都要知道「现在第几层」——
// deserializer 是 &mut 借用, 没法存进 Visitor 里顺着递归传递;
// seed 是 Copy 的, 可以按值传进每一层递归。
// 解法: NodeSeed { depth, max_depth } 每进一层 +1, 超过即报错。

#[derive(Debug, PartialEq)]
struct Node {
    value: i32,
    children: Vec<Node>,
}

#[derive(Clone, Copy)]
struct NodeSeed {
    depth: u32,
    max_depth: u32,
}

const NODE_FIELDS: &[&str] = &["value", "children"];

impl<'de> DeserializeSeed<'de> for NodeSeed {
    type Value = Node;

    fn deserialize<D>(self, deserializer: D) -> Result<Node, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 进入本节点前先检查深度
        if self.depth > self.max_depth {
            return Err(de::Error::custom(format!(
                "nesting depth {} exceeds max_depth {}",
                self.depth, self.max_depth
            )));
        }
        deserializer.deserialize_struct("Node", NODE_FIELDS, NodeVisitor { seed: self })
    }
}

struct NodeVisitor {
    seed: NodeSeed,
}

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a node at depth {}", self.seed.depth)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Node, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut value = None;
        let mut children = None;
        while let Some(key) = map.next_key::<NodeField>()? {
            match key {
                NodeField::Value => {
                    if value.is_some() {
                        return Err(de::Error::duplicate_field("value"));
                    }
                    value = Some(map.next_value()?);
                }
                NodeField::Children => {
                    if children.is_some() {
                        return Err(de::Error::duplicate_field("children"));
                    }
                    // ★ 孩子的深度 +1, 由 ChildrenSeed 继续往下传
                    children = Some(map.next_value_seed(ChildrenSeed {
                        depth: self.seed.depth + 1,
                        max_depth: self.seed.max_depth,
                    })?);
                }
            }
        }
        Ok(Node {
            value: value.ok_or_else(|| de::Error::missing_field("value"))?,
            children: children.unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
enum NodeField {
    Value,
    Children,
}

impl<'de> Deserialize<'de> for NodeField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Fv;
        impl<'de> Visitor<'de> for Fv {
            type Value = NodeField;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("field `value` or `children`")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<NodeField, E> {
                match v {
                    "value" => Ok(NodeField::Value),
                    "children" => Ok(NodeField::Children),
                    other => Err(de::Error::unknown_field(other, NODE_FIELDS)),
                }
            }
        }
        deserializer.deserialize_identifier(Fv)
    }
}

// 「一组孩子」也要一个种子: 携带深度, 用来反序列化 Vec<Node>
struct ChildrenSeed {
    depth: u32,
    max_depth: u32,
}

impl<'de> DeserializeSeed<'de> for ChildrenSeed {
    type Value = Vec<Node>;

    fn deserialize<D>(self, deserializer: D) -> Result<Vec<Node>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChildrenVisitor {
            depth: self.depth,
            max_depth: self.max_depth,
        })
    }
}

struct ChildrenVisitor {
    depth: u32,
    max_depth: u32,
}

impl<'de> Visitor<'de> for ChildrenVisitor {
    type Value = Vec<Node>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a list of child nodes")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Vec<Node>, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let node_seed = NodeSeed {
            depth: self.depth,
            max_depth: self.max_depth,
        };
        let mut out = Vec::new();
        // ★ seq 元素的解析也走 seed
        while let Some(node) = seq.next_element_seed(node_seed)? {
            out.push(node);
        }
        Ok(out)
    }
}

// ============================================================
// 运行
// ============================================================
fn main() {
    println!("=== 场景 A: 版本号驱动 (next_value_seed) ===");
    let c1: Config = serde_json::from_str(r#"{"version": 1, "data": {"list": [10, 20]}}"#).unwrap();
    println!("  v1 输入           → {c1:?}");
    let c2: Config = serde_json::from_str(r#"{"version": 2, "data": [1, 2, 3]}"#).unwrap();
    println!("  v2 输入           → {c2:?}");
    println!(
        "  形状错配 v1 给数组  → {}",
        serde_json::from_str::<Config>(r#"{"version": 1, "data": [10, 20]}"#).unwrap_err()
    );
    println!(
        "  形状错配 v2 给对象  → {}",
        serde_json::from_str::<Config>(r#"{"version": 2, "data": {"list": [10]}}"#).unwrap_err()
    );
    println!(
        "  data 先于 version   → {}",
        serde_json::from_str::<Config>(r#"{"data": [1, 2], "version": 2}"#).unwrap_err()
    );

    println!("\n=== 场景 B: 字符串驻留 (next_key_seed + 顶层 seed) ===");
    let pool = Rc::new(RefCell::new(HashSet::new()));
    let seed = CountsSeed { pool: pool.clone() };

    let c1 = seed
        .clone()
        .deserialize(&mut serde_json::Deserializer::from_str(r#"{"apple": 3, "banana": 1, "apple": 2}"#))
        .unwrap();
    let c2 = seed
        .clone()
        .deserialize(&mut serde_json::Deserializer::from_str(r#"{"apple": 9, "orange": 5}"#))
        .unwrap();

    println!("  第一次解析           → {c1:?}");
    println!("  第二次解析           → {c2:?}");
    println!(
        "  同一次解析内, 两个 apple 共享一份字符串: {}",
        Rc::ptr_eq(&c1.items[0].0, &c1.items[2].0)
    );
    println!(
        "  两次解析之间, apple 也是同一份:         {}",
        Rc::ptr_eq(&c1.items[0].0, &c2.items[0].0)
    );
    println!(
        "  池子里只有 3 个字符串 (apple/banana/orange): {}",
        pool.borrow().len()
    );

    println!("\n=== 场景 C: 递归深度限制 (next_value_seed + next_element_seed) ===");
    let deep = r#"{"value": 1, "children": [{"value": 2, "children": [{"value": 3}]}]}"#;
    let tree = NodeSeed { depth: 0, max_depth: 2 }
        .deserialize(&mut serde_json::Deserializer::from_str(deep))
        .unwrap();
    println!("  max_depth=2  → {tree:?}");
    let err = NodeSeed { depth: 0, max_depth: 1 }
        .deserialize(&mut serde_json::Deserializer::from_str(deep))
        .unwrap_err();
    println!("  max_depth=1  → {err}");
}
