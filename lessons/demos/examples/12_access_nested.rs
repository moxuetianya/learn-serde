/// 第二十一章专题 demo ④: 复合类型嵌套 —— Access 的递归调用链
///
/// 运行: cargo run --example 12_access_nested
///
/// 这是「复合类型访问器」系列示例之四 (struct / map / enum / 嵌套):
///   - 09_access_struct: struct 两种入口 (MapAccess / SeqAccess)
///   - 10_access_map:    map 任意键 (MapAccess)
///   - 11_access_enum:   enum 四种 variant 形状 (EnumAccess / VariantAccess)
///   - 12_access_nested: 复合类型嵌套的递归调用链 ← 本 demo
///
/// 嵌套的核心机制 (一句话):
///   visit_map 里的 map.next_value::<T>() 会调用 T::deserialize ——
///   而 T 的 deserialize 又走一遍 d.deserialize_xxx → visitor.visit_xxx → Access。
///   于是"访问器"层层嵌套, 每一层都是一模一样的握手协议,
///   直到某个字段是基本类型 (i32/String...) 为止。
///
/// 本 demo 用带缩进的追踪打印, 把递归展开过程完整显示出来:
///   反序列化: Game(struct) → players(Vec) → Player(struct) / power(enum) ...
///   序列化:   同样的结构, 只是方向相反 (Serialize* → 内层 Serialize*)
///
/// 注意: 追踪打印只是教学辅助, 正常手写代码不要加。

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeStruct, SerializeStructVariant};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---- 追踪工具: 全局缩进深度 ----
static DEPTH: AtomicUsize = AtomicUsize::new(0);

fn indent() -> String {
    "  ".repeat(DEPTH.load(Ordering::Relaxed))
}

fn say(tag: &str, msg: &str) {
    println!("[{tag}] {}{}", indent(), msg);
}

struct DepthGuard;

impl DepthGuard {
    fn new() -> Self {
        DEPTH.fetch_add(1, Ordering::Relaxed);
        DepthGuard
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        DEPTH.fetch_sub(1, Ordering::Relaxed);
    }
}

// ============================================================
// 1. 数据: 三层嵌套结构
//    Game (struct) 包含:
//      - Vec<Player>        struct 嵌在 seq 里
//      - BTreeMap<String,u32> map 嵌在 struct 里
//      - PowerUp (enum)     enum 嵌在 struct 里
//      - Vec<Vec<u32>>      seq 嵌在 seq 里
// ============================================================

#[derive(Debug, PartialEq)]
struct Player {
    name: String,
    score: u32,
}

impl<'de> Deserialize<'de> for Player {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        say("de", "Player::deserialize → d.deserialize_struct");
        let _g = DepthGuard::new();
        deserializer.deserialize_struct("Player", &["name", "score"], PlayerVisitor)
    }
}

struct PlayerVisitor;

impl<'de> Visitor<'de> for PlayerVisitor {
    type Value = Player;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Player")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Player, A::Error>
    where
        A: MapAccess<'de>,
    {
        let _g = DepthGuard::new();
        let mut name = None;
        let mut score = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "name" => {
                    let v = map.next_value::<String>()?;
                    say("de", &format!("next_key {key:?} → next_value → {v:?}"));
                    name = Some(v);
                }
                "score" => {
                    let v = map.next_value::<u32>()?;
                    say("de", &format!("next_key {key:?} → next_value → {v:?}"));
                    score = Some(v);
                }
                other => {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                    eprintln!("  (ignored unknown field {other:?})");
                }
            }
        }
        Ok(Player {
            name: name.ok_or_else(|| de::Error::missing_field("name"))?,
            score: score.ok_or_else(|| de::Error::missing_field("score"))?,
        })
    }
}

