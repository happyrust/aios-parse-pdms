# Parser 模块集成指南

## 概述

本文档描述了新 `parser` 模块的架构设计和使用方法,以及如何逐步从旧的单文件实现迁移到新的模块化架构。

## 架构对比

### 旧架构 (parse.rs)

```
parse.rs (2800+ 行)
├─ parse_implicit_attr_value()  // 隐式属性解析 (170行)
│  ├─ 表达式处理 (check_is_expr)
│  ├─ 类型判断 (基于 default_val)
│  └─ 9种数据类型解析
├─ parse_expression_attr()       // 表达式解析
├─ parse_explicit_attrs()        // 显式属性解析
└─ get_merged_data()             // 0x07分段合并 (已废弃)
```

**问题**:
- 单文件过大,难以维护
- 表达式处理和隐式属性解析耦合
- 代码重复 (v1/v2/v3 表达式解析器)
- 偏移计算错误 (20字节 vs 12字节)

### 新架构 (parser/)

```
parser/
├─ combinator.rs                 // 通用解析组合子
│  ├─ collect_segmented_payload() // 修复的0x07分段合并
│  └─ extend_impl_len()
├─ attribute/
│  ├─ implicit.rs                // 隐式属性解析 (370行)
│  │  ├─ ImplicitAttrOffset      // 偏移信息结构
│  │  ├─ parse_implicit_attr_value()
│  │  ├─ parse_integer()
│  │  ├─ parse_double()
│  │  ├─ parse_bool()
│  │  ├─ parse_string()
│  │  ├─ parse_refno()
│  │  ├─ parse_word()
│  │  ├─ parse_vec3()
│  │  ├─ parse_double_array()
│  │  └─ parse_int_array()
│  ├─ expression.rs              // 表达式解析
│  ├─ explicit.rs                // 显式属性解析
│  └─ axis.rs                    // 轴向数据解析
├─ element/
│  ├─ children.rs                // Members 块解析
│  └─ ...
└─ database/
    └─ ...
```

**优势**:
- 模块化清晰,职责分离
- 表达式处理独立模块
- 类型安全 (基于 DbAttributeType)
- 修复了偏移错误
- 易于测试和维护

## 关键差异

### 1. 数据类型

| 方面 | 旧实现 | 新实现 |
|------|--------|--------|
| 输入结构 | `AttrInfo` | `ImplicitAttrOffset` |
| 返回类型 | `AttrVal` | `NamedAttrValue` |
| 类型判断 | `attr_info.default_val` | `attr_info.attr_type` |
| 表达式处理 | 混合在解析中 | 独立模块 |

### 2. 偏移计算

**旧实现 (错误)**:
```rust
let offset = ((attr_info.offset & 0xFFFF) as usize - f32_neg_offset) * 4;
```

**新实现 (正确)**:
```rust
let offset_words = (attr_info.offset & 0xFFFF) as usize;
let offset_bytes = offset_words * 4;
let actual_offset = if is_f32 {
    offset_bytes.saturating_sub(f32_neg_offset * 4)
} else {
    offset_bytes
};
```

**差异**: 旧实现在 word 级别减去偏移,新实现在 byte 级别减去偏移。

### 3. 类型转换

`AttrVal` 和 `NamedAttrValue` 之间的转换:

```rust
// AttrVal -> NamedAttrValue (aios_core 已实现)
impl From<AttrVal> for NamedAttrValue {
    fn from(v: AttrVal) -> Self {
        (&v).into()
    }
}

// 使用示例
let old_val: AttrVal = parse_implicit_attr_value_old(...)?;
let new_val: NamedAttrValue = old_val.into();
```

## 使用新模块

### 1. 导入

```rust
use crate::parser::attribute::implicit::{
    ImplicitAttrOffset,
    parse_implicit_attr_value,
};
use crate::parser::combinator::collect_segmented_payload;
```

### 2. 创建 ImplicitAttrOffset

**从 AttrInfo 转换**:
```rust
let implicit_offset = ImplicitAttrOffset {
    name: attr_info.name.clone(),
    offset: attr_info.offset,
    attr_type: attr_info.att_type.clone(),
};
```

### 3. 调用解析函数

