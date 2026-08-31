# 第八章: serde_derive —— proc-macro 生成代码

**源码参考**: `serde_derive/src/lib.rs`, `serde_derive/src/ser.rs`, `serde_derive/src/de.rs`

## 架构概览

`serde_derive` 是一个 **proc-macro crate**,提供 `#[derive(Serialize)]` 和 `#[derive(Deserialize)]`:

```
用户源码                           proc-macro 处理
┌─────────────────┐               ┌──────────────────────┐
│ #[derive(Ser...)]│  编译时调用   │ serde_derive         │
│ struct Point {   │──────────────>│  ├─ 解析 AST          │
│     x: i32,      │               │  ├─ 解析属性          │
│     y: i32,      │               │  ├─ 验证约束          │
│ }                │<──────────────│  └─ 生成 impl 代码    │
└─────────────────┘  返回 TokenStream └──────────────────────┘
```

## 入口点

```rust
// 源码: serde_derive/src/lib.rs
#[proc_macro_derive(Serialize, attributes(serde))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    ser::expand_derive_serialize(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Deserialize, attributes(serde))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    de::expand_derive_deserialize(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
```

关键参数 `attributes(serde)` 告诉编译器: 如果 `#[serde(...)]` 属性出现在没有 `#[derive(Serialize)]` 的类型上,会产生 unused attribute 警告。

## 处理流程(8 步)

### 步骤 1: 解析 TokenStream → syn::DeriveInput

```rust
let input = parse_macro_input!(input as DeriveInput);
// input.ident               = "Point"
// input.generics            = <T: Clone>
// input.data                = Struct { fields: ... }
// input.attrs               = #[serde(rename_all = "camelCase")]
```

### 步骤 2: Self → 具体类型替换

```rust
// 源码: serde_derive_internals/src/receiver.rs
replace_receiver(&mut input);
// 将 Self 替换为 MyStruct<T> —— proc-macro 卫生所需
```

### 步骤 3: 创建错误上下文

```rust
// 源码: serde_derive_internals/src/ctxt.rs
let cx = Ctxt::new();
// 收集所有错误,一次报告给用户
// Drop 时如果未 check() 则 panic(强制检查错误)
```

### 步骤 4: 构建内部 AST

```rust
// 源码: serde_derive_internals/src/ast.rs
let cont = Container::from_ast(&cx, &input, Derive::Serialize, &private);
// 解析所有 #[serde(...)] 属性
// 分类 Style: Struct / Tuple / Newtype / Unit
// 应用 rename_all 规则
// 标记 packed 结构
```

### 步骤 5: 交叉验证

```rust
// 源码: serde_derive_internals/src/check.rs
check::check(&cx, &mut cont, Derive::Serialize);
// 验证:
//  - transparent 恰好一个非跳过字段
//  - flatten 不允许在 newtype/tuple 上
//  - tag/content 字段名不冲突
//  - 等等
cx.check()?; // 返回第一个错误(如果有)
```

### 步骤 6: 构建参数(泛型、生命周期、Trait bounds)

```rust
// Parameters 包含了:
// - 类型路径(local vs remote)
// - 泛型参数及其 bounds
// - 生命周期(Deserialize 的 'de)
let params = Parameters::new(&cont);
```

### 步骤 7: 生成代码

```rust
let (impl_generics, ty_generics, where_clause) = params.generics.split_for_impl();
let body = serialize_body(&cont, &params);

let code = quote! {
    #[automatically_derived]
    impl #impl_generics serde::Serialize for #ident #ty_generics #where_clause {
        fn serialize<__S>(&self, __serializer: __S) -> Result<__S::Ok, __S::Error>
        where
            __S: serde::Serializer,
        {
            #body
        }
    }
};
```

### 步骤 8: 包装返回

```rust
// 源码: serde_derive/src/dummy.rs
// 包装在 const _: () = { ... } 中抑制 dead_code 警告
let code = dummy::wrap_in_const(serde_path, code);
// 转换 TokenStream2 → proc_macro::TokenStream
code.into()
```

## Serialize derive 代码生成详解

### 生成示例

