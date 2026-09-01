# 第二十一章 专题: struct / map / enum 的 Access 访问器使用指南

**配套 demo**(按数据形状拆分的四个示例):
- `demos/examples/09_access_struct.rs` — struct 双入口(visit_map / visit_seq)
- `demos/examples/10_access_map.rs` — map 任意键 + seq 家族
- `demos/examples/11_access_enum.rs` — enum 四种 variant 形状
- `demos/examples/12_access_nested.rs` — 复合类型嵌套的递归调用链(带追踪打印)

**前置章节**: 第 2 章(数据模型)、第 3-8 章(Serialize/Deserialize/Visitor/Deserializer)、第 15 章(枚举)
**扩展阅读**: `05_expand.rs`(derive 生成的真实代码)、`08_why_visitor.rs` 第 4 节(枚举格式侧)、
第 22 章(`next_key_seed` / `next_value_seed` / `next_element_seed` 等 `*_seed` 的用法)

---

## 0. 心智模型: 格式生产 token, 类型消费 token

Serde 的核心抽象是**数据模型**(第 2 章)。对复合类型,反序列化两侧各有一组协议:

```
反序列化(类型驱动, 调用 Deserializer 方法):
  struct S { x: i32 }
  ──► d.deserialize_struct("S", &["x"], Visitor)
        ──► visitor.visit_map(MapAccess)      ← 类型侧在这里"轮询"数据
              next_key() → next_value() → ... → None

序列化(类型主动, 调用 Serializer 方法):
  s.serialize_struct("S", 1)
    ──► SerializeStruct.serialize_field("x", &v) → end()
```

**最关键的区分 —— 谁是实现方, 谁是调用方:**

| trait | 实现方 | 调用方 |
|-------|--------|--------|
| `MapAccess` / `SeqAccess` / `EnumAccess` / `VariantAccess` | **格式方**(serde_json 等) | **类型方**(derive 生成的或手写的 `Deserialize`) |
| `SerializeMap` / `SerializeStruct` / `SerializeTupleVariant` / ... | **格式方** | **类型方**(手写的 `Serialize`) |

即: Access 系列 trait 对你(手写 Deserialize 的类型作者)来说是**被交付的协议对象**,
不是需要你自己实现的(除非你在写一个格式)。derive 生成的代码就是你该学会写的东西。

**token 是消耗品, 协议是交替握手:**

```
JSON 对象 {"x": 1.5, "y": -2.0} 的 token 流:
  MapStart → Str("x") → F64(1.5) → Str("y") → F64(-2.0) → MapEnd

next_key()   ──消费──►  Str("x")   → 返回 Some(x 标识)
next_value() ──消费──►  F64(1.5)   → 返回 1.5
next_key()   ──消费──►  Str("y")   → 返回 Some(y 标识)
next_value() ──消费──►  F64(-2.0)  → 返回 -2.0
next_key()   ────────►  (已耗尽)   → 返回 None, 结束
```

`next_key` 与 `next_value` **必须严格交替调用**: key 没消费完就调 `next_value` 是
未定义行为;反过来,读到的键不消费对应值就 `continue`,后面的键值对会全部错位。

---

## 1. struct 专题

### 1.1 反序列化调用链

```rust
impl<'de> Deserialize<'de> for Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_struct("Point", &["x", "y"], PointVisitor)
        //                                  名        字段表      Visitor
    }
}
```

`deserialize_struct` 之后,格式方按输入形状从两个入口中选一个调用 Visitor:

| 输入形状 | 入口 | 典型格式 |
|----------|------|----------|
| 对象(键值对) | `visit_map(MapAccess)` | JSON、TOML |
| 序列(按位置) | `visit_seq(SeqAccess)` | bincode、CSV |

**两者都要实现** —— 你不知道调用方用哪种格式。

### 1.2 字段名 → `__Field` 标识符枚举

visit_map 里 `next_key` 的键类型几乎总是**自定义的标识符枚举**(derive 生成的 `__Field`):

