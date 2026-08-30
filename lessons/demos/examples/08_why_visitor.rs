/// 第七章补充 demo: 为什么反序列化要经过 Visitor?
///
/// 运行: cargo run --example 08_why_visitor
///
/// 核心问题:
///   格式(Deserializer)只知道自己"有什么 token" (整数 300 / 字符串 "42" ...),
///   目标类型才决定"我要什么, 能不能转换" (i32? u64? f64? String? 自定义类型?)。
///   Visitor 就是连接两者的协议:
///     - 格式侧: 生产 token, 调用 visitor.visit_xxx(value)
///     - 类型侧: 实现 visit_xxx, 决定接受 / 转换 / 拒绝

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::forward_to_deserialize_any;
use std::fmt;

// ============================================================
// 1. 极简格式一: 只有一个 token —— 整数 300 (模拟 JSON 数字)
// ============================================================
struct NumDeserializer;

impl<'de> Deserializer<'de> for NumDeserializer {
    type Error = de::value::Error;

    // deserializer 不知道目标类型是什么, 只把"我这里有整数 300"
    // 告诉 visitor, 让目标类型自己决定怎么处理。
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_i64(300)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier ignored_any
    }
}

// ============================================================
// 2. 极简格式二: 只有一个 token —— 字符串 "42" (模拟 JSON 字符串)
// ============================================================
struct StrDeserializer<'de> {
    s: &'de str,
}

impl<'de> Deserializer<'de> for StrDeserializer<'de> {
    type Error = de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_str(self.s)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier ignored_any
    }
}

// ============================================================
// 3. 自定义类型: 一个 Visitor 同时接受"整数"和"字符串"两种 token
//    (这就是"类型决定" —— 格式不认识 Wrapped, 但不需要认识)
// ============================================================
#[derive(Debug)]
struct Wrapped(i64);

impl<'de> Deserialize<'de> for Wrapped {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct WrappedVisitor;

        impl<'de> Visitor<'de> for WrappedVisitor {
            type Value = Wrapped;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an integer or a string")
            }

            // 格式一(整数 token)走这里
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Wrapped, E> {
                Ok(Wrapped(v))
            }

            // 格式二(字符串 token)走这里, 还能自定义错误信息
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Wrapped, E> {
                v.parse().map(Wrapped).map_err(|_| {
                    E::invalid_type(de::Unexpected::Str(v), &self)
                })
            }
        }

        d.deserialize_any(WrappedVisitor)
    }
}

// ============================================================
// 4. 运行
// ============================================================
fn main() {
    // 同一个 NumDeserializer, 服务四种完全不同的目标类型:
    let a: i32 = i32::deserialize(NumDeserializer).unwrap();
    println!("i32   <- 整数 token 300        = {a}");

    // u64 的 visitor 做了范围检查, 300 >= 0, 接受
    let b: u64 = u64::deserialize(NumDeserializer).unwrap();
    println!("u64   <- 整数 token 300        = {b}");

    // f64 的 visitor 做了 i64 -> f64 转换
    let c: f64 = f64::deserialize(NumDeserializer).unwrap();
    println!("f64   <- 整数 token 300        = {c}");

    // i8 的 visitor 范围检查失败: 300 > 127, 拒绝
    let d = i8::deserialize(NumDeserializer);
    println!("i8    <- 整数 token 300        = {:?}", d.err().map(|e| e.to_string()));

    // String 的 visitor 没有实现 visit_i64, 拒绝
    let e = String::deserialize(NumDeserializer);
    println!("String<- 整数 token 300        = {:?}", e.err().map(|e| e.to_string()));

    // 同一个自定义类型, 两种格式都能喂给它:
    let w1: Wrapped = Wrapped::deserialize(NumDeserializer).unwrap();
    let w2: Wrapped = Wrapped::deserialize(StrDeserializer { s: "42" }).unwrap();
    println!("Wrapped <- 两种格式的 token      = {w1:?}, {w2:?}");
}

// ============================================================
// 5. 如果没有 visitor, 会怎样?
// ============================================================
// 朴素设计: 每种类型一个方法, 返回具体类型:
//
//   impl NumDeserializer {
//       fn as_i32(&self) -> Result<i32, E> { ... }   // 只能服务 i32
//       fn as_u64(&self) -> Result<u64, E> { ... }   // 只能服务 u64
//   }
//
// 问题:
//   - N 种格式 x M 种类型 => 需要 N*M 份转换代码
//     (改一个格式要同步改所有类型, 反之亦然, 而 visitor 让两者独立演化)
//   - 格式不认识的自定义类型(如 Wrapped)永远无法接入 ——
//     Wrapped 能反序列化, 靠的是它自己实现了 Visitor, 而不是格式认识它
//   - seq/map 的递归(seed -> 子元素 deserializer -> 子元素 visitor)
//     无法抽象: 子元素的类型在写格式代码时根本不知道, 只有运行时的
//     DeserializeSeed 能拿到, 而 seed 的返回类型正是通过 Visitor 关联的
// ============================================================