```rust
// 输入:
#[derive(Serialize)]
struct Point { x: i32, y: i32 }

// 生成(简化):
const _: () = {
    extern crate serde as _serde;
    _serde::__require_serde_not_serde_core!();  // 编译期检查使用正确的 crate

    #[automatically_derived]
    impl _serde::Serialize for Point {
        fn serialize<__S>(&self, __serializer: __S) -> Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = __serializer.serialize_struct("Point", 2u32 as usize)?;
            _serde::ser::SerializeStruct::serialize_field(&mut __serde_state, "x", &self.x)?;
            _serde::ser::SerializeStruct::serialize_field(&mut __serde_state, "y", &self.y)?;
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
```

### 分发逻辑

```rust
// 源码: serde_derive/src/ser.rs:serialize_body()
fn serialize_body(cont: &Container, params: &Parameters) -> TokenStream {
    match cont.data {
        // transparent: 只序列化标记的字段
        Data::Struct(Style::Struct, ref fields)
            if cont.attrs.transparent() =>
            serialize_transparent(cont, params, fields),

        // into: 先转换为中间类型再序列化
        Data::Struct(..) if cont.attrs.into().is_some() =>
            serialize_into(cont, params),

        // 普通类型
        Data::Enum(ref variants)     => serialize_enum(params, variants, cont),
        Data::Struct(Style::Unit, _) => serialize_unit_struct(params, cont),
        Data::Struct(Style::Newtype, ref fields) => serialize_newtype_struct(params, fields),
        Data::Struct(Style::Tuple, ref fields)   => serialize_tuple_struct(params, fields),
        Data::Struct(Style::Struct, ref fields)  => serialize_struct(params, fields, cont),
    }
}
```

### 枚举序列化代码生成

```rust
// 输入:
#[derive(Serialize)]
enum Event { Click { x: u32, y: u32 }, KeyPress(char) }

// 生成的 serialize 方法(简化):
fn serialize<__S>(&self, __serializer: __S) -> Result<__S::Ok, __S::Error>
where
    __S: _serde::Serializer,
{
    match *self {
        Event::Click { ref x, ref y } => {
            let mut __serde_state = __serializer.serialize_struct_variant(
                "Event", 0u32, "Click", 2u32 as usize
            )?;
            _serde::ser::SerializeStructVariant::serialize_field(
                &mut __serde_state, "x", x
            )?;
            _serde::ser::SerializeStructVariant::serialize_field(
                &mut __serde_state, "y", y
            )?;
            _serde::ser::SerializeStructVariant::end(__serde_state)
        }
        Event::KeyPress(ref __field0) => {
            __serializer.serialize_newtype_variant(
                "Event", 1u32, "KeyPress", __field0
            )
        }
    }
}
```

### 不同 tag 策略的序列化

```rust
// 源码: serde_derive/src/ser.rs ~650-950

// Externally tagged: {"Variant": value}
// 用 serialize_*_variant 方法(如上例)

// Internally tagged: {"type": "Variant", ...fields...}
// 序列化为 struct,第一个字段是 tag
// 源码: serde_derive/src/ser.rs ~820
fn serialize_internally_tagged_enum(...) {
    // 创建 struct,先写 tag 字段,再写剩余字段
}

// Adjacently tagged: {"t": "Variant", "c": value}
// 创建一个 __AdjacentlyTagged 包装结构
// 源码: serde_derive/src/ser.rs ~870
fn serialize_adjacently_tagged_enum(...) {
    // 生成 __AdjacentlyTagged { t: &str, c: &Variant }
    // 然后序列化这个 struct
}

// Untagged: 直接序列化 variant 内容,无 tag
// 源码: serde_derive/src/ser.rs ~920
fn serialize_untagged_enum(...) {
    // 对于 unit variant → serialize_unit()
    // 对于 newtype variant → 序列化内层值
}
```

## Deserialize derive 代码生成详解

### 生成示例(结构体)