```rust
enum PointField { X, Y }

impl<'de> Deserialize<'de> for PointField {
    fn deserialize<D>(d: D) -> Result<Self, D::Error> {
        struct Fv;
        impl<'de> Visitor<'de> for Fv {
            type Value = PointField;
            fn visit_str<E: de::Error>(self, v: &str) -> Result<PointField, E> {
                match v {
                    "x" => Ok(PointField::X),
                    "y" => Ok(PointField::Y),
                    other => Err(de::Error::unknown_field(other, &["x", "y"])),
                }
            }
        }
        d.deserialize_identifier(Fv)   // ← 专门给"键/变体名"用的入口
    }
}
```

**`__Field` 只在 visit_map 路径出场**: 它是 `next_key` 的键类型,而键只存在于
键值对(对象)形状里。visit_seq 路径按位置取 `next_element`、没有字段名,
**根本不会反序列化 `__Field`** —— 所以 bincode/CSV 输入走 visit_seq 时,这份
代码一个字节都不执行。此外它的 Visitor 实现的是**标量入口** `visit_str`(JSON
字段名必是字符串)、`visit_bytes` / `visit_u64`(二进制格式),而不是
`visit_map` / `visit_seq` —— 键永远是单个标量 token,不可能遇到复合形状。

为什么多此一举?
1. **匹配更快更安全**: 枚举 match 替代字符串比较,编译器检查分支完备性;
2. **未知字段报错正确**: `unknown_field` 自动附上"expected x or y";
3. **兼容无名字格式**: 另加 `visit_u64`(0→X, 1→Y) 即可支持 bincode 的数字下标
   (见 `serde_derive/src/de/identifier.rs` 与 `05_expand.rs`)。

### 1.3 visit_map 的三件套: 收集 → 检查重复 → 最后统一查缺失

```rust
fn visit_map<A>(self, mut map: A) -> Result<Point, A::Error> {
    let mut x = None;
    let mut y = None;                     // 键顺序不确定, 先收集到 Option

    while let Some(key) = map.next_key::<PointField>()? {
        match key {
            PointField::X => {
                if x.is_some() { return Err(de::Error::duplicate_field("x")); }
                x = Some(map.next_value()?);   // 类型由 x 的 Option<f64> 推断
            }
            PointField::Y => { ... }
        }
    }

    let x = x.ok_or_else(|| de::Error::missing_field("x"))?;
    let y = y.ok_or_else(|| de::Error::missing_field("y"))?;
    Ok(Point { x, y })
}
```

要点:
- **重复字段**由你检查(JSON 允许重复键,derive 会生成同样的检查);
- **缺失字段**最后统一用 `missing_field` 报错 —— 这样错误信息里能看到字段名;
- 未知键分支: 若不想报错而是忽略,必须先消费值 `let _ = map.next_value::<serde::de::IgnoredAny>()?;`
  再继续循环,否则键值对错位。

### 1.4 visit_seq 按位置取

```rust
fn visit_seq<A>(self, mut seq: A) -> Result<Point, A::Error> {
    let x = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
    let y = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
    Ok(Point { x, y })
}
```

### 1.5 序列化侧: SerializeStruct

```rust
impl Serialize for Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Point", 2)?;  // (名字, 字段数)
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        state.end()                                                 // 必须调用!
    }
}
```

`end()` 返回 `S::Ok` —— 忘记 `end()` 或忘记返回它,序列化会静默失败。

---

## 2. map 专题

### 2.1 map 与 struct 的区别

| | struct | map |
|--|--------|-----|
| 键的类型 | 编译期固定的字段名 | **任意类型**(`i32`、`&str`、自定义……) |
| 键的反序列化 | `deserialize_identifier` | 普通 `Deserialize` |
| 键枚举 | 需要 `__Field` | 不需要 |
| 形状约定 | `serialize_field`(自动字符串键) | `serialize_key` + `serialize_value`(任意键) |

**为什么 struct 也走 visit_map?** 因为格式只有两种复合 token: 数组与对象。
JSON 里 struct 就是对象 `{"x":1,"y":2}` —— serde_json 的 `deserialize_struct`
和 `deserialize_map` 遇到 `{` 都只能给调用方一个 MapAccess,于是两者最终都调
`visitor.visit_map`。struct 与 map 在 token 流层面**完全一样**,"类型名"只影响
类型侧如何消费: struct 的键是编译期固定的字段名(由 `__Field` 校验),map 的键
是任意 `Deserialize` 类型。差异只在 `next_key` 的类型上。
(反过来, bincode/CSV 把 struct 表示成序列,同一份 struct 代码就会走 visit_seq。)

