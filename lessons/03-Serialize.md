# 第三章: Serialize —— 从 Rust 类型到数据模型

**源码参考**: `serde_core/src/ser/mod.rs:234` 和 `serde_core/src/ser/impls.rs`

## Serialize Trait

```rust
// 源码: serde_core/src/ser/mod.rs:234
pub trait Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}
```

**核心原理**: 实现 `Serialize` 的类型需要调用 `Serializer` 上的**恰好一个**方法,将自己的数据"告知"序列化器。这叫 **tell-don't-ask** 模式:类型主动告知序列化器自己的结构和值。

### 规则:
- 每个 `serialize` 实现必须调用 **恰好一个** `Serializer` 方法
- 调用哪个方法取决于 Rust 类型到数据模型的映射
- 只有这一个方法需要实现——没有其他辅助方法

## 手工实现 Serialize

### 示例 1: 基本类型

```rust
// bool 的实现 (源码: serde_core/src/ser/impls.rs ~10)
impl Serialize for bool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(*self)
    }
}

// i32 的实现
impl Serialize for i32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*self)
    }
}
```

### 示例 2: 自定义结构体

```rust
struct Duration {
    secs: u64,
    nanos: u32,
}

// 手工实现 Serialize
impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 使用 serialize_struct 表示这是一个有命名字段的结构体
        let mut state = serializer.serialize_struct("Duration", 2)?;
        state.serialize_field("secs", &self.secs)?;
        state.serialize_field("nanos", &self.nanos)?;
        state.end()
    }
}
```

### 示例 3: 自定义枚举

```rust
enum Command {
    Login { user: String, pass: String },
    Logout,
    Message(String),
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Command::Login { user, pass } => {
                let mut state = serializer.serialize_struct_variant(
                    "Command", 0, "Login", 2
                )?;
                state.serialize_field("user", user)?;
                state.serialize_field("pass", pass)?;
                state.end()
            }
            Command::Logout => {
                serializer.serialize_unit_variant("Command", 1, "Logout")
            }
            Command::Message(msg) => {
                serializer.serialize_newtype_variant(
                    "Command", 2, "Message", msg
                )
            }
        }
    }
}
```

## 选择 Serializer 方法的决策树

```
Rust 类型是什么?
├─ 基本类型 (bool, i32, f64, char)
│  → 直接用 serialize_bool / serialize_i32 / serialize_f64 / serialize_char
├─ &str
│  → serialize_str
├─ &[u8] / Vec<u8>
│  → serialize_bytes
├─ ()
│  → serialize_unit
├─ Option<T>
│  → match { None => serialize_none(), Some(v) => serialize_some(v) }
├─ 单元结构体 struct Unit;
│  → serialize_unit_struct(name)
├─ newtype 结构体 struct Meters(f64);
│  → serialize_newtype_struct(name, &self.0)
├─ 元组结构体 struct Point(u8, u8);
│  → serialize_tuple_struct(name, len)
│     + serialize_field 逐个字段
├─ 命名字段结构体 struct Point { x: u8, y: u8 };
│  → serialize_struct(name, len)
│     + serialize_field 逐个字段
├─ Vec<T> / 切片
│  → serialize_seq(len)
│     + serialize_element 逐个元素
├─ HashMap<K, V>
│  → serialize_map(len)
│     + serialize_key / serialize_value 逐对
├─ 枚举
│  ├─ 单元变体 E::V
│  │  → serialize_unit_variant(name, variant_index, variant_name)
│  ├─ newtype 变体 E::V(T)
│  │  → serialize_newtype_variant(name, variant_index, variant_name, value)
│  ├─ 元组变体 E::V(T1, T2)
│  │  → serialize_tuple_variant(name, variant_index, variant_name, len)
│  │     + serialize_field
│  └─ 结构体变体 E::V { f1, f2 }
│     → serialize_struct_variant(name, variant_index, variant_name, len)
│        + serialize_field
└─ 元组 (T1, T2, ...)
   → serialize_tuple(len)
      + serialize_element
```

## 源码: serde_core 为内置类型实现的 Serialize

文件 `serde_core/src/ser/impls.rs` (~1045 行) 为约 70+ 个 Rust 类型实现了 `Serialize`:

```
基本类型:       bool, i8..i128, u8..u128, f32, f64, char, isize, usize
字符串:         str, String
C 风格:         CStr, CString
Option:         Option<T>
PhantomData:    PhantomData<T>
数组:           [T; 0]..[T; 32]
切片:           [T]
引用:           &T, &mut T, Box<T>, Rc<T>, Arc<T>, Cow<T>
智能指针:       RcWeak<T>, ArcWeak<T>
容器:           Vec<T>, VecDeque<T>, LinkedList<T>, BinaryHeap<T>
集合:           BTreeSet<T>, HashSet<T>
映射:           BTreeMap<K, V>, HashMap<K, V>
Range:          Range, RangeFrom, RangeInclusive, RangeTo
Bound:          Bound<T>
原子类型:       AtomicBool, AtomicI8..AtomicU64, AtomicIsize, AtomicUsize
网络类型:       IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6
其他:           (), ! (never), NonZero*, Cell, RefCell, Mutex, RwLock
                Result, Duration, SystemTime, Path, PathBuf, OsStr, OsString
                Wrapping, Saturating, Reverse
元组:           (T,), (T0, T1), ... (T0, ..., T15)
```

### 特殊实现分析

```rust
// Option<T>: None 和 Some 分别映射到不同的数据模型元素
// 源码: serde_core/src/ser/impls.rs ~150
impl<T: Serialize> Serialize for Option<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            None => serializer.serialize_none(),
            Some(v) => serializer.serialize_some(v),
        }
    }
}

// 引用: 自动解引用
// 源码: serde_core/src/ser/impls.rs ~600
impl<T: Serialize + ?Sized> Serialize for &T {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(serializer)
    }
}

// 数组: 当作 tuple 序列化
// 注意: [u8] 特殊处理为 serialize_bytes, 其他当 seq
// 源码: serde_core/src/ser/impls.rs ~320
impl Serialize for [u8] {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self)
    }
}
```

## Impossible 类型

```rust
// 源码: serde_core/src/ser/impossible.rs:60
// 当某个 Serializer 不支持某些复合类型时使用
pub struct Impossible<Ok, Error> {
    void: Void,           // Void 是无人 inhabitable 的枚举
    _marker: PhantomData<(Ok, Error)>,
}

// 由于 Void 不可能被构造,Impossible 实现了所有复合 trait
// (SerializeSeq, SerializeMap 等),但任何调用都会 match void {}
// 从而编译优化掉
```

---

**练习**: 
1. 为 `serde_core/src/ser/impls.rs` 中的 `Option<T>` 和 `Vec<T>` 的实现添加注释
2. 手工为以下类型实现 `Serialize`:
   - `struct Color(u8, u8, u8);`
   - `enum Status { Active, Inactive { reason: String } }`