```rust
// 输入:
#[derive(Deserialize)]
struct Point { x: i32, y: i32 }

// 生成(简化):
const _: () = {
    extern crate serde as _serde;

    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Point {
        fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            // 定义字段名常量
            const FIELDS: &[&str] = &["x", "y"];

            // 创建 Visitor 并传给 deserializer
            __deserializer.deserialize_struct("Point", FIELDS, PointVisitor)
        }
    }

    // Visitor 结构体
    struct PointVisitor;

    impl<'de> _serde::de::Visitor<'de> for PointVisitor {
        type Value = Point;

        fn expecting(&self, f: &mut _serde::__private::fmt::Formatter) -> _serde::__private::fmt::Result {
            f.write_str("struct Point")
        }

        // visit_seq: 按顺序读
        fn visit_seq<__A>(self, mut __seq: __A) -> Result<Self::Value, __A::Error>
        where
            __A: _serde::de::SeqAccess<'de>,
        {
            let x = match __seq.next_element()? {
                Some(value) => value,
                None => return Err(_serde::de::Error::invalid_length(0usize, &"2 elements")),
            };
            let y = match __seq.next_element()? {
                Some(value) => value,
                None => return Err(_serde::de::Error::invalid_length(1usize, &"2 elements")),
            };
            Ok(Point { x, y })
        }

        // visit_map: 按字段名读
        fn visit_map<__A>(self, mut __map: __A) -> Result<Self::Value, __A::Error>
        where
            __A: _serde::de::MapAccess<'de>,
        {
            // 为每个字段声明 Option<T> 变量
            let mut __field_x: Option<i32> = None;
            let mut __field_y: Option<i32> = None;

            while let Some(__key) = __map.next_key::<Field>()? {
                // Field 是自动生成的字段名标识符枚举
                match __key {
                    Field::x => {
                        if __field_x.is_some() {
                            return Err(_serde::de::Error::duplicate_field("x"));
                        }
                        __field_x = Some(__map.next_value()?);
                    }
                    Field::y => {
                        if __field_y.is_some() {
                            return Err(_serde::de::Error::duplicate_field("y"));
                        }
                        __field_y = Some(__map.next_value()?);
                    }
                }
            }

            // 提取值,缺失的字段用 default 或报错
            let x = match __field_x {
                Some(x) => x,
                None => return Err(_serde::de::Error::missing_field("x")),
            };
            let y = match __field_y {
                Some(y) => y,
                None => return Err(_serde::de::Error::missing_field("y")),
            };

            Ok(Point { x, y })
        }
    }

    // 字段标识符枚举(用于 visit_map 的 key 匹配)
    enum Field { x, y }

    impl<'de> _serde::Deserialize<'de> for Field {
        fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            struct FieldVisitor;

            impl<'de> _serde::de::Visitor<'de> for FieldVisitor {
                type Value = Field;

                fn expecting(&self, f: &mut _serde::__private::fmt::Formatter) -> _serde::__private::fmt::Result {
                    f.write_str("field identifier")
                }

                fn visit_str<__E>(self, value: &str) -> Result<Field, __E>
                where
                    __E: _serde::de::Error,
                {
                    match value {
                        "x" => Ok(Field::x),
                        "y" => Ok(Field::y),
                        _ => Err(_serde::de::Error::unknown_field(value, FIELDS)),
                    }
                }

                fn visit_bytes<__E>(self, value: &[u8]) -> Result<Field, __E>
                where
                    __E: _serde::de::Error,
                {
                    match value {
                        b"x" => Ok(Field::x),
                        b"y" => Ok(Field::y),
                        _ => {
                            let value = &_serde::__private::from_utf8_lossy(value);
                            Err(_serde::de::Error::unknown_field(&value, FIELDS))
                        }
                    }
                }

                fn visit_u64<__E>(self, value: u64) -> Result<Field, __E>
                where
                    __E: _serde::de::Error,
                {
                    match value {  // 有些格式用索引代替字段名
                        0u64 => Ok(Field::x),
                        1u64 => Ok(Field::y),
                        _ => Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(value),
                            &"field index 0 <= i < 2"
                        )),
                    }
                }
            }

            __deserializer.deserialize_identifier(FieldVisitor)
        }
    }
};
```

## 生成的 Visitor 支持两种反序列化路径

对于一个 struct,生成的 Visitor 同时支持 `visit_seq` 和 `visit_map`:

```
JSON 对象: {"x": 1, "y": 2}   → visit_map(按字段名)
JSON 数组: [1, 2]              → visit_seq(按顺序)
某些格式: MessagePack Array    → visit_seq
某些格式: MessagePack Map      → visit_map
```

这使得同一个 Rust 类型可以从多种数据格式反序列化。

---

## 看真实的展开代码: cargo expand

serde_derive 真正的生成代码可以现场看,仓库里已经存了一份:
`lessons/demos/examples/05_expand.rs`。

**它是什么**: `cargo expand` 命令的输出 —— 把
`05_custom_deserializer.rs` 里所有宏(`#[derive(...)]` 等)完全展开后的源码快照。

