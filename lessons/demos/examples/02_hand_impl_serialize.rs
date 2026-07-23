/// 第二章/第三章 demo: 手工实现 Serialize
///
/// 运行: cargo run --example 02_hand_impl_serialize
///
/// 手工实现 Serialize 需要:
/// 1. 实现 Serialize trait 的 serialize 方法
/// 2. 在 serialize 中调用 Serializer 的**恰好一个**方法
/// 3. 复合类型需要通过状态机关联类型逐个序列化子元素

use serde::ser::{SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTupleStruct};
use serde::{Serialize, Serializer};

// ============================================================
// 示例 1: 手工实现 Duration 的 Serialize
// ============================================================
struct Duration {
    secs: u64,
    nanos: u32,
}

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 调用 serializer.serialize_struct —— 恰好一个方法!
        // "Duration":  类型名(仅用于人类可读格式)
        // 2:           字段数
        let mut state = serializer.serialize_struct("Duration", 2)?;
        state.serialize_field("secs", &self.secs)?;
        state.serialize_field("nanos", &self.nanos)?;
        state.end() // 结束,返回最终结果
    }
}

// ============================================================
// 示例 2: 手工实现元组结构体的 Serialize
// ============================================================
struct Color(u8, u8, u8);

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 元组结构体 → serialize_tuple_struct
        let mut ts = serializer.serialize_tuple_struct("Color", 3)?;
        ts.serialize_field(&self.0)?;
        ts.serialize_field(&self.1)?;
        ts.serialize_field(&self.2)?;
        ts.end()
    }
}

// ============================================================
// 示例 3: 手工实现枚举的 Serialize(外部标签)
// ============================================================
#[derive(Debug, PartialEq)]
enum Command {
    Login { user: String, pass: String },
    Logout,
    Send(String),
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Command::Login { user, pass } => {
                // struct variant → serialize_struct_variant
                let mut sv = serializer.serialize_struct_variant(
                    "Command", 0, "Login", 2,
                )?;
                sv.serialize_field("user", user)?;
                sv.serialize_field("pass", pass)?;
                sv.end()
            }
            Command::Logout => {
                // unit variant → serialize_unit_variant
                serializer.serialize_unit_variant("Command", 1, "Logout")
            }
            Command::Send(msg) => {
                // newtype variant → serialize_newtype_variant
                serializer.serialize_newtype_variant(
                    "Command", 2, "Send", msg,
                )
            }
        }
    }
}

// ============================================================
// 示例 4: Vec-like 类型的 Serialize
// ============================================================
struct Stack<T> {
    items: Vec<T>,
}

impl<T: Serialize> Serialize for Stack<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.items.len()))?;
        for item in &self.items {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

// ============================================================
// 示例 5: newtype 模式的 Serialize
// ============================================================
struct UserId(u64);

impl Serialize for UserId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // transparent 模式: 直接序列化内层值
        // serialize_newtype_struct 用于保留类型名信息
        // 某些格式(如 JSON)会选择忽略名字,只序列化 u64
        serializer.serialize_newtype_struct("UserId", &self.0)
    }
}

// ============================================================
// 测试验证
// ============================================================
fn main() {
    println!("=== 1. Duration struct ===");
    let d = Duration { secs: 3600, nanos: 500_000_000 };
    let json = serde_json::to_string(&d).unwrap();
    println!("Duration: {}", json);
    assert_eq!(json, r#"{"secs":3600,"nanos":500000000}"#);

    println!("\n=== 2. Color tuple struct ===");
    let c = Color(255, 128, 0);
    let json = serde_json::to_string(&c).unwrap();
    println!("Color: {}", json);
    assert_eq!(json, "[255,128,0]");

    println!("\n=== 3. Command enum ===");
    let login = Command::Login { user: "admin".into(), pass: "secret".into() };
    let logout = Command::Logout;
    let send = Command::Send("hello".into());

    println!("Login:  {}", serde_json::to_string(&login).unwrap());
    println!("Logout: {}", serde_json::to_string(&logout).unwrap());
    println!("Send:   {}", serde_json::to_string(&send).unwrap());

    println!("\n=== 4. Stack (Vec-like) ===");
    let stack = Stack { items: vec![1, 2, 3] };
    let json = serde_json::to_string(&stack).unwrap();
    println!("Stack: {}", json);
    assert_eq!(json, "[1,2,3]");

    println!("\n=== 5. UserId (newtype) ===");
    let uid = UserId(42);
    let json = serde_json::to_string(&uid).unwrap();
    println!("UserId: {}", json);
    // JSON 中 newtype 是透明的(大多数格式如此)
    assert_eq!(json, "42");

    println!("\n=== 6. 使用 serde_test 做无格式测试 ===");
    use serde_test::{assert_ser_tokens, Token};
    // serde_test 可以绕过真实格式,直接验证 token 流
    assert_ser_tokens(&d, &[
        Token::Struct { name: "Duration", len: 2 },
        Token::Str("secs"),
        Token::U64(3600),
        Token::Str("nanos"),
        Token::U32(500_000_000),
        Token::StructEnd,
    ]);
    println!("serde_test assertions passed!");
}
