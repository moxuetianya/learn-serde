# 第二十二章 专题: DeserializeSeed —— 种子(带上下文的 Deserialize)

**配套 demo**: `demos/examples/13_seed.rs` — 三个 seed 场景逐一运行

**前置章节**: 第 2 章(数据模型)、第 5 章(Deserialize)、第 6 章(Visitor)、第 21 章(Access 访问器)
**扩展阅读**: `serde_core/src/de/mod.rs` — `DeserializeSeed` trait(约 803 行)、
`PhantomData<T>` 的 DeserializeSeed 实现(814 行)、`next_key`(1897 行)

---

## 0. 心智模型: Deserialize 只是无上下文的种子

一句话: **seed = 带上下文的 Deserialize**。

`DeserializeSeed` 与 `Deserialize` 一一对应(28 个入口一个不少),区别只在
`DeserializeSeed::deserialize` 的调用方可以传入任意自己定义的数据。看源码
(`serde_core/src/de/mod.rs`):

```rust
// 1897 行: next_key 就是无上下文版本的 next_key_seed
fn next_key<K>(&mut self) -> Result<Option<K>, Self::Error>
where
    K: Deserialize<'de>,
{
    self.next_key_seed(PhantomData)
}

// 814 行: PhantomData<T> 在 T: Deserialize 时实现 DeserializeSeed
impl<'de, T> DeserializeSeed<'de> for PhantomData<T>
where
    T: Deserialize<'de>,
{
    type Value = T;
    ...
}
```

所以普通 `next_key::<K>()` / `next_value::<V>()` / `next_element::<T>()`
只是 `*_seed(PhantomData)` 的语法糖 —— **上下文为空时的特例**。

**什么时候该升级到 seed**(满足任一条即可):

1. **上下文依赖**: 解析规则依赖"先读到的数据" —— 版本号、标志位、父节点状态;
2. **共享状态**: 想在多次解析之间复用/累积状态 —— 字符串驻留池、缓存、arena;
3. **递归传参**: 嵌套结构的每一层都要携带配置 —— seed 是 `Copy` 的,可以按值
   传进递归调用(而 deserializer 是 `&mut` 借用,没法存进递归结构里带着走)。

四种入口对照:

| 位置 | 无上下文(语法糖) | 带上下文(本体) |
|------|------------------|----------------|
| map 键 | `next_key::<K>()` | `next_key_seed(KSeed)` |
| map 值 | `next_value::<V>()` | `next_value_seed(VSeed)` |
| seq 元素 | `next_element::<T>()` | `next_element_seed(TSeed)` |
| 顶层入口 | `serde_json::from_str::<T>()` | `seed.deserialize(&mut Deserializer::from_str(s))` |

第 21 章总表里 `next_key_seed / next_key<K>` 那一列,就是这个关系。

---

## 1. 场景 A: 版本号驱动 —— next_value_seed

**问题**: data 字段的形状要读到 version 之后才知道:

```json
{"version": 1, "data": {"list": [10, 20]}}   // v1: data 是对象
{"version": 2, "data": [1, 2, 3]}            // v2: data 是数组
```

而 visit_map 循环里 `map.next_value::<Vec<u32>>()` 的**入口在编译期就定死**了
(要么 `deserialize_seq` 要么 `deserialize_struct`,二选一)。但"选哪个入口"是个
**运行时数据**决定的问题。

**解法**: 把 version 夹进 seed,`next_value_seed` 把决策权推迟到运行时:

```rust
struct DataSeed { version: u32 }

impl<'de> DeserializeSeed<'de> for DataSeed {
    type Value = Vec<u32>;

    fn deserialize<D>(self, deserializer: D) -> Result<Vec<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.version >= 2 {
            deserializer.deserialize_seq(PlainListVisitor)          // v2: 数组
        } else {
            deserializer.deserialize_struct("data", &["list"], WrappedListVisitor)  // v1: 对象
        }
    }
}
```

使用处(Config 的 visit_map 里):