**为什么需要 nightly 才能编译**: 展开后的代码大量引用编译器/标准库内部实现,
文件头部已预置 4 个 feature 门,缺一不可(实测移除任一个都会报错):

```rust
#![feature(structural_match, core_intrinsics, print_internals, fmt_helpers_for_derive)]
#![feature(prelude_import)]   // 原本就在展开输出里
```

- `structural_match` → `StructuralPartialEq`(derive PartialEq 的展开,第 407 行)
- `core_intrinsics` / `fmt_helpers_for_derive` → derive Debug/Display 的展开
- `print_internals` → `println!` 宏的展开
- 此外 `_serde::__private229` 是 serde 构建期生成的公开(doc-hidden)私有模块,
  229 是版本号后缀(见 `serde_derive/src/lib.rs` 的 `concat!("__private", ...)`),
  只要 serde 版本与生成时一致就能解析

**运行方式**: nightly 下可以直接运行验证(输出与 `05_custom_deserializer` 相同):

```bash
cargo +nightly run --example 05_expand
```

stable 下编译会失败(`#![feature]` 不允许),这是预期的 —— 它是**给人类读的
参考文档**,不是常规示例。常规跑 demo 用原文件:

```bash
cargo run --example 05_custom_deserializer
```

重新生成快照(即 `lessons/demos/run.sh` 的内容):

```bash
cargo expand -p learn-serde-demos --example 05_custom_deserializer > lessons/demos/examples/05_expand.rs
```

(需要 nightly + 已安装 `cargo-expand`。)

## 为什么每个复合类型生成两个 Visitor?

在 `05_expand.rs` 里可以看到,`#[derive(Deserialize)]` 为每个复合类型生成
**两个 Visitor**: `__FieldVisitor`(第 441 行)和 `__Visitor`(第 510 行)。
struct、map、enum 无一例外。为什么?

两个 Visitor 的职责不同,处理的 token 层级不同:

| | `__Visitor`(主 Visitor) | `__FieldVisitor`(标识符 Visitor) |
|--|--|--|
| 处理什么 | **整个结构体/枚举**(一整段复合 token 流) | **单个字段名/变体名 token** |
| 传给谁 | `deserialize_struct` / `deserialize_enum`(05_expand.rs:523, 789) | `deserialize_identifier`(05_expand.rs:503) |
| 实现的方法 | `visit_map` / `visit_seq` / `visit_enum` | 只 `visit_str` / `visit_bytes` / `visit_u64` |
| 产出 | 最终值 `User` / `Command` | `__Field` 枚举(`__field0`/`__field1`/`__ignore`) |

原因:**反序列化是递归的,键也是"值"**。主 Visitor 的 visit_map 循环里:

```rust
while let Some(key) = map.next_key::<__Field>()? {   // ← 键也要反序列化!
    // next_key 内部: __Field::deserialize(identifier_deserializer)
    //    └─ 需要一个 Visitor 来处理键 token → __FieldVisitor
    match key {
        __Field::__field0 => { ... map.next_value()? ... }  // ← 值由主 Visitor 流程消费
        ...
    }
}
```

而 Visitor 的协议是"**一个 Visitor 接收一种 token 流**":
- 主 `__Visitor` 要接收 Map/Seq 整体 —— 整段键值对流
- `__FieldVisitor` 只接收一个字符串/数字 token

两类 token 形态不同,无法共用同一个 Visitor,所以必须成对生成。

**嵌套会让 Visitor 变多**: 每个"容器层"都贡献一对 Visitor。例如 enum 的
struct variant(05_expand.rs:1035-1109)的载荷又是一个 struct,又生成一对
`__FieldVisitor` + `__Visitor`。一个含 struct variant 的 enum 展开后共有 4 个
Visitor 定义:主 `__Visitor` + 主 `__FieldVisitor` + variant 的 `__Visitor` +
variant 的 `__FieldVisitor`。

---

**练习**:
1. 使用 `cargo expand` 展开一个简单的 derive 宏,观察生成的代码
2. 在 `serde_derive/src/ser.rs` 中找到 `serialize_struct` 函数,理解字段循环的逻辑
3. 在 `serde_derive/src/de/struct_.rs` 中找到 `deserialize_map` 函数,理解 visit_map 生成的逻辑
