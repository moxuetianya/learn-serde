# 第十三章: proc-macro 内部细节(三) —— bound, fragment, dummy

**源码参考**: `serde_derive/src/{bound.rs, fragment.rs, dummy.rs, pretend.rs, deprecated.rs}`

## bound.rs —— Trait Bounds 生成

```rust
// 源码: serde_derive/src/bound.rs
// 自动为泛型参数添加 Trait bounds

// 核心问题: 对 struct S<T> { f: T }
// 生成的 impl 需要 T: Serialize,但如果字段是 #[serde(skip)] 则不需要

// 解决: 遍历所有字段的类型,找出实际参与序列化的泛型参数
pub fn without_defaults(generics: &syn::Generics) -> syn::Generics {
    // 移除泛型参数上的默认类型(derive 展开后默认类型已在 impl 中)
}

pub fn with_where_predicates(
    generics: &syn::Generics,
    predicates: &[syn::WherePredicate],
) -> syn::Generics {
    // 向 where 子句添加用户指定的额外 bounds
}

pub fn with_where_predicates_from_fields(
    cont: &Container,
    generics: &syn::Generics,
    from_field: fn(&attr::Field) -> &[syn::WherePredicate],
) -> syn::Generics {
    // 从字段的属性中收集 bounds
}
```

### FindTyParams —— 找到使用的泛型参数

```rust
// 源码: serde_derive/src/bound.rs ~100
// 这是一个 syn::visit::Visit 的实现
// 遍历类型 AST,找到所有被引用的泛型参数

struct FindTyParams {
    // 输出: 找到的泛型参数名集合
    found: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for FindTyParams {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        // 检查路径是否是泛型参数名
        if path.leading_colon.is_none()
            && path.segments.len() == 1
        {
            let ident = &path.segments[0].ident;
            // 检查 ident 是否在泛型参数列表中
            self.found.insert(ident.to_string());
        }
        // 继续递归遍历
        syn::visit::visit_path(self, path);
    }

    fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
        // 检查 lifetime 是否在泛型参数列表中("found" 包括 lifetimes)
        self.found.insert(lifetime.ident.to_string());
    }
}
```

### 完整的 bound 生成流程

```rust
// 对于:
struct S<T, U: Default> {
    #[serde(serialize_with = "custom")]
    a: T,              // T 用自定义函数,不需要 T: Serialize

    b: U,              // U 需要 U: Serialize + Default
}

// 生成的 impl:
// impl<T, U: Default + Serialize> Serialize for S<T, U>
//     // T 不需要 Serialize bound!
```

### ser_bound / de_bound 属性

```rust
// 用户自定义 bounds:
#[serde(bound(serialize = "T: MySerialize", deserialize = "T: MyDeserialize"))]
struct S<T> { value: T }

// 分别生成:
impl<T: MySerialize> Serialize for S<T> { ... }
impl<'de, T: MyDeserialize> Deserialize<'de> for S<T> { ... }

// #[serde(bound = "T: MyTrait")]
// 对序列化和反序列化使用相同的 bounds(除非分别指定)
```

## fragment.rs —— 代码片段抽象

```rust
// 源码: serde_derive/src/fragment.rs
// 问题: 代码生成中,同样的逻辑可能返回表达式或语句块
// 例如: 反序列化一个字段,有时作为赋值语句,有时作为 match 臂

pub enum Fragment {
    Expr(TokenStream),
    Block(TokenStream),
}

pub struct Expr(pub TokenStream);
pub struct Stmts(pub TokenStream);
pub struct Match(pub TokenStream);

// 根据上下文将 Block 转换为合适的格式
impl From<Stmts> for Expr {
    fn from(stmts: Stmts) -> Expr {
        Expr(quote!({ #stmts }))  // 语句块用 {} 包裹
    }
}

impl From<Stmts> for Match {
    fn from(stmts: Stmts) -> Match {
        Match(stmts.0)  // match 中不需要额外包裹
    }
}

// 宏便捷方法
macro_rules! quote_expr {
    ($($tt:tt)*) => { Expr(quote!($($tt)*)) };
}
macro_rules! quote_block {
    ($($tt:tt)*) => { Stmts(quote!($($tt)*)) };
}
```

使用场景在 `serde_derive/src/de/struct_.rs` 中:

```rust
// 生成 visit_seq 时,字段赋值在 match 臂上下文中(不需要 {})
// 生成 visit_map 时,字段赋值在 if let 块中(需要 {})
fn deserialize_field(..., as_match: bool) -> TokenStream {
    if as_match {
        quote! {
            __field = Some(__seq.next_element()?);
        }
    } else {
        quote! {{
            __field = Some(__seq.next_element()?);
        }}
    }
}
```

## dummy.rs —— 死代码抑制

```rust
// 源码: serde_derive/src/dummy.rs
// 将 impl 包装在 const _: () = { ... } 中

pub fn wrap_in_const(serde_path: Option<&syn::Path>, code: TokenStream) -> TokenStream {
    let use_serde = match serde_path {
        Some(path) => quote! { extern crate #path as _serde; },
        None => quote! { extern crate serde as _serde; },
    };

    quote! {
        const _: () = {
            #use_serde
            // 编译期检查: 确保用户使用了 serde 而不是 serde_core
            _serde::__require_serde_not_serde_core!();
            #code
        };
    }
}
```

为什么需要 `const _: ()`?

1. **dead_code 抑制**: pub fn 不会被标记为 unused
2. **extern crate 的作用域限制**: derive 生成代码可能引用 serde 的私有模块
3. **编译期版本检查**: `__require_serde_not_serde_core!()` 确保用户用的是 serde(门面) 而不是 serde_core

## pretend.rs —— Remote Derive 的 Dead Code 抑制

```rust
// 源码: serde_derive/src/pretend.rs
// 当使用 #[serde(remote = "OtherType")] 时,
// 本地定义的字段实际上不会被直接访问(因为访问的是远程类型的字段),
// 编译器会警告 dead_code

// 解决方案: 生成"假装使用"的 match 语句
pub fn pretend_used(cont: &Container) -> TokenStream {
    match cont.data {
        Data::Struct(_, ref fields) => {
            let patterns = fields.iter().map(|f| {
                let member = &f.member;
                let ty = &f.ty;
                quote! {
                    let _: #ty;  // "假装"使用类型
                }
            });
            quote! { #(#patterns)* }
        }
        Data::Enum(ref variants) => {
            // 对每个 variant 的构造模式也生成"假装使用"
        }
    }
}
```

## deprecated.rs —— 废弃属性传播

```rust
// 源码: serde_derive/src/deprecated.rs
// 如果 struct/enum 或其 variant 上有 #[deprecated] 属性,
// 生成的 impl 也应该有 #[allow(deprecated)]

pub fn is_no_deprecated(input: &syn::DeriveInput) -> bool {
    // 检查是否有 #[allow(deprecated)] 或 #[deprecated]
    // 如果没有 #[deprecated],返回 true(不需要 allow)
}
```

---

**练习**:
1. 为自定义 struct `S<T, U, V>` 添加字段 `a: T`, `b: U`, `#[serde(skip)] c: V`,用 `cargo expand` 观察生成的 bounds
2. 阅读 `fragment.rs`,重构: 如何统一 Stmts/Expr/Match 的转换?
3. 理解 `pretend_used` 的 panic 信息: 为什么 packed struct 需要特殊处理?