### 2.2 反序列化: deserialize_map → visit_map → next_entry

```rust
impl<'de> Deserialize<'de> for ScoreTable {
    fn deserialize<D>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(ScoreTableVisitor)     // 注意: 是 map 不是 struct
    }
}

fn visit_map<A>(self, mut map: A) -> Result<ScoreTable, A::Error> {
    let mut scores = BTreeMap::new();
    // next_entry = next_key + next_value 的组合, 键值类型各写各的
    while let Some((score, name)) = map.next_entry()? {
        scores.insert(score, name);
    }
    Ok(ScoreTable(scores))
}
```

`{"10": "alice"}` 走 `next_key::<i32>()` 会把字符串 "10" 解析成 `10` ——
这就是"键是任意类型"的含义。

### 2.3 序列化: SerializeMap

```rust
impl Serialize for ScoreTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;  // len 未知传 None
        for (score, name) in &self.0 {
            map.serialize_entry(score, name)?;   // = serialize_key + serialize_value
        }
        map.end()
    }
}
```

---

## 3. enum 专题

### 3.1 完整调用链(外部标签, 默认策略)

```
JSON {"Move": {"x": 10, "y": 20}} 的 token 流:
  MapStart → Str("Move") → MapStart → Str("x") → I64(10) → Str("y") → I64(20) → MapEnd → MapEnd

Event::deserialize
  └─ d.deserialize_enum("Event", VARIANTS, EventVisitor)
      └─ visitor.visit_enum(EnumAccess)
          └─ data.variant::<EventField>()           ① 消费 Str("Move"), 得 (变体标识, VariantAccess)
              └─ match 变体标识                     ② 匹配即提交!
                  ├─ "Move" → access.struct_variant(&["x","y"], MoveVisitor)
                  │              └─ 内部又是一个 visit_map 交替循环
                  └─ 未知   → unknown_variant, 不重试
```

### 3.2 EnumAccess 与 VariantAccess 的分工

- `EnumAccess::variant()`: **整个流程只调用一次**,消费变体名 token,返回
  `(变体标识, VariantAccess)`。变体标识通常也是 `__Field` 标识符枚举
  (同 1.2 节,只是 `unknown_variant` 报错)。
- `VariantAccess`: 四种方法,一一对应四种 variant 形状:

| variant 形状 | 反序列化方法 | 载荷 |
|--------------|--------------|------|
| `Quit` | `unit_variant()` | 无 |
| `Write(String)` | `newtype_variant()` | 单个值 |
| `ChangeColor(i32,i32,i32)` | `tuple_variant(len, Visitor)` | 序列,由 Visitor 的 visit_seq 接收 |
| `Move { x, y }` | `struct_variant(&["x","y"], Visitor)` | 键值对,由 Visitor 的 visit_map 接收 |

```rust
fn visit_enum<A>(self, data: A) -> Result<Event, A::Error> {
    use de::VariantAccess;
    let (variant, access) = data.variant::<EventField>()?;   // ① 唯一一次读名字

    match variant {
        EventField::Quit => { access.unit_variant()?; Ok(Event::Quit) }
        EventField::Write => Ok(Event::Write(access.newtype_variant()?)),
        EventField::ChangeColor => {
            let (r, g, b) = access.tuple_variant(3, ChangeColorVisitor)?;   // ②
            Ok(Event::ChangeColor(r, g, b))
        }
        EventField::Move => {
            let (x, y) = access.struct_variant(&["x", "y"], MoveVisitor)?;  // ②
            Ok(Event::Move { x, y })
        }
    }
}
```

`tuple_variant` / `struct_variant` 的 Visitor 就是普通 Visitor(分别实现 `visit_seq`
/ `visit_map`),只是把结果原样吐出来 —— 见 demo 里的 `ChangeColorVisitor`/`MoveVisitor`
(`11_access_enum.rs`)。

### 3.3 「匹配即提交」, 不回退

