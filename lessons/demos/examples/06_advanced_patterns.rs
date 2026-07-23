/// 高级 serde 模式 demo
///
/// 运行: cargo run --example 06_advanced_patterns
///
/// 涵盖: rename, alias, skip, default, flatten, transparent,
///       serialize_with/deserialize_with, tag 策略, with 模块

use serde::{Deserialize, Serialize};

// ============================================================
// 1. rename / rename_all / alias
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    status_code: u32,        // → "statusCode"
    error_message: String,   // → "errorMessage"
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum PaymentMethod {
    #[serde(rename = "credit_card")]
    CreditCard,
    #[serde(rename = "paypal")]
    PayPal,
    // 反序列化时同时接受旧名称
    #[serde(alias = "wire", alias = "bank_transfer")]
    BankTransfer,
}

fn rename_alias_demo() {
    println!("=== 1. rename / alias ===");

    let resp = ApiResponse {
        status_code: 200,
        error_message: "OK".into(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    println!("  camelCase: {}", json);

    // alias: 反序列化时接受多个名称
    let pm: PaymentMethod = serde_json::from_str(r#""wire""#).unwrap();
    println!("  alias 'wire' → {:?}", pm);
    let pm: PaymentMethod = serde_json::from_str(r#""bank_transfer""#).unwrap();
    println!("  alias 'bank_transfer' → {:?}", pm);
}

// ============================================================
// 2. skip / skip_serializing / skip_serializing_if / skip_deserializing
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct UserProfile {
    name: String,
    #[serde(skip)]    // 完全不参与序列化和反序列化
    password_hash: String,
    #[serde(skip_serializing)]  // 反序列化时接受,序列化时跳过
    secret_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bio: Option<String>,        // 为 None 时跳过
}

fn skip_demo() {
    println!("\n=== 2. skip / skip_serializing_if ===");

    let user = UserProfile {
        name: "Alice".into(),
        password_hash: "hash_value".into(),
        secret_key: "key_value".into(),
        bio: None,
    };

    let json = serde_json::to_string(&user).unwrap();
    println!("  serialized: {}", json);
    // password_hash(skip)和secret_key(skip_serializing)不会出现
    // bio is None, 被 skip_serializing_if 跳过
    assert!(!json.contains("password_hash"));
    assert!(!json.contains("secret_key"));
    assert!(!json.contains("bio"));
}

// ============================================================
// 3. default / default = "path"
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Config {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    timeout_secs: u32,        // u32::default() = 0
    #[serde(default)]
    tags: Vec<String>,        // Vec::default() = empty
}

fn default_port() -> u16 { 8080 }

fn default_demo() {
    println!("\n=== 3. default values ===");

    // 只提供 host, 其他用默认值
    let json = r#"{"host": "localhost"}"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    println!("  from '{:?}':", json);
    println!("    host={}, port={}, timeout={}, tags={:?}",
        cfg.host, cfg.port, cfg.timeout_secs, cfg.tags
    );
    assert_eq!(cfg.port, 8080);
    assert_eq!(cfg.timeout_secs, 0);
    assert!(cfg.tags.is_empty());
}

// ============================================================
// 4. flatten —— 扁平化嵌套结构
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PaginatedResponse<T> {
    page: u32,
    per_page: u32,
    total: u32,
    #[serde(flatten)]
    data: T,     // data 的字段展开到顶层
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct UsersData {
    users: Vec<String>,
}

fn flatten_demo() {
    println!("\n=== 4. flatten ===");

    let resp = PaginatedResponse {
        page: 1,
        per_page: 10,
        total: 100,
        data: UsersData { users: vec!["Alice".into(), "Bob".into()] },
    };

    let json = serde_json::to_string(&resp).unwrap();
    println!("  serialized: {}", json);

    // "users" 出现在顶层,不在嵌套对象中
    assert!(json.contains("\"users\""));

    let back: PaginatedResponse<UsersData> = serde_json::from_str(&json).unwrap();
    assert_eq!(resp, back);
}

// ============================================================
// 5. transparent —— 透明 newtype
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(transparent)]
struct Email(String);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Contact {
    name: String,
    email: Email,
}

fn transparent_demo() {
    println!("\n=== 5. transparent ===");

    let contact = Contact {
        name: "Alice".into(),
        email: Email("alice@example.com".into()),
    };
    let json = serde_json::to_string(&contact).unwrap();
    println!("  Contact with transparent Email: {}", json);

    // Email 被序列化为普通字符串(不是 {"Email": "..."})
    assert!(json.contains("alice@example.com"));
}

// ============================================================
// 6. serialize_with / deserialize_with
// ============================================================
use serde::de::{self, Visitor};
use serde::Serializer;
use std::fmt;

fn serialize_bool_as_int<S: Serializer>(v: &bool, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u8(if *v { 1 } else { 0 })
}

fn deserialize_int_as_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    struct IntBoolVisitor;
    impl<'de> Visitor<'de> for IntBoolVisitor {
        type Value = bool;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("0 or 1")
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
            match v { 0 => Ok(false), 1 => Ok(true), _ => Err(E::custom("expected 0 or 1")) }
        }
    }
    d.deserialize_u64(IntBoolVisitor)
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct FeatureFlags {
    #[serde(serialize_with = "serialize_bool_as_int")]
    #[serde(deserialize_with = "deserialize_int_as_bool")]
    enabled: bool,
}

fn custom_ser_de_demo() {
    println!("\n=== 6. serialize_with / deserialize_with ===");

    let flags = FeatureFlags { enabled: true };
    let json = serde_json::to_string(&flags).unwrap();
    println!("  bool → int: {}", json);
    assert_eq!(json, r#"{"enabled":1}"#);

    let back: FeatureFlags = serde_json::from_str(r#"{"enabled":0}"#).unwrap();
    println!("  int → bool: {:?}", back);
}

// ============================================================
// 7. with = "module" —— 模块化的自定义 ser/de
// ============================================================

// 需要 hex crate,这里用简化版
mod fake_hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i+2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

mod hex_string {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::fake_hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        super::fake_hex::decode(&s).map_err(de::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct BinaryData {
    #[serde(with = "hex_string")]
    payload: Vec<u8>,
}

fn with_module_demo() {
    println!("\n=== 7. with = \"module\" ===");

    let data = BinaryData { payload: vec![0xDE, 0xAD, 0xBE, 0xEF] };
    let json = serde_json::to_string(&data).unwrap();
    println!("  bytes → hex string: {}", json);
    assert_eq!(json, r#"{"payload":"deadbeef"}"#);

    let back: BinaryData = serde_json::from_str(&json).unwrap();
    println!("  hex string → bytes: {:?}", back);
}

// ============================================================
// 8. 枚举 tag 策略比较
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
enum InternallyTagged {
    Text { content: String },
    Image { url: String, width: u32 },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t", content = "c")]
enum AdjacentlyTagged {
    Text(String),
    Image { url: String },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum Untagged {
    Num(i32),
    Str(String),
    Obj { x: i32, y: i32 },
}

fn tag_strategies_demo() {
    println!("\n=== 8. Enum tag strategies ===");

    // Internal tag
    let msg = InternallyTagged::Text { content: "Hi".into() };
    let json = serde_json::to_string(&msg).unwrap();
    println!("  Internal tag: {}", json);
    let back: InternallyTagged = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, back);

    // Adjacent tag
    let msg = AdjacentlyTagged::Text("Hello".into());
    let json = serde_json::to_string(&msg).unwrap();
    println!("  Adjacent tag: {}", json);
    let back: AdjacentlyTagged = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, back);

    // Untagged
    let v: Untagged = serde_json::from_str("42").unwrap();
    println!("  Untagged: 42 → {:?}", v);

    let v: Untagged = serde_json::from_str(r#""hello""#).unwrap();
    println!("  Untagged: \"hello\" → {:?}", v);

    let v: Untagged = serde_json::from_str(r#"{"x":1,"y":2}"#).unwrap();
    println!("  Untagged: {{\"x\":1,\"y\":2}} → {:?}", v);
}

// ============================================================
// 9. deny_unknown_fields
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct StrictConfig {
    host: String,
    port: u16,
}

fn deny_unknown_demo() {
    println!("\n=== 9. deny_unknown_fields ===");

    let json = r#"{"host":"localhost","port":8080}"#;
    let cfg: StrictConfig = serde_json::from_str(json).unwrap();
    println!("  OK: {:?}", cfg);

    let json = r#"{"host":"localhost","port":8080,"extra":"nope"}"#;
    let result: Result<StrictConfig, _> = serde_json::from_str(json);
    println!("  Error for unknown field: {}", result.unwrap_err());
}

// ============================================================
// 10. from / into / try_from
// ============================================================
// 注意: 这里手工实现 Serialize/Deserialize 演示 into/try_from 模式
// 实际使用中可以用 #[derive(Serialize, Deserialize)] + #[serde(into = "String", try_from = "String")]

#[derive(Debug, PartialEq, Clone)]
struct NonEmptyString(String);

impl TryFrom<String> for NonEmptyString {
    type Error = &'static str;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() { Err("empty string") } else { Ok(NonEmptyString(s)) }
    }
}

impl From<NonEmptyString> for String {
    fn from(nes: NonEmptyString) -> String { nes.0 }
}

impl Serialize for NonEmptyString {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // 使用 into: 序列化为 String
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for NonEmptyString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 使用 try_from: 先反序列化为 String,再转换
        let s = String::deserialize(d)?;
        s.try_into().map_err(de::Error::custom)
    }
}

fn from_into_demo() {
    println!("\n=== 10. into / try_from (via Serialize/Deserialize) ===");

    let nes = NonEmptyString("hello".into());
    let json = serde_json::to_string(&nes).unwrap();
    println!("  Serialized: {}", json);

    let back: NonEmptyString = serde_json::from_str(r#""world""#).unwrap();
    println!("  Deserialized: {:?}", back);

    let result: Result<NonEmptyString, _> = serde_json::from_str(r#""""#);
    println!("  Empty string error: {}", result.unwrap_err());
}

// ============================================================
// Main
// ============================================================
fn main() {
    rename_alias_demo();
    skip_demo();
    default_demo();
    flatten_demo();
    transparent_demo();
    custom_ser_de_demo();
    with_module_demo();
    tag_strategies_demo();
    deny_unknown_demo();
    from_into_demo();

    println!("\n===== All advanced patterns demonstrated! =====");
}
