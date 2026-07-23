# 第十二章: proc-macro 内部细节(二) —— AST 构建与 Self 替换

**源码参考**: `serde_derive_internals/src/{ast.rs, receiver.rs, respan.rs}`

## Container —— 核心 AST

```rust
// 源码: serde_derive_internals/src/ast.rs
pub struct Container<'a> {
    pub ident: syn::Ident,
    pub attrs: attr::Container,     // 解析后的属性
    pub data: Data<'a>,             // 枚举或结构体的字段数据
    pub generics: &'a syn::Generics,
    pub original: &'a syn::DeriveInput, // 原始 AST
}

pub enum Data<'a> {
    Enum(Vec<Variant<'a>>),
    Struct(Style, Vec<Field<'a>>),
}

pub struct Variant<'a> {
    pub ident: syn::Ident,
    pub attrs: attr::Variant,
    pub style: Style,
    pub fields: Vec<Field<'a>>,
    pub original: &'a syn::Variant,
}

pub enum Style {
    Struct,   // 命名字段: struct S { x: i32 }
    Tuple,    // 位置元组: struct S(i32, i32)
    Newtype,  // 单字段 newtype: struct S(i32)
    Unit,     // 无字段: struct S;
}

pub struct Field<'a> {
    pub member: syn::Member,
    pub attrs: attr::Field,
    pub ty: &'a syn::Type,
    pub original: &'a syn::Field,
}
```

## Container::from_ast() —— 主入口

```rust
// 源码: serde_derive_internals/src/ast.rs ~48
impl<'a> Container<'a> {
    pub fn from_ast(
        cx: &Ctxt,
        item: &'a syn::DeriveInput,
        derive: Derive,
        _private: Option<&'static str>,  // 版本号标识符
    ) -> Option<Self> {
        // Step 1: 解析属性
        let attrs = attr::Container::from_ast(cx, item);

        // Step 2: 转换数据
        let data = match &item.data {
            syn::Data::Enum(enum_data) => {
                Data::Enum(enum_from_ast(cx, &enum_data.variants, &attrs))
            }
            syn::Data::Struct(struct_data) => {
                let (style, fields) = struct_from_ast(cx, &struct_data.fields);
                Data::Struct(style, fields)
            }
            syn::Data::Union(_) => {
                cx.error_spanned_by(item, "Serde does not support derive for unions");
                return None;
            }
        };

        let mut cont = Container { ident: item.ident.clone(), attrs, data, ... };

        // Step 3: 应用 rename rules
        cont.apply_rename_rules();

        // Step 4: 检查 packed
        cont.check_packed();

        Some(cont)
    }
}
```

### Style 判定

```rust
// 源码: serde_derive_internals/src/ast.rs ~110
fn struct_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a syn::Fields,
) -> (Style, Vec<Field<'a>>) {
    match fields {
        syn::Fields::Named(fields_named) => {
            // 命名字段 → Style::Struct
            (Style::Struct, fields_from_ast(cx, &fields_named.named))
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            // 无名字段: 1个→Newtype, 多个→Tuple
            let style = if fields_unnamed.unnamed.len() == 1 {
                Style::Newtype
            } else {
                Style::Tuple
            };
            (style, fields_from_ast(cx, &fields_unnamed.unnamed))
        }
        syn::Fields::Unit => {
            (Style::Unit, Vec::new())
        }
    }
}

fn fields_from_ast<'a>(
    cx: &Ctxt,
    syn_fields: impl IntoIterator<Item = &'a syn::Field>,
) -> Vec<Field<'a>> {
    syn_fields.into_iter().enumerate().map(|(i, f)| {
        Field {
            // 无名字段用 Unnamed(index), 命名字段用 Named(ident)
            member: match &f.ident {
                Some(ident) => syn::Member::Named(ident.clone()),
                None => syn::Member::Unnamed(syn::Index::from(i)),
            },
            attrs: attr::Field::from_ast(cx, i as u32, f, None),
            ty: &f.ty,
            original: f,
        }
    }).collect()
}
```

## replace_receiver —— Self → 具体类型