变体名在 `variant()` 时被**消费一次**;一旦 match 命中分支,载荷失败(如
`{"Move": {"x": "abc"}}`)会直接传播错误,**绝不会回头尝试其他变体**。
想支持"多形状"只有: `#[serde(untagged)]`(先缓冲再逐个尝试,见第 15 章)
或自己在 `deserialize_any` 里手工分派。

### 3.4 序列化侧: 四个 `serialize_*_variant`

```rust
impl Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Event::Quit => serializer.serialize_unit_variant("Event", 0, "Quit"),
            Event::Write(s) => serializer.serialize_newtype_variant("Event", 1, "Write", s),
            Event::ChangeColor(r, g, b) => {
                let mut tv = serializer.serialize_tuple_variant("Event", 2, "ChangeColor", 3)?;
                tv.serialize_field(r)?; tv.serialize_field(g)?; tv.serialize_field(b)?;
                tv.end()
            }
            Event::Move { x, y } => {
                let mut sv = serializer.serialize_struct_variant("Event", 3, "Move", 2)?;
                sv.serialize_field("x", x)?; sv.serialize_field("y", y)?;
                sv.end()
            }
        }
    }
}
```

注意参数顺序: **(类型名, 变体下标, 变体名, [长度 | 载荷])**。
下标只对 bincode 等二进制格式有意义,JSON 忽略。

---

## 4. 复合类型嵌套 —— 递归调用链

**嵌套的核心机制(一句话)**:

> `visit_map` 里的 `map.next_value::<T>()` 会调用 `T::deserialize`,
> 而 `T::deserialize` 又走一遍 `d.deserialize_xxx → visitor.visit_xxx → Access`。
> 每一层都是同一套握手协议, 直到某个字段是基本类型 (`i32` / `String` …) 为止。

例如 `Game { title: String, players: Vec<Player>, power: PowerUp }`:

```
Game::deserialize → d.deserialize_struct
  └─ visit_map(MapAccess)
      ├─ next_key "title" → next_value::<String>       ← 基本类型, 一步到底
      ├─ next_key "players" → next_value::<Vec<Player>>
      │    └─ Vec::deserialize → visit_seq(SeqAccess)
      │        └─ Player::deserialize → visit_map(MapAccess)   ← 又一轮握手
      │            ├─ next_key "name" → next_value::<String>
      │            └─ next_key "score" → next_value::<u32>
      └─ next_key "power" → next_value::<PowerUp>
           └─ PowerUp::deserialize → visit_enum(EnumAccess)
               └─ variant() → newtype_variant::<u32>()   ← 又一轮握手
```

几个"顺理成章"的推论:

1. **visit_map 内部可以嵌套任何东西** —— 因为 `next_value` 的类型参数 `T` 由你决定,
   `T: Deserialize` 就自动获得完整递归能力。手写时你不需要做任何"穿透"处理。
2. **访问器互相组合**: struct 里可以嵌 enum(VariantAccess 的 struct_variant 内部
   又是 visit_map),enum 的载荷里可以嵌 map,map 的值里可以嵌 seq…… 没有深度限制。
3. **序列化侧完全镜像**: `serialize_field("players", &self.players)` 内部触发
   `Vec<Player>::serialize → serialize_seq → Player::serialize → serialize_struct`,
   方向相反, 结构相同。
4. **格式方毫不知情**: serde_json 不需要认识 `Game` —— 它只把 token 流一段段
   吐出来, 每层 visitor 各取所需。

> 可运行演示: `demos/examples/12_access_nested.rs`
> 带缩进追踪打印, 把反序列化/序列化两边的递归展开过程完整显示出来。
> 示例结构: `Game(struct)` → `Vec<Player>` → `Player(struct)`、`BTreeMap`、
> `PowerUp(enum)`、`Vec<Vec<u32>>`(seq 嵌 seq)。

---

## 5. 总对照表