```rust
ConfigField::Data => {
    // ★ 把已读到的 version 夹带进去 (没读到就用默认 1)
    let seed = DataSeed { version: version.unwrap_or(1) };
    data = Some(map.next_value_seed(seed)?);
}
```

**要点**:
- seed 只有 4 个字节(u32),却改变了整个 data 的解析路径;
- "data 先于 version 出现"会用默认版本 1 解析,形状对不上就报错 ——
  **上下文的先后顺序由你的 visit_map 循环决定**,这是 seed 方案的固有特性;
- 错误信息里能看到两个入口各自的 expecting(对象 vs 数组),
  见 demo 里三组报错输出。

**常见疑问: `deserialize_struct` 的返回值是"struct"吗?** 不是。
`deserialize_struct` 的签名是
`fn deserialize_struct<V>(self, name, fields, visitor: V) -> Result<V::Value, D::Error>` ——
返回类型完全由 **Visitor 的 `type Value`** 决定。`WrappedListVisitor` 的 Value 就是
`Vec<u32>`,所以两条分支返回的都是 `Result<Vec<u32>, D::Error>`,类型一致。
`deserialize_seq` 与 `deserialize_struct` 只是**两个进入 Visitor 的入口**
(分别触发 `visit_seq` / `visit_map`),"struct" 只表示"期望对象形状的 token",
**入口形状 ≠ 返回类型**,产出什么由 Visitor 说了算(第 6 章的适配层思想)。

---

## 2. 场景 B: 字符串驻留 —— next_key_seed + 顶层 seed

**问题**: 键是 `Rc<str>`,相同键只希望分配一次。但 `next_key::<Rc<str>>()`
每解析一个键就新建一份字符串,JSON 里 `"apple"` 出现 100 次就分配 100 次。

**解法**: seed 携带共享池,`next_key_seed` 把池子传进键的反序列化:

```rust
type Pool = Rc<RefCell<HashSet<Rc<str>>>>;

struct InternedKeySeed { pool: Pool }

impl<'de> DeserializeSeed<'de> for InternedKeySeed {
    type Value = Rc<str>;

    fn deserialize<D>(self, deserializer: D) -> Result<Rc<str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor { pool: Pool }
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Rc<str>;
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Rc<str>, E> {
                let mut pool = self.pool.borrow_mut();
                if let Some(existing) = pool.get(v) {
                    Ok(existing.clone())   // 命中: 引用计数 +1, 不重新分配
                } else {
                    let rc: Rc<str> = Rc::from(v.to_owned());
                    pool.insert(rc.clone());   // 驻留一份, 池与返回值共享
                    Ok(rc)
                }
            }
        }
        deserializer.deserialize_identifier(KeyVisitor { pool: self.pool })
    }
}
```

**顶层 seed 入口**: 这个例子里 `Counts` 不实现 `Deserialize`,而是由
`CountsSeed` 直接当入口 —— 这样调用方才能把池子传进去:

```rust
let seed = CountsSeed { pool: pool.clone() };
let c1 = seed.clone().deserialize(&mut serde_json::Deserializer::from_str(json1))?;
let c2 = seed.clone().deserialize(&mut serde_json::Deserializer::from_str(json2))?;
```

demo 运行结果(`Rc::ptr_eq` 验证的是"同一个内存地址",不是内容相等):

```
同一次解析内, 两个 apple 共享一份字符串: true
两次解析之间, apple 也是同一份:         true
池子里只有 3 个字符串 (apple/banana/orange): 3
```

这正是 serde 内部 `InternedString`(私有)的做法 —— 同一个字段名反复出现时只
分配一次。

---

## 3. 场景 C: 递归深度限制 —— Copy 上下文顺着递归传

**问题**: 嵌套结构有深度上限,每一层都要知道"现在第几层":

```json
{"value": 1, "children": [{"value": 2, "children": [{"value": 3}]}]}
```

**为什么必须用 seed**: deserializer 是 `&mut` 借用,Visitor 也是按值收下后
不可再借;唯一能顺着递归调用链传递的东西,是 `MapAccess` / `SeqAccess` 的
`*_seed` 方法参数 —— 而 seed 是 `Copy` 的,可以按值传进每一层。

