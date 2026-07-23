/// 第一章 demo: 序言 —— serde 是什么
///
/// 运行: cargo run --example 01_basic_usage
///
/// Serde 是一个框架,不是一种数据格式。
/// 它定义了 Serialize/Deserialize trait,数据格式实现 Serializer/Deserializer。

use serde::{Deserialize, Serialize};

// ============================================================
// 1. 最基本的用法: derive 宏
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

// ============================================================
// 2. Serde 与数据格式解耦 —— 同样的类型,不同的格式
// ============================================================
fn basic_demo() {
    let point = Point { x: 10, y: 20 };

    // JSON 格式
    let json = serde_json::to_string(&point).unwrap();
    println!("JSON:  {}", json);
    let back: Point = serde_json::from_str(&json).unwrap();
    assert_eq!(point, back);

    // JSON 格式化
    let pretty = serde_json::to_string_pretty(&point).unwrap();
    println!("JSON pretty:\n{}", pretty);
}

// ============================================================
// 3. 枚举示例
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Message {
    // unit variant: 无数据
    Ping,
    // newtype variant: 单个值
    Text(String),
    // tuple variant: 多个位置值
    Move { x: i32, y: i32 },
    // struct variant: 命名字段
    Quit,
}

fn enum_demo() {
    let msgs = vec![
        Message::Ping,
        Message::Text("hello".to_string()),
        Message::Move { x: 10, y: 20 },
        Message::Quit,
    ];

    for msg in &msgs {
        let json = serde_json::to_string(msg).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();
        println!("{:?}  →  {}  →  {:?}", msg, json, back);
        assert_eq!(msg, &back);
    }
}

// ============================================================
// 4. 泛型类型
// ============================================================
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ApiResponse<T> {
    code: u32,
    message: String,
    data: T,
}

fn generic_demo() {
    let resp = ApiResponse {
        code: 200,
        message: "OK".to_string(),
        data: Point { x: 5, y: 5 },
    };
    let json = serde_json::to_string(&resp).unwrap();
    println!("Generic response: {}", json);
    let back: ApiResponse<Point> = serde_json::from_str(&json).unwrap();
    assert_eq!(resp, back);
}

fn main() {
    println!("=== 1. Basic derive demo ===");
    basic_demo();

    println!("\n=== 2. Enum demo ===");
    enum_demo();

    println!("\n=== 3. Generic demo ===");
    generic_demo();

    println!("\n=== 4. Serde data model summary ===");
    println!("Rust types are mapped to the serde data model via Serialize");
    println!("Data formats (JSON, etc.) implement Serializer/Deserializer");
    println!("This is the core of serde's design: types ↔ data model ↔ format");
}