| 数据形状 | 反序列化(类型侧) | 序列化(类型侧) |
|----------|------------------|----------------|
| `struct S {..}` | `deserialize_struct` → `visit_map`(MapAccess) / `visit_seq`(SeqAccess) | `serialize_struct` → SerializeStruct |
| `struct S(T)` | `deserialize_newtype_struct` → `visit_newtype_struct` | `serialize_newtype_struct` |
| `struct S();` | `deserialize_unit_struct` → `visit_unit` | `serialize_unit_struct` |
| `struct S(A,B)` | `deserialize_tuple_struct` → `visit_seq` | `serialize_tuple_struct` |
| `HashMap/BTreeMap` | `deserialize_map` → `visit_map`(MapAccess) | `serialize_map` → SerializeMap |
| `Vec/[T]` | `deserialize_seq` → `visit_seq`(SeqAccess) | `serialize_seq` → SerializeSeq |
| `enum E { V }` | `deserialize_enum` → `visit_enum`(EnumAccess) → `unit_variant` | `serialize_unit_variant` |
| `enum E { V(T) }` | … → `newtype_variant` | `serialize_newtype_variant` |
| `enum E { V(A,B) }` | … → `tuple_variant` | `serialize_tuple_variant` |
| `enum E { V {..} }` | … → `struct_variant` | `serialize_struct_variant` |

访问器 API 一览(签名见 `serde_core/src/de/mod.rs`):

```
MapAccess<'de>     next_key_seed / next_key<K>      Option<K>
                   next_value_seed / next_value<V>   V
                   next_entry_seed / next_entry<K,V> Option<(K, V)>
SeqAccess<'de>     next_element_seed / next_element<T> Option<T>
EnumAccess<'de>    variant_seed / variant<V>          (V, Self::Variant)
VariantAccess<'de> unit_variant | newtype_variant<T> | tuple_variant(len, V) | struct_variant(fields, V)
```

---

## 6. 常见错误清单

1. **`next_key` 后忘了 `next_value` / 先调了 `next_value`** —— 协议要求严格交替,
   违反是未定义行为(可能 panic 或拿到错位数据)。
2. **键类型推断不出来** —— `map.next_key()` 不带类型标注时,从上下文无法推断,
   写全 `next_key::<PointField>()?`。
3. **未知字段直接跳过没消费值** —— `continue` 后键值对错位;要么报 `unknown_field`,
   要么 `map.next_value::<IgnoredAny>()?`。
4. **字段存在性检查**: `Option<T>` 字段本身能接受 null,但"键根本没出现"和"键出现值为
   null"是两回事 —— derive 生成代码里,缺失的 Option 字段直接 `Ok(None)`,不需要
   `missing_field`。
5. **`variant()` 只调一次** —— 不要写循环去"试探"变体名;没有第二次机会。
6. **variant 形状与方法不匹配** —— 例如对 `Write(String)` 调 `unit_variant()`,
   格式方会返回 invalid_type 错误。按 3.2 表选方法。
7. **忘了 `use de::VariantAccess;`** —— `unit_variant` 等方法是 VariantAccess 的
   trait 方法,需要引入 trait 才能调用。
8. **序列化侧忘了 `end()` 或忘了返回 `end()` 的结果** —— 序列化静默失败。
9. **`serialize_map` 的 len 乱填** —— 不知道就传 `None`,填错会导致某些格式输出
   损坏(如二进制格式的分配)。
10. **标识符枚举只实现 `visit_str`** —— 面向 bincode/CSV 的格式需要 `visit_bytes`
   或 `visit_u64`(derive 会生成齐全)。

---

**练习**:
1. 给 `09_access_struct.rs` 的 `Point` 增加一个 `Option<i32>` 字段 `z`,观察
   `{"x":1,"y":2}` 与 `{"x":1,"y":2,"z":null}` 两种输入的行为差异。
2. 把 `10_access_map.rs` 的 `ScoreTable` 键改成自定义类型
   (如 `enum Tier { Bronze, Gold }`),体会"map 的键是任意类型"与 struct 字段键的
   本质区别。
3. 为 `11_access_enum.rs` 的 `Event` 手写一个 `deserialize_any` 分派版: 输入是
   字符串时按 `Write` 处理,是整数时按 `Quit` 处理(untagged 的手工版,对比第 3.3 节)。
4. 给 `12_access_nested.rs` 的 `Game` 增加一个 `Option<PowerUp>` 字段和一层
   `Vec<Vec<Vec<u32>>>`,运行后观察追踪打印的递归深度。
5. 打开 `05_expand.rs`,找到 `Point` 对应结构体的 derive 展开代码,与 `09_access_struct.rs`
   的手写版本逐行对比。