impl Serialize for Player {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        say("se", "Player::serialize → serializer.serialize_struct");
        let _g = DepthGuard::new();
        let mut s = serializer.serialize_struct("Player", 2)?;
        say("se", "serialize_field(\"name\", ...)");
        s.serialize_field("name", &self.name)?;
        say("se", "serialize_field(\"score\", ...)");
        s.serialize_field("score", &self.score)?;
        s.end()
    }
}

// ---- enum 嵌在 struct 里 ----
#[derive(Debug, PartialEq)]
enum PowerUp {
    None,
    Shield(u32),
    Combo { multiplier: f64, duration: f64 },
}

impl<'de> Deserialize<'de> for PowerUp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        say("de", "PowerUp::deserialize → d.deserialize_enum");
        let _g = DepthGuard::new();
        deserializer.deserialize_enum("PowerUp", &["None", "Shield", "Combo"], PowerUpVisitor)
    }
}

struct PowerUpVisitor;

impl<'de> Visitor<'de> for PowerUpVisitor {
    type Value = PowerUp;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("enum PowerUp")
    }

    fn visit_enum<A>(self, data: A) -> Result<PowerUp, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        use de::VariantAccess;
        let _g = DepthGuard::new();
        let (variant, access) = data.variant::<String>()?;
        say("de", &format!("variant() → 变体名 {variant:?}"));
        match variant.as_str() {
            "None" => {
                access.unit_variant()?;
                Ok(PowerUp::None)
            }
            "Shield" => {
                let n = access.newtype_variant()?;
                say("de", &format!("newtype_variant() → {n:?}"));
                Ok(PowerUp::Shield(n))
            }
            "Combo" => {
                let (m, d) = access.struct_variant(&["multiplier", "duration"], ComboVisitor)?;
                Ok(PowerUp::Combo { multiplier: m, duration: d })
            }
            other => Err(de::Error::unknown_variant(other, &["None", "Shield", "Combo"])),
        }
    }
}

struct ComboVisitor;

impl<'de> Visitor<'de> for ComboVisitor {
    type Value = (f64, f64);

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Combo { multiplier, duration }")
    }

    fn visit_map<A>(self, mut map: A) -> Result<(f64, f64), A::Error>
    where
        A: MapAccess<'de>,
    {
        let _g = DepthGuard::new();
        let mut m = None;
        let mut d = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "multiplier" => m = Some(map.next_value()?),
                "duration" => d = Some(map.next_value()?),
                _ => {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
        }
        Ok((
            m.ok_or_else(|| de::Error::missing_field("multiplier"))?,
            d.ok_or_else(|| de::Error::missing_field("duration"))?,
        ))
    }
}

impl Serialize for PowerUp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        say("se", "PowerUp::serialize → serialize_*_variant");
        let _g = DepthGuard::new();
        match self {
            PowerUp::None => serializer.serialize_unit_variant("PowerUp", 0, "None"),
            PowerUp::Shield(n) => serializer.serialize_newtype_variant("PowerUp", 1, "Shield", n),
            PowerUp::Combo { multiplier, duration } => {
                let mut sv = serializer.serialize_struct_variant("PowerUp", 2, "Combo", 2)?;
                sv.serialize_field("multiplier", multiplier)?;
                sv.serialize_field("duration", duration)?;
                sv.end()
            }
        }
    }
}

// ---- 顶层: struct 嵌一切 ----
#[derive(Debug, PartialEq)]
struct Game {
    title: String,
    players: Vec<Player>,
    high_scores: BTreeMap<String, u32>,
    power: PowerUp,
    grid: Vec<Vec<u32>>,
}

impl<'de> Deserialize<'de> for Game {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        say("de", "Game::deserialize → d.deserialize_struct");
        let _g = DepthGuard::new();
        deserializer.deserialize_struct(
            "Game",
            &["title", "players", "high_scores", "power", "grid"],
            GameVisitor,
        )
    }
}

struct GameVisitor;

