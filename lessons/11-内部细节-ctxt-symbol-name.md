# 第十一章: proc-macro 内部细节(一) —— Ctxt, Symbol, Name

**源码参考**: `serde_derive_internals/src/{ctxt.rs, symbol.rs, name.rs, case.rs}`

## Ctxt —— 错误收集器

```rust
// 源码: serde_derive_internals/src/ctxt.rs
pub struct Ctxt {
    errors: RefCell<Option<Vec<syn::Error>>>,
}

impl Ctxt {
    pub fn new() -> Self {
        Ctxt { errors: RefCell::Some(Vec::new()) }
    }

    // 添加错误(带 span)
    pub fn error_spanned_by<A: ToTokens, T: Display>(&self, obj: A, msg: T) {
        self.errors
            .borrow_mut()
            .as_mut()
            .unwrap()
            .push(syn::Error::new_spanned(obj.into_token_stream(), msg));
    }

    // 添加已有的 syn::Error
    pub fn syn_error(&self, err: syn::Error) {
        self.errors.borrow_mut().as_mut().unwrap().push(err);
    }

    // 消费所有错误,返回 Result
    pub fn check(self) -> syn::Result<()> {
        let mut errors = self.errors.borrow_mut().take().unwrap();
        match errors.len() {
            0 => Ok(()),
            1 => Err(errors.pop().unwrap()),
            n => {
                let mut combined = errors.pop().unwrap();
                for e in errors {
                    combined.combine(e);  // 合并多个错误
                }
                Err(combined)
            }
        }
    }
}

// 关键设计: Drop 时 panic 如果未 check()
impl Drop for Ctxt {
    fn drop(&mut self) {
        if !std::thread::panicking() && self.errors.borrow().is_some() {
            panic!("forgot to check Ctxt");
        }
    }
}
```

### 为什么要这样设计?

- **收集所有错误**: 属性解析过程中可能发现多个问题,全部收集,一次报告
- **防止遗忘**: Drop guard 确保错误一定被检查
- **RefCell 内部可变性**: 避免到处传 `&mut Ctxt`

```rust
// 使用模式:
let cx = Ctxt::new();

// 解析过程中可能多次调用
attr.set(&cx, value); // 内部调用 cx.error_spanned_by() 如果重复
// ...
validate(&cx, field); // 也可能添加错误

// 最后统一检查
cx.check()?; // 消费 Ctxt,返回第一个错误(或合并的错误)
```

## Symbol —— 属性名常量

```rust
// 源码: serde_derive_internals/src/symbol.rs
#[derive(Copy, Clone)]
pub struct Symbol(pub &'static str);

// 34 个预定义的符号常量
pub const ALIAS: Symbol = Symbol("alias");
pub const BORROW: Symbol = Symbol("borrow");
pub const BOUND: Symbol = Symbol("bound");
pub const CONTENT: Symbol = Symbol("content");
pub const CRATE: Symbol = Symbol("crate");
pub const DEFAULT: Symbol = Symbol("default");
pub const DENY_UNKNOWN_FIELDS: Symbol = Symbol("deny_unknown_fields");
pub const DESERIALIZE_WITH: Symbol = Symbol("deserialize_with");
pub const EXPECTING: Symbol = Symbol("expecting");
pub const FIELD_IDENTIFIER: Symbol = Symbol("field_identifier");
pub const FLATTEN: Symbol = Symbol("flatten");
pub const FROM: Symbol = Symbol("from");
pub const GETTER: Symbol = Symbol("getter");
pub const INTO: Symbol = Symbol("into");
pub const NON_EXHAUSTIVE: Symbol = Symbol("non_exhaustive");
pub const OTHER: Symbol = Symbol("other");
pub const REMOTE: Symbol = Symbol("remote");
pub const RENAME: Symbol = Symbol("rename");
pub const RENAME_ALL: Symbol = Symbol("rename_all");
pub const RENAME_ALL_FIELDS: Symbol = Symbol("rename_all_fields");
pub const REPR: Symbol = Symbol("repr");
pub const SERIALIZE_WITH: Symbol = Symbol("serialize_with");
pub const SKIP: Symbol = Symbol("skip");
pub const SKIP_DESERIALIZING: Symbol = Symbol("skip_deserializing");
pub const SKIP_SERIALIZING: Symbol = Symbol("skip_serializing");
pub const SKIP_SERIALIZING_IF: Symbol = Symbol("skip_serializing_if");
pub const TAG: Symbol = Symbol("tag");
pub const TRANSPARENT: Symbol = Symbol("transparent");
pub const TRY_FROM: Symbol = Symbol("try_from");
pub const UNTAGGED: Symbol = Symbol("untagged");
pub const VARIANT_IDENTIFIER: Symbol = Symbol("variant_identifier");
pub const WITH: Symbol = Symbol("with");

// PartialEq 实现,方便与 Ident/Path 比较
impl PartialEq<Symbol> for syn::Ident {
    fn eq(&self, symbol: &Symbol) -> bool {
        self == symbol.0  // 直接比较字符串
    }
}

impl PartialEq<Symbol> for &syn::Ident {
    fn eq(&self, symbol: &Symbol) -> bool {
        *self == symbol.0
    }
}
```

