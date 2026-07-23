# 第十八章: serde_core 的内部细节 —— lib, format, forward 宏

**源码参考**: `serde_core/src/{lib.rs, crate_root.rs, macros.rs, format.rs, std_error.rs}`

## crate_root! 宏 —— 整个 crate 的骨架

```rust
// 源码: serde_core/src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

crate_root!();

// 源码: serde_core/src/crate_root.rs
macro_rules! crate_root {
    () => {
        // 1. lib 模块: 条件性 re-export core/std/alloc 类型
        #[macro_use]
        mod lib {
            // 从 core 始终可用:
            pub use core::{f32, f64, iter, num, str, ...};

            // 从 alloc/std 条件性可用:
            #[cfg(any(feature = "std", feature = "alloc"))]
            pub use alloc::{String, Vec, Box, Cow, ...};

            #[cfg(feature = "std")]
            pub use std::{HashMap, HashSet, ...};
        }

        // 2. tri! 宏
        #[macro_export]
        macro_rules! tri {
            ($e:expr) => {
                match $e {
                    $crate::lib::result::Result::Ok(val) => val,
                    $crate::lib::result::Result::Err(err) => {
                        return $crate::lib::result::Result::Err(
                            $crate::lib::convert::From::from(err)
                        );
                    }
                }
            };
        }

        // 3. re-export 公共 API
        #[doc(inline)]
        pub use self::de::{Deserialize, Deserializer};
        #[doc(inline)]
        pub use self::ser::{Serialize, Serializer};

        // 4. 私有模块
        mod __private {
            pub use crate::private::*;
        }
    };
}
```

### 为什么 serde_core 内部不用 std::result::Result?

```rust
// serde_core 使用 crate::lib::result::Result 而不是 std::result::Result
// 原因: no_std 兼容
// crate::lib 模块根据 feature 选择 re-export 来源:
//   std → std::result::Result
//   alloc → core::result::Result
//   nothing → core::result::Result
```

## tri! 宏 vs ? 操作符

```rust
// tri! 宏展开:
tri!(expression)
// 等价于:
match expression {
    Ok(val) => val,
    Err(err) => return Err(From::from(err)),
}
```

为什么 serde 的测试显示 `tri!` 比 `?` 快 5.5-9%?

- `?` 在编译时产生更复杂的 LLVM IR(因为有 `Try` trait、`From` trait 的多种实现)
- `tri!` 是对 Result 的简单 match,LLVM 优化器处理得更快
- 在 serde 这样的泛型密集型代码中,编译性能差异显著

## macros.rs —— forward_to_deserialize_any!

```rust
// 源码: serde_core/src/macros.rs
// 为一个 Deserializer 实现生成转发方法

// 基本用法:
impl<'de> Deserializer<'de> for MyDeserializer {
    // 手动实现 deserialize_bool, deserialize_i32, ...

    forward_to_deserialize_any! {
        i8 i16 i64 u8 u16 u32 u64 f32 f64 char
        string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}

// 展开为:
fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    self.deserialize_any(visitor)
}
fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    self.deserialize_any(visitor)
}
// ... 26 个方法
```

### 高级用法: 自定义 Visitor / 生命周期

```rust
// 当 Deserializer 的 impl 有额外的泛型参数或自定义 Visitor bound 时:
forward_to_deserialize_any! {
    <W: Visitor<'de> + ?Sized>
    i8 i16 i64 ...
}
// 展开为:
fn deserialize_i8<W: Visitor<'de> + ?Sized>(self, visitor: W) -> ...
```

### 宏实现原理

```rust
macro_rules! forward_to_deserialize_any {
    // 入口: 解析 visitor 绑定
    (<$visitor:ident: Visitor<$lifetime:tt>> $($ty:ident)*) => { ... };
    ($($ty:ident)*) => { ... };

    // 对每个类型,生成对应方法
    // 不同类型可能需要不同的 visitor 方法名:
    //   deserialize_bool → visitor.visit_bool
    //   deserialize_string → visitor.visit_string
    //   deserialize_seq → 传入 visitor
}

// 内部使用 forward_to_deserialize_any_method! 做类型→方法名映射:
macro_rules! forward_to_deserialize_any_method {
    (bool $($rest:tt)*) => { visit_bool };
    (i8 $($rest:tt)*)    => { visit_i8 };
    // ...
    (string $($rest:tt)*) => { visit_string };
    // ...
}
```

## format.rs —— Buf(固定大小字节缓冲区)

```rust
// 源码: serde_core/src/format.rs
// 一个固定大小的 &mut [u8] 写入缓冲区,实现 fmt::Write

pub struct Buf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Buf<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self { Self { buf, pos: 0 } }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.pos]).unwrap()
    }
}

impl fmt::Write for Buf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let remaining = &mut self.buf[self.pos..];
        if bytes.len() <= remaining.len() {
            remaining[..bytes.len()].copy_from_slice(bytes);
            self.pos += bytes.len();
            Ok(())
        } else {
            Err(fmt::Error)  // 缓冲区不够 → 错误
        }
    }
}
```