impl<'de> Visitor<'de> for GameVisitor {
    type Value = Game;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Game")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Game, A::Error>
    where
        A: MapAccess<'de>,
    {
        let _g = DepthGuard::new();
        let mut title = None;
        let mut players = None;
        let mut high_scores = None;
        let mut power = None;
        let mut grid = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "title" => {
                    let v = map.next_value::<String>()?;
                    say("de", &format!("next_key {key:?} → next_value::<String> → {v:?} (基本类型, 递归到底)"));
                    title = Some(v);
                }
                "players" => {
                    say("de", &format!("next_key {key:?} → next_value::<Vec<Player>> 开始递归"));
                    let v = map.next_value::<Vec<Player>>()?;
                    say("de", "← Vec<Player> 递归返回");
                    players = Some(v);
                }
                "high_scores" => {
                    say("de", &format!("next_key {key:?} → next_value::<BTreeMap<String,u32>> 开始递归"));
                    let v = map.next_value::<BTreeMap<String, u32>>()?;
                    say("de", "← BTreeMap 递归返回");
                    high_scores = Some(v);
                }
                "power" => {
                    say("de", &format!("next_key {key:?} → next_value::<PowerUp> 开始递归"));
                    let v = map.next_value::<PowerUp>()?;
                    say("de", "← PowerUp 递归返回");
                    power = Some(v);
                }
                "grid" => {
                    say("de", &format!("next_key {key:?} → next_value::<Vec<Vec<u32>>> 开始递归"));
                    let v = map.next_value::<Vec<Vec<u32>>>()?;
                    say("de", "← Vec<Vec<u32>> 递归返回");
                    grid = Some(v);
                }
                other => {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                    eprintln!("  (ignored unknown field {other:?})");
                }
            }
        }

        Ok(Game {
            title: title.ok_or_else(|| de::Error::missing_field("title"))?,
            players: players.ok_or_else(|| de::Error::missing_field("players"))?,
            high_scores: high_scores.ok_or_else(|| de::Error::missing_field("high_scores"))?,
            power: power.ok_or_else(|| de::Error::missing_field("power"))?,
            grid: grid.ok_or_else(|| de::Error::missing_field("grid"))?,
        })
    }
}

impl Serialize for Game {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        say("se", "Game::serialize → serializer.serialize_struct");
        let _g = DepthGuard::new();
        let mut s = serializer.serialize_struct("Game", 5)?;
        say("se", "serialize_field(\"title\", ...)");
        s.serialize_field("title", &self.title)?;
        say("se", "serialize_field(\"players\", ...) → Vec<Player> 开始递归");
        s.serialize_field("players", &self.players)?;
        say("se", "← Vec<Player> 递归返回");
        say("se", "serialize_field(\"high_scores\", ...)");
        s.serialize_field("high_scores", &self.high_scores)?;
        say("se", "serialize_field(\"power\", ...) → PowerUp 开始递归");
        s.serialize_field("power", &self.power)?;
        say("se", "← PowerUp 递归返回");
        say("se", "serialize_field(\"grid\", ...)");
        s.serialize_field("grid", &self.grid)?;
        s.end()
    }
}

// ============================================================
// 2. 运行
// ============================================================
fn main() {
    let json = r#"{
        "title": "Serde Wars",
        "players": [
            {"name": "alice", "score": 42},
            {"name": "bob", "score": 7}
        ],
        "high_scores": {"alice": 42, "bob": 7},
        "power": {"Shield": 3},
        "grid": [[1, 2], [3, 4]]
    }"#;

    println!("=== 反序列化: Game(struct) → players(Vec) → Player(struct) / power(enum) ===");
    let game: Game = serde_json::from_str(json).unwrap();

    println!("\n=== 序列化 (方向相反, 同一结构) ===");
    let _ = serde_json::to_string(&game).unwrap();

    println!("\n=== 结果 ===");
    println!("{game:?}");
    println!("\n(注意: 基本类型字段 title 一步到底, 复合类型字段层层展开又返回)");
}
