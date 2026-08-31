#!/bin/sh
# 重新生成 05_expand.rs —— 05_custom_deserializer.rs 的宏展开源码快照
#
# 用法: sh lessons/demos/run.sh   (需要 nightly + cargo-expand 插件)
#
# 注意:
#   - 生成物是「阅读参考文档」, 不是常规示例 (含 nightly 内部 feature,
#     stable 下编译会失败, 见 lessons/08-derive宏.md「看真实的展开代码」一节)
#   - 已预置 4 个 feature 门, 可用 nightly 直接运行验证:
#       cargo +nightly run --example 05_expand
#   - 运行原 demo 请用: cargo run --example 05_custom_deserializer

out=lessons/demos/examples/05_expand.rs
cargo expand -p learn-serde-demos --example 05_custom_deserializer > "$out.tmp" &&
    sed '1i #![feature(structural_match, core_intrinsics, print_internals, fmt_helpers_for_derive)]' "$out.tmp" > "$out" &&
    rm "$out.tmp"