```rust
#[derive(Clone, Copy)]
struct NodeSeed { depth: u32, max_depth: u32 }

impl<'de> DeserializeSeed<'de> for NodeSeed {
    type Value = Node;
    fn deserialize<D>(self, deserializer: D) -> Result<Node, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth > self.max_depth {   // 进节点前先查深度
            return Err(de::Error::custom(format!(
                "nesting depth {} exceeds max_depth {}", self.depth, self.max_depth)));
        }
        deserializer.deserialize_struct("Node", NODE_FIELDS, NodeVisitor { seed: self })
    }
}
```

递归的两处接力(注意深度只 +1 一次):

```rust
// ① 节点 → 孩子序列: 深度 +1
children = Some(map.next_value_seed(ChildrenSeed {
    depth: self.seed.depth + 1,
    max_depth: self.seed.max_depth,
})?);

// ② 序列 → 元素: 用同一个深度 (不再 +1)
while let Some(node) = seq.next_element_seed(node_seed)? {
    out.push(node);
}
```

demo 运行结果:

```
max_depth=2  → Node { value: 1, children: [Node { value: 2, children: [Node { value: 3, ... }] }] }
max_depth=1  → nesting depth 2 exceeds max_depth 1 at line 1 column 52
```

这个模式推广开就是: **tree + arena、嵌套结构配额、递归深度保护** ——
凡是"每层带状态"的递归反序列化,seed 都是标准答案。

---

## 4. 误区清单

1. **seed 不是 Deserializer** —— `seed.deserialize(d)` 里的 `d` 才是。
   你在 `DeserializeSeed` 实现里该写 `d.deserialize_xxx(...)`,不该自己解析数据。
2. **实现 DeserializeSeed 内部仍是完整链路** —— 走的是
   `d.deserialize_xxx → visitor.visit_xxx → Access` 的同一套握手(第 21 章),
   seed 只是夹带一份上下文,协议一点没变。
3. **能不用就不用** —— derive 场景永远不需要 seed;`next_key::<K>()` 全家桶
   (即 `*_seed(PhantomData)`)就是 99% 的情况。只有上下文确需跨层级传递时,
   才实现自定义 seed。
4. **顺序敏感** —— 场景 A 里 `data` 在 `version` 之前出现会用到默认版本;
   上下文"先读后传",读取顺序由你的 visit_map 循环决定,想兜底就给默认值。
5. **驻留类的共享池要放 `Rc` 本身** —— 场景 B 的池子如果存 `String`,
   命中时 `Rc::from(existing.clone())` 会重新分配一份,`Rc::ptr_eq` 立刻露馅
   (demo 里就是这么改对的)。
6. **入口形状 ≠ 返回类型** —— `deserialize_struct` 的返回值是 Visitor 的
   `type Value`,不是"struct"(见 1 节的常见疑问)。seed 里两个分支返回同一类型
   才算写对。

---

**练习**:
1. 给场景 A 增加 v3: data 变成对象 `{"nums": [...], "sum": ...}`,
   观察 v1 / v2 输入在 v3 版 DataSeed 下各报什么错。
2. 把场景 B 的 `Rc<str>` 换成 `&'de str` 试试(提示: `'de` 生命周期
   DeserializeSeed 同样携带,借用上下文本身就是一种 seed)。
3. 把场景 C 的深度检查从 `NodeSeed::deserialize` 挪到 `ChildrenSeed::deserialize`
   (提前一层报错),对比报错时的列号差异,理解"检查发生在哪个 token 上"。
4. 不用 seed 重写场景 A: 在 visit_map 里把 version 存进局部变量,data 改用
   `next_value::<serde_json::Value>()` 再事后转换 —— 体会"先收后转"与
   "seed 决定形状"两种风格各自的代价。
5. 打开 `serde_core/src/de/mod.rs` 的 `next_entry`(约 1924 行),确认
   `next_entry::<K, V>()` 也是 `next_entry_seed(PhantomData, PhantomData)` 的
   语法糖。