```rust
let (_, value) = parse_implicit_attr_value(
    implicit_data,      // &[u8] - 隐式数据块
    &implicit_offset,   // &ImplicitAttrOffset
    is_f32,            // bool - f32模式标志
    f32_neg_offset,    // usize - f32偏移调整
    step,              // usize - 当前步骤(用于调试)
)?;

// value 的类型是 NamedAttrValue
match value {
    NamedAttrValue::F32Type(v) => println!("Float: {}", v),
    NamedAttrValue::IntegerType(v) => println!("Int: {}", v),
    NamedAttrValue::Vec3Type(v) => println!("Vec3: {:?}", v),
    _ => {}
}
```

### 4. 处理表达式

新架构中,表达式处理是独立的:

```rust
use crate::parser::attribute::implicit::check_is_expr;
use crate::parser::attribute::expression::parse_expression_attr;

if check_is_expr(attr_info.hash) {
    // 使用表达式解析器
    let (_, (expr_type, expr_value)) = parse_expression_attr(data, refno)?;
} else {
    // 使用隐式属性解析器
    let (_, value) = parse_implicit_attr_value(...)?;
}
```

## 迁移策略

### 阶段 1: 准备 (已完成 ✅)

- [x] 创建新模块骨架
- [x] 实现核心解析函数
- [x] 修复编译错误
- [x] 单元测试通过

### 阶段 2: 并行运行 (当前阶段)

**目标**: 保持旧代码运行,新代码作为可选路径

**实现方案**:

1. **添加 feature flag**:
```toml
# Cargo.toml
[features]
default = []
new-parser = []  # 启用新解析器
```

2. **在调用点添加条件编译**:
```rust
#[cfg(feature = "new-parser")]
{
    // 新实现
    let offset = ImplicitAttrOffset { ... };
    let (_, new_val) = parser::attribute::implicit::parse_implicit_attr_value(...)?;
    let old_val: AttrVal = new_val.into();  // 转换回旧类型
}
#[cfg(not(feature = "new-parser"))]
{
    // 旧实现
    let (_, old_val) = parse_implicit_attr_value(...)?;
}
```

### 阶段 3: 验证 (未来)

- [ ] 添加集成测试对比新旧实现
- [ ] 性能基准测试
- [ ] 真实数据验证

### 阶段 4: 切换 (未来)

- [ ] 默认启用新实现
- [ ] 标记旧实现为 deprecated
- [ ] 删除旧代码

## 测试

### 单元测试

```bash
# 测试新模块
cargo test parser::attribute::implicit

# 测试所有 parser 模块
cargo test parser::
```

### 集成测试 (待实现)

```rust
#[test]
fn test_implicit_parsing_compatibility() {
    let test_data = include_bytes!("test-files/属性有07打断.txt");

    // 旧实现
    let old_result = parse_implicit_attr_value_old(...);

    // 新实现
    let new_result = parser::attribute::implicit::parse_implicit_attr_value(...);

    // 比较结果
    assert_eq!(
        AttrVal::from(new_result),
        old_result
    );
}
```

## 性能考虑

### 优化点

1. **偏移计算**: 新实现使用 `saturating_sub` 避免溢出
2. **内存分配**: 新实现避免不必要的克隆
3. **类型匹配**: 使用 `DbAttributeType` 枚举而非运行时类型判断

### 基准测试 (待实现)

```rust
#[bench]
fn bench_old_impl(b: &mut Bencher) {
    b.iter(|| parse_implicit_attr_value_old(...));
}

#[bench]
fn bench_new_impl(b: &mut Bencher) {
    b.iter(|| parser::attribute::implicit::parse_implicit_attr_value(...));
}
```

## 常见问题

### Q: 为什么不直接替换旧代码?

A: 因为:
1. 旧代码混合了表达式处理,需要重新设计调用链
2. 需要验证新实现的正确性
3. 渐进式迁移风险更低

### Q: Vec3 为什么需要额外依赖?

A: `glam` 是标准的数学库,提供高性能的向量运算。`aios_core` 已经使用它,我们只是显式声明依赖。

### Q: 新实现是否完全兼容?

A: 几乎兼容,但有以下差异:
1. 偏移计算更正确 (修复了旧的bug)
2. 返回类型不同 (`NamedAttrValue` vs `AttrVal`)
3. 表达式处理需要单独调用

## 参考文档

- [pdms-0x07-segment-offset-investigation.md](../agent/pdms-0x07-segment-offset-investigation.md) - 偏移差异分析
- [pdms_implicit_attribute_investigation.md](../agent/pdms_implicit_attribute_investigation.md) - 隐式属性格式
- [pdms_expression_dual_track_investigation.md](../agent/pdms_expression_dual_track_investigation.md) - 表达式解析对比