这样在属性解析时可以写作:

```rust
if meta.path == RENAME {
    // ...
} else if meta.path == ALIAS {
    // ...
}
```

## Name 和 MultiName —— 名称管理

```rust
// 源码: serde_derive_internals/src/name.rs

// 简单的名称 + Span 对
#[derive(Clone)]
pub struct Name {
    pub value: String,
    pub span: Span,
}

// MultiName 是名称管理的核心:
// 序列化和反序列化可以有不同的名称
#[derive(Clone)]
pub struct MultiName {
    serialize: Name,
    serialize_renamed: bool,       // 显式 rename 了?
    deserialize: Name,
    deserialize_renamed: bool,     // 显式 rename 了?
    deserialize_aliases: BTreeSet<Name>,  // 反序列化别名
}

impl MultiName {
    pub fn from_attrs(
        source_name: Name,       // Rust 中的原始名称
        ser_name: Option<Name>,  // #[serde(rename = "...")] 的值
        de_name: Option<Name>,
        de_aliases: Option<BTreeSet<Name>>,  // #[serde(alias = "...")]
    ) -> Self {
        // 逻辑:
        // serialize = ser_name.unwrap_or(source_name)
        // deserialize = de_name.unwrap_or(source_name)
        // serialize_renamed = ser_name.is_some()
        // deserialize_renamed = de_name.is_some()
        // deserialize_aliases = de_aliases.unwrap_or_default()
    }
}

// 应用 rename rules:
impl MultiName {
    pub fn serialize_name(&self) -> &str { &self.serialize.value }
    pub fn deserialize_name(&self) -> &str { &self.deserialize.value }

    // rename_by_rules: 如果用户没显式 rename,则应用 rename_all 规则
    // 源码: serde_derive_internals/src/ast.rs ~170
}
```

### rename_all 规则的优先级

```rust
// 枚举:
// 1. #[serde(rename = "...")] 在 variant 上 —— 最高优先级
// 2. #[serde(rename_all = "...")] 在枚举上 —— 应用于变体名
// 3. #[serde(rename_all_fields = "...")] 在枚举上 —— 应用于字段名
// 4. 变体级的 #[serde(rename_all = "...")] —— 应用于该变体的字段

// 结构体:
// 1. #[serde(rename = "...")] 在字段上 —— 最高优先级
// 2. #[serde(rename_all = "...")] 在结构体上

// 原理: 显式 rename 会设置 serialize_renamed = true,
//       阻止后续 rename_all 覆盖它
if !multi_name.serialize_renamed {
    multi_name.serialize.value = rename_rule.apply(&source.value);
}
```

## RenameRule 实现

```rust
// 源码: serde_derive_internals/src/case.rs

impl RenameRule {
    // snake_case 核心算法
    fn apply_to_snake_case(&self, name: &str) -> String {
        let mut snake = String::with_capacity(name.len());
        for (i, ch) in name.char_indices() {
            if ch.is_uppercase() {
                // 除了开头的大写前加 _
                if i > 0 {
                    // 处理连续大写: HTTPResponse → http_response
                    if !name[..i].ends_with(|c: char| c.is_uppercase()) {
                        snake.push('_');
                    } else if i + 1 < name.len()
                        && name[i + 1..].starts_with(|c: char| c.is_lowercase())
                    {
                        snake.push('_');
                    }
                }
                snake.extend(ch.to_lowercase());
            } else {
                snake.push(ch);
            }
        }
        snake
    }
}
```

---

**练习**:
1. 阅读 `Ctxt` 的 drop 实现,理解为什么需要检查 `thread::panicking()`
2. 给 `Name::from_attrs` 添加注释,解释 `serialize_renamed` 的作用
3. 为 `case.rs` 中的 snake_case 写单元测试:`CamelCase` → `camel_case`, `HTTPResponse` → `http_response`
