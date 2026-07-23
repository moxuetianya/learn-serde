# 第一章: 序言 —— 为什么要学 Serde

## 学习目标

完成本教程后，你将能够:
- 透彻理解 serde 的数据模型与核心 trait 体系
- 为任意 Rust 类型手工实现 `Serialize` / `Deserialize`
- 编写自己的 `Serializer` / `Deserializer`(数据格式实现)
- 深度定制 derive 宏的行为(自定义 serialize_with、deserialize_with 等)
- 理解 proc-macro 如何生成序列化代码
- 阅读 serde 源码无障碍

## Serde 是什么?

Serde 是一个**框架**,不是一种数据格式。它定义了:

1. **数据模型**(serde data model)——一套与 Rust 类型系统对应的抽象类型体系
2. **序列化接口**(`Serialize` + `Serializer`)——将 Rust 类型映射到数据模型
3. **反序列化接口**(`Deserialize` + `Deserializer` + `Visitor`)——从数据模型重建 Rust 类型

数据格式(JSON、Postcard、Bincode 等)只需要实现 `Serializer` / `Deserializer` 即可接入。

```
Rust Type ──[Serialize]──> Data Model ──[Serializer]──> Bytes/String
                            ▲
Rust Type <─[Deserialize]───┘
```

## 为什么要深入学 Serde?

1. **Rust 生态基石**——几乎所有 Rust 项目都依赖 serde
2. **理解 trait 设计范式**——Visitor 模式、状态机模式、零开销抽象
3. **自定义需求**——当默认 derive 不够用时(特殊格式、性能优化、零拷贝)
4. **编写数据格式**——为自己的协议实现 Serializer/Deserializer
5. **阅读源码能力**——serde 是 Rust proc-macro 和 trait 设计的最佳教材

## 本教程的项目结构

项目 `/home/peter/project/learn-serde` 包含:

```
serde_core/          # 核心 trait 定义: Serialize, Serializer, Deserialize, Deserializer
serde/               # 门面 crate, 重导出 serde_core + 私有辅助模块
serde_derive/        # proc-macro derive 实现: #[derive(Serialize, Deserialize)]
serde_derive_internals/  # proc-macro 内部 AST 分析(属性解析、验证)
test_suite/          # 完整测试套件
```

学习顺序建议:

| 章节 | 内容 | 涉及源码 |
|------|------|----------|
| 1-2 | 数据模型与核心概念 | serde_core/src/ser/mod.rs, serde_core/src/de/mod.rs |
| 3-4 | Serialize & Serializer | serde_core/src/ser/{mod,impls,fmt,impossible}.rs |
| 5-7 | Deserialize, Deserializer & Visitor | serde_core/src/de/{mod,impls,value,ignored_any}.rs |
| 8-10 | derive 宏实现 | serde_derive/src/{lib,ser,de,bound}.rs |
| 11-13 | derive 内部细节 | serde_derive/src/internals/{ast,attr,check,ctxt}.rs |
| 14-16 | 实战: 自定义 Serializer | 综合练习 |
| 17-19 | 实战: 自定义 Deserializer | 综合练习 |
| 20 | 总结与进阶方向 | 综述 |

现在,让我们从数据模型开始。