### 使用场景

```rust
// 用于 Unexpected 的 display 实现中的大整数格式化:
impl Unexpected<'_> {
    fn fmt_u128(&self, v: u128) -> String {
        let mut buf = [0u8; 40];  // 栈上的小缓冲区
        let mut buf = Buf::new(&mut buf);
        write!(buf, "{}", v).unwrap();
        buf.as_str().to_owned()
    }
}

// 也用于 serialize_display_bounded_length! 宏
// 但这不是 serde 的核心功能
```

## std_error.rs —— no_std Error trait

```rust
// 源码: serde_core/src/std_error.rs
// 条件性编译: 当没有 core::error::Error 且没有 std 时

#[cfg(not(feature = "std"))]
pub trait Error: fmt::Debug + fmt::Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> { None }
}

// 有 std 时:
#[cfg(feature = "std")]
pub use std::error::Error;
```

## private 模块 —— 内部工具

### private/mod.rs

```rust
// 源码: serde_core/src/private/mod.rs
mod content;   // Content<'de> 枚举
mod seed;      // InPlaceSeed<'a, T>
mod doc;       // 文档测试辅助
mod size_hint; // 分配大小估算
mod string;    // from_utf8_lossy

pub use self::content::Content;
pub use self::seed::InPlaceSeed;
```

### size_hint.rs

```rust
// 源码: serde_core/src/private/size_hint.rs
// 用于 Vec/HashMap 等在反序列化时的预分配

// 从迭代器获取精确的大小提示
pub fn from_bounds<I>(iter: &I) -> Option<usize>
where I: Iterator,
{
    let (lower, upper) = iter.size_hint();
    // 只有精确匹配时才返回
    if Some(lower) == upper { upper } else { None }
}

// 限制预分配的大约上限(1 MiB)
pub fn cautious<Element>(hint: Option<usize>) -> usize {
    const MAX_PREALLOC_BYTES: usize = 1_048_576; // 1 MiB
    let hint = hint.unwrap_or(0);
    let max_elements = MAX_PREALLOC_BYTES / std::mem::size_of::<Element>();
    std::cmp::min(hint, max_elements)
}

// 使用示例(Vec::deserialize):
fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
    let hint = size_hint::cautious::<T>(seq.size_hint());
    let mut v = Vec::with_capacity(hint);
    while let Some(e) = seq.next_element()? {
        v.push(e);
    }
    Ok(v)
}
```

### string.rs —— from_utf8_lossy

```rust
// 源码: serde_core/src/private/string.rs
// 与 std::string::String::from_utf8_lossy 功能相同
// 但在 no_std 环境下提供自己的实现

#[cfg(any(feature = "std", feature = "alloc"))]
pub fn from_utf8_lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
pub fn from_utf8_lossy(bytes: &[u8]) -> &str {
    // no_std: 只用 \u{fffd} 替换非 UTF-8 字节
    // 在栈上构造小的缓冲区,返回 &str(无分配)
    match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            let mut tmp = [0u8; 256];  // 小缓冲区
            // ... 替换非 UTF-8 字节
        }
    }
}
```

## serde_core 的 build.rs

```rust
// 源码: serde_core/build.rs
fn main() {
    // 检测 rustc 版本,设置 cfg 标志
    let minor = rustc_minor_version().unwrap_or(0);

    if minor < 60 { println!("cargo:rustc-cfg=no_target_has_atomic"); }
    if minor < 34 { println!("cargo:rustc-cfg=no_std_atomic"); }
    if minor < 64 { println!("cargo:rustc-cfg=no_core_cstr"); }
    if minor < 71 { println!("cargo:rustc-cfg=no_serde_derive"); }
    if minor < 74 { println!("cargo:rustc-cfg=no_core_num_saturating"); }
    if minor < 77 { println!("cargo:rustc-cfg=no_core_net"); }
    if minor < 78 { println!("cargo:rustc-cfg=no_diagnostic_namespace"); }
    if minor < 81 { println!("cargo:rustc-cfg=no_core_error"); }

    // 这些 cfg 标志控制了哪些 impl 可用
    // 例如: no_core_net → IpAddr 的 Serialize/Deserialize 不可用
}
```

---

**练习**:
1. 阅读 `size_hint::cautious` 的实现,理解 1 MiB 限制的原因
2. 追踪 `forward_to_deserialize_any!` 宏的完整展开(使用 `cargo expand`)
3. 理解 serde_core 如何在 no_std 和 std 之间切换 Error trait 的来源