```rust
// 源码: serde_derive_internals/src/receiver.rs
// 为什么需要替换 Self?
// proc-macro 中, Self 指向 proc-macro 自己的 crate,
// 而不是用户代码中的类型!
// 所以必须把所有 Self 替换为具体的类型名 + 泛型参数。

pub fn replace_receiver(input: &mut syn::DeriveInput) {
    let ident = &input.ident;
    let ty_generics = input.generics.split_for_impl().1;
    let self_ty = parse_quote!(#ident #ty_generics);

    // 遍历整个 AST 替换 Self
    replace_in_type(&mut input.generics, &self_ty);

    match &mut input.data {
        syn::Data::Struct(data) => {
            for field in &mut data.fields {
                replace_in_type(&mut field.ty, &self_ty);
            }
        }
        syn::Data::Enum(data) => {
            for variant in &mut data.variants {
                for field in &mut variant.fields {
                    replace_in_type(&mut field.ty, &self_ty);
                }
            }
        }
        syn::Data::Union(_) => {}
    }

    // 也要替换 where 子句中
    if let Some(where_clause) = &mut input.generics.where_clause {
        for predicate in &mut where_clause.predicates {
            // 替换 trait bounds 中的 Self
        }
    }
}

fn replace_in_type(ty: &mut syn::Type, self_ty: &syn::Type) {
    match ty {
        syn::Type::Path(type_path) => {
            if type_path.path.is_ident("Self") {
                *ty = self_ty.clone();
                // 使用 respan 保留原始 span 信息
                respan(ty, self_ty);
            }
        }
        syn::Type::Reference(ref_type) => {
            replace_in_type(&mut ref_type.elem, self_ty);
        }
        syn::Type::Tuple(tuple) => {
            for elem in &mut tuple.elems {
                replace_in_type(elem, self_ty);
            }
        }
        // ... 递归处理所有 Type 变体
    }
}
```

### respan —— Span 重设

```rust
// 源码: serde_derive_internals/src/respan.rs
// 将 TokenStream 中所有 token 的 span 改为目标 span
pub fn respan(stream: &mut dyn ToTokens, span: Span) {
    // 递归遍历 TokenStream,修改每个 TokenTree 的 span
    // 用于保持错误信息指向正确的位置
}
```

## check.rs —— 交叉验证规则

```rust
// 源码: serde_derive_internals/src/check.rs

pub fn check(cx: &Ctxt, cont: &mut Container, derive: Derive) {
    // 1. default on tuple: 如果 tuple 中某个字段有 default,
    //    则后续字段必须有 default
    check_default_on_tuple(cx, cont);

    // 2. remote 类型的泛型参数必须全有或全无
    check_remote_generic(cx, cont);

    // 3. getter 只能在 remote struct 上使用
    check_getter(cx, cont);

    // 4. flatten 不能用于 tuple/newtype 字段
    check_flatten(cx, cont);

    // 5. field_identifier / variant_identifier 检查
    check_identifier(cx, cont);

    // 6. variant 的 skip 和 serialize_with/deserialize_with 冲突
    check_variant_skip_attrs(cx, cont);

    // 7. internal tag 字段名不能与变体字段名冲突
    check_internal_tag_field_name_conflict(cx, cont);

    // 8. adjacent tag 和 content 不能同名
    check_adjacent_tag_conflict(cx, cont);

    // 9. transparent:
    //    - 不能用于枚举
    //    - 不能用于 unit struct
    //    - 必须有恰好一个非 skipped 字段
    //    - 标记该字段为 transparent
    check_transparent(cx, cont, derive);

    // 10. from 和 try_from 互斥
    check_from_and_try_from(cx, cont);
}
```

### check_transparent 详解

```rust
// 源码: serde_derive_internals/src/check.rs ~400
fn check_transparent(cx: &Ctxt, cont: &mut Container, derive: Derive) {
    if !cont.attrs.transparent() {
        return;
    }

    // 枚举不能 transparent
    if let Data::Enum(ref variants) = cont.data {
        cx.error_spanned_by(cont.original, "transparent not allowed on enum");
        return;
    }

    // Unit struct 不能 transparent
    if let Data::Struct(Style::Unit, _) = cont.data {
        cx.error_spanned_by(cont.original, "transparent not allowed on unit struct");
        return;
    }

    // 计数非跳过字段
    let fields = match cont.data {
        Data::Struct(_, ref fields) => fields,
        Data::Enum(_) => unreachable!(),
    };

    let non_skipped: Vec<_> = fields.iter()
        .filter(|f| !f.attrs.skip_serializing() && !f.attrs.skip_deserializing())
        .collect();

    match non_skipped.len() {
        0 => {
            cx.error_spanned_by(cont.original,
                "transparent needs at least one non-skipped field");
        }
        1 => {
            // 标记该字段,序列化/反序列化时用 transparent 逻辑
        }
        n => {
            cx.error_spanned_by(cont.original,
                format!("transparent needs exactly one field, got {}", n));
        }
    }
}
```

---

**练习**:
1. 创建一个包含 `Self` 引用的 struct,用 `cargo expand` 观察 `replace_receiver` 的效果
2. 阅读 `check.rs` 中的 `check_default_on_tuple`,理解为什么 tuple 的 default 有这个约束
3. 追踪: `#[serde(transparent)] struct S(String, i32);` 会触发哪些错误?
