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
///
/// 第 4-5 节补充: 枚举反序列化的调用链(EnumAccess + VariantAccess),
///   以及「匹配即提交, 不回退」与 untagged「逐个尝试」的对比。

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
// 4. 枚举反序列化: EnumAccess + VariantAccess, 「匹配即提交, 不回退」
// ============================================================
// JSON 形态 {"Move": 42} / {"Say": "hi"} / "Quit":
//
//   deserialize_enum
//     └─ visitor.visit_enum(EnumAccess)        格式方: (变体名, 载荷) 都在这, 你决定
//         └─ data.variant::<String>()          类型方读变体名 —— 读一次就被消费
//             └─ match 变体名                    匹配即提交!
//                 ├─ "Quit" → unit_variant()
//                 ├─ "Move" → newtype_variant::<i64>()
//                 └─ 其他   → unknown_variant 直接报错, 不会试别的分支
//         注意: 变体名一旦匹配, 载荷反序列化失败也不会回退到别的变体

// 极简枚举格式: 变体名 token + 载荷 token
struct EnumDeserializer<'de> {
    variant: &'de str,
    payload: &'de str,
}

impl<'de> Deserializer<'de> for EnumDeserializer<'de> {
    type Error = de::value::Error;

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(EnumAccessImpl {
            variant: self.variant,
            payload: self.payload,
        })
    }

    // forward 宏把其他入口都转发到这里; 本格式只有 (变体名, 载荷) 两个 token,
    // 只有 deserialize_enum 有定义, 其余入口一律明确报错
    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(de::Error::custom(
            "EnumDeserializer only supports deserialize_enum",
        ))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct identifier ignored_any
    }
}

// EnumAccess: 格式方实现, 把"变体名"交给 seed 反序列化, 然后交出载荷
struct EnumAccessImpl<'de> {
    variant: &'de str,
    payload: &'de str,
}

impl<'de> de::EnumAccess<'de> for EnumAccessImpl<'de> {
    type Error = de::value::Error;
    type Variant = VariantAccessImpl<'de>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        // ① 变体名 token 在这里被消费 —— 之后的 match 只能在这几个名字里选
        let name = seed.deserialize(StrDeserializer { s: self.variant })?;
        Ok((name, VariantAccessImpl { payload: self.payload }))
    }
}

// VariantAccess: 格式方实现, 只负责"把载荷反序列化出来", 不管是什么变体
struct VariantAccessImpl<'de> {
    payload: &'de str,
}

// 载荷 token 的形状由内容决定 (模拟真实 JSON: {"Move": 42} 是整数 token,
// {"Move": "42"} 是字符串 token): 能解析成整数就报整数, 否则报字符串
struct PayloadDeserializer<'de>(&'de str);

impl<'de> Deserializer<'de> for PayloadDeserializer<'de> {
    type Error = de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0.parse::<i64>() {
            Ok(n) => visitor.visit_i64(n),
            Err(_) => visitor.visit_str(self.0),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> de::VariantAccess<'de> for VariantAccessImpl<'de> {
    type Error = de::value::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(PayloadDeserializer(self.payload))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(de::Error::custom("tuple variant not supported in this demo"))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(de::Error::custom("struct variant not supported in this demo"))
    }
}

// 类型侧: 手写 Deserialize, 形状和 derive 生成的一模一样 (见 serde_derive/src/de/enum_externally.rs)
#[derive(Debug)]
enum Command {
    Quit,
    Move(i64),
    Say(String),
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct CommandVisitor;

        impl<'de> Visitor<'de> for CommandVisitor {
            type Value = Command;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("enum Command")
            }

            fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<Command, A::Error> {
                use de::VariantAccess;

                // ① 读变体名 —— 格式方在这里消费 token, 一旦返回就"认定"了
                let (name, payload) = data.variant::<String>()?;

                // ② match 命中即提交: 就算载荷失败, 也不会回头试别的分支
                match name.as_str() {
                    "Quit" => {
                        payload.unit_variant()?;
                        Ok(Command::Quit)
                    }
                    "Move" => Ok(Command::Move(payload.newtype_variant()?)),
                    "Say" => Ok(Command::Say(payload.newtype_variant()?)),
                    other => Err(de::Error::unknown_variant(other, &["Quit", "Move", "Say"])),
                }
            }
        }

        d.deserialize_enum("Command", &["Quit", "Move", "Say"], CommandVisitor)
    }
}

// ============================================================
// 5. 对比: untagged 枚举才"逐个尝试"
// ============================================================
// serde 的 #[serde(untagged)] 是唯一会回退的表示方式:
//   1. 先把整个输入缓冲成 Content (serde 的私有中间表示, 相当于 AST)
//   2. 按声明顺序, 对每个变体做一次完整的 Deserialize
//   3. 失败就丢弃错误试下一个; 全失败返回最后一次尝试的错误
// 代价: 不能流式解析, 不能借用输入, 顺序敏感。
// 下面用极简 token 手动演示同样的"先试整数, 不行再试文本":

#[derive(Debug)]
enum NumOrText {
    Num(i64),
    Text(String),
}

impl<'de> Deserialize<'de> for NumOrText {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct TryVisitor;

        impl<'de> Visitor<'de> for TryVisitor {
            type Value = NumOrText;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an integer or a string")
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<NumOrText, E> {
                Ok(NumOrText::Num(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<NumOrText, E> {
                // 尝试 1: 按整数解析 (失败不是报错, 是换下一种形状)
                if let Ok(n) = v.parse::<i64>() {
                    return Ok(NumOrText::Num(n));
                }
                // 尝试 2: 按文本收下
                Ok(NumOrText::Text(v.to_owned()))
            }
        }

        d.deserialize_any(TryVisitor)
    }
}

// ============================================================
// 6. 运行
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
    println!("Wrapped <- 两种格式的 token     = {w1:?}, {w2:?}");

    // ---- 枚举: 调用链 & 不回退 ----
    let quit = Command::deserialize(EnumDeserializer { variant: "Quit", payload: "" }).unwrap();
    println!("Command <- Quit token          = {quit:?}");

    let mv = Command::deserialize(EnumDeserializer { variant: "Move", payload: "42" }).unwrap();
    println!("Command <- Move + \"42\"         = {mv:?}");

    let say = Command::deserialize(EnumDeserializer { variant: "Say", payload: "hello" }).unwrap();
    println!("Command <- Say + \"hello\"       = {say:?}");

    // 未知变体名: 直接报错, 不会"再试试别的名字"
    let bad = Command::deserialize(EnumDeserializer { variant: "Boom", payload: "" });
    println!("Command <- 未知变体 \"Boom\"     = {:?}", bad.err().map(|e| e.to_string()));

    // 关键: 变体名已匹配 Move, 载荷 \"abc\" 当 i64 解析失败 ——
    // 即使 Say(String) 能接受 \"abc\", 也绝不回退
    let stuck = Command::deserialize(EnumDeserializer { variant: "Move", payload: "abc" });
    println!("Command <- Move + \"abc\"        = {:?}", stuck.err().map(|e| e.to_string()));

    // ---- 枚举: untagged 式逐个尝试 ----
    let n1: NumOrText = NumOrText::deserialize(NumDeserializer).unwrap();
    let n2: NumOrText = NumOrText::deserialize(StrDeserializer { s: "42" }).unwrap();
    let n3: NumOrText = NumOrText::deserialize(StrDeserializer { s: "hello" }).unwrap();
    println!("NumOrText <- 3 种 token        = {n1:?}, {n2:?}, {n3:?}");
}

// ============================================================
// 7. 如果没有 visitor, 会怎样?
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
//   - 枚举变体名 token 在 EnumAccess::variant() 时被消费一次就没了,
//     所以"变体名匹配 → 载荷失败"无法回退 ——
//     想支持多形状只能靠 untagged 的先缓冲、再逐个尝试(见第 5 节)
// ============================================================
