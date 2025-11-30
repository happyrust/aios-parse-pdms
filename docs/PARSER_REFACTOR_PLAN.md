# PDMS 解析器重构计划

## 一、现状分析

### 1.1 当前代码结构

```text
src/
├── parse.rs                 # 4110 行，主解析逻辑
├── parse_explict_tools.rs   # 930 行，显式属性解析
├── error_types.rs           # 错误类型定义
├── consts.rs                # 常量定义
└── lib.rs                   # 库入口
```

### 1.2 现有问题

1. **巨型文件**：`parse.rs` 超过 4000 行，职责过多
2. **命名不一致**：
   - 部分函数有 `_nom` 后缀：`parse_expression_attr_nom`, `parse_axis_expression_nom`
   - 部分函数无后缀但也使用 nom：`parse_attr_members`, `parse_attr_owner`
   - 存在重复功能的函数：`parse_expression_attr` vs `parse_expression_attr_nom`
3. **混合解析风格**：部分使用原始字节操作，部分使用 nom
4. **缺乏模块化**：所有解析函数集中在一个文件

---

## 二、重构目标

1. **全面采用 nom**：统一使用 nom 组合子进行解析
2. **模块化设计**：按功能拆分为独立模块
3. **统一命名规范**：清晰、一致的函数命名
4. **提高复用性**：提取通用解析组件

---

## 三、新模块结构

```text
src/
├── lib.rs
├── error.rs                      # 错误类型
├── consts.rs                     # 常量定义
│
├── parser/                       # 解析器模块
│   ├── mod.rs                    # 模块入口，导出公共 API
│   │
│   ├── primitives.rs             # 基础类型解析器
│   │   - parse_refno             # RefU64 解析
│   │   - parse_ref_tuple         # RefI32Tuple 解析
│   │   - parse_hash              # 哈希值解析
│   │   - parse_padded_string     # PDMS 字符串解析
│   │
│   ├── numeric.rs                # 数值解析器
│   │   - parse_f32_be            # 大端 f32
│   │   - parse_f64_be            # 大端 f64
│   │   - parse_explicit_num      # 显式数值（带标志位）
│   │   - parse_unit_value        # 带单位的数值
│   │
│   ├── element.rs                # 元素解析器
│   │   - parse_element           # 完整元素解析
│   │   - parse_element_header    # 元素头部
│   │   - parse_element_children  # 子元素列表
│   │   - parse_element_members   # 成员列表
│   │
│   ├── attribute/                # 属性解析器
│   │   ├── mod.rs
│   │   ├── implicit.rs           # 隐式属性
│   │   │   - parse_implicit_attr
│   │   │   - parse_implicit_value
│   │   ├── explicit.rs           # 显式属性
│   │   │   - parse_explicit_attr
│   │   │   - parse_explicit_header
│   │   │   - parse_explicit_value
│   │   └── expression.rs         # 表达式属性
│   │       - parse_expression
│   │       - parse_axis_expr
│   │       - parse_func_expr
│   │       - parse_const_expr
│   │
│   ├── database.rs               # 数据库文件解析器
│   │   - parse_db_header         # 数据库头部
│   │   - parse_db_info           # 数据库信息
│   │   - parse_project_name      # 项目名称
│   │
│   └── combinator.rs             # 自定义组合子
│       - take_padded             # 带填充的 take
│       - many_refno              # 多个 refno
│       - until_marker            # 直到标记
│
├── convert/                      # 转换器模块
│   ├── mod.rs
│   ├── axis.rs                   # 轴向转换
│   ├── string.rs                 # 字符串转换
│   └── numeric.rs                # 数值转换
│
└── api.rs                        # 高层 API（兼容旧接口）
    - parse_pdms_dir
    - parse_file
    - parse_db
    - parse_ele_data
```

---

## 四、命名规范

### 4.1 解析函数命名

| 前缀 | 含义 | 示例 |
|------|------|------|
| `parse_` | 返回 `IResult` 的 nom 解析器 | `parse_refno`, `parse_element` |
| `try_parse_` | 可能失败的解析，返回 `Option` | `try_parse_expression` |
| `read_` | 从文件/流读取 | `read_db_file` |

### 4.2 组合子命名

| 前缀 | 含义 | 示例 |
|------|------|------|
| `take_` | 提取固定长度 | `take_padded_string` |
| `many_` | 解析多个 | `many_refno` |
| `until_` | 直到条件 | `until_marker` |

### 4.3 转换函数命名

| 前缀 | 含义 | 示例 |
|------|------|------|
| `to_` | 类型转换 | `to_axis_string` |
| `from_` | 从某类型构造 | `from_bytes` |

---

## 五、重构步骤

### 阶段 1：基础设施（1-2 天）

1. 创建 `parser/` 模块目录结构
2. 实现 `parser/primitives.rs` - 基础类型解析器
3. 实现 `parser/numeric.rs` - 数值解析器
4. 实现 `parser/combinator.rs` - 自定义组合子
5. 添加单元测试

### 阶段 2：属性解析器（2-3 天）

1. 实现 `parser/attribute/implicit.rs`
2. 实现 `parser/attribute/explicit.rs`
3. 实现 `parser/attribute/expression.rs`
4. 迁移 `parse_explict_tools.rs` 中的功能
5. 添加集成测试

### 阶段 3：元素解析器（1-2 天）

1. 实现 `parser/element.rs`
2. 迁移 `parse.rs` 中的元素解析逻辑
3. 添加测试

### 阶段 4：数据库解析器（1 天）

1. 实现 `parser/database.rs`
2. 迁移数据库文件解析逻辑

### 阶段 5：API 兼容层（1 天）

1. 创建 `api.rs` 导出兼容旧接口的函数
2. 更新 `lib.rs` 导出
3. 确保所有测试通过

### 阶段 6：清理（1 天）

1. 删除旧的 `parse.rs` 和 `parse_explict_tools.rs`
2. 移除带 `_nom` 后缀的重复函数
3. 更新文档

---

## 六、代码示例

### 6.1 primitives.rs

```rust
use nom::{IResult, Parser};
use nom::number::complete::{be_u32, be_i32};
use nom::sequence::tuple;
use nom::combinator::map;

/// 解析 RefU64（两个 u32 组成）
pub fn parse_refno(input: &[u8]) -> IResult<&[u8], RefU64> {
    map(
        tuple((be_u32, be_u32)),
        |(high, low)| RefU64::from_two_nums(high, low)
    ).parse(input)
}

/// 解析 RefI32Tuple
pub fn parse_ref_tuple(input: &[u8]) -> IResult<&[u8], RefI32Tuple> {
    map(
        tuple((be_i32, be_i32)),
        |(a, b)| RefI32Tuple::new(a, b)
    ).parse(input)
}

/// 解析 4 字节哈希值
pub fn parse_hash(input: &[u8]) -> IResult<&[u8], i32> {
    be_i32.parse(input)
}
```

### 6.2 attribute/expression.rs

```rust
use nom::{IResult, Parser};
use nom::branch::alt;
use nom::combinator::map;

/// 表达式类型
pub enum ExpressionKind {
    Axis,
    Function,
    Constant,
    String,
}

/// 解析表达式属性
pub fn parse_expression(input: &[u8], refno: RefU64) -> IResult<&[u8], Expression> {
    let (input, hash) = parse_hash(input)?;
    let expr_type = dehash(hash);
    
    alt((
        map(parse_axis_expr, |v| Expression::Axis(v)),
        map(parse_func_expr, |v| Expression::Function(v)),
        map(parse_const_expr, |v| Expression::Constant(v)),
    )).parse(input)
}

/// 解析轴向表达式
pub fn parse_axis_expr(input: &[u8]) -> IResult<&[u8], AxisExpression> {
    // 检查是否为轴向表达式（16字节固定格式）
    let (input, _) = verify_axis_format(input)?;
    let (input, values) = tuple((be_i32, be_i32, be_i32, be_i32)).parse(input)?;
    Ok((input, decode_axis(values)))
}
```

### 6.3 combinator.rs

```rust
use nom::{IResult, Parser};
use nom::bytes::complete::take;
use nom::multi::count;

/// 解析带填充的字符串
pub fn take_padded_string(len: usize) -> impl Fn(&[u8]) -> IResult<&[u8], String> {
    move |input| {
        let (input, bytes) = take(len).parse(input)?;
        let s = bytes.iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
        Ok((input, s))
    }
}

/// 解析多个 refno
pub fn many_refno(cnt: usize) -> impl Fn(&[u8]) -> IResult<&[u8], Vec<RefU64>> {
    move |input| {
        count(parse_refno, cnt).parse(input)
    }
}
```

---

## 七、测试策略

1. **单元测试**：每个解析函数都有对应的测试
2. **集成测试**：测试完整的元素/数据库解析流程
3. **回归测试**：确保重构后行为与原代码一致
4. **性能测试**：验证重构后性能无明显下降

---

## 八、风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 破坏现有功能 | 保持 API 兼容，逐步迁移 |
| 性能下降 | 基准测试对比，优化热点 |
| 测试覆盖不足 | 迁移前补充测试用例 |

---

## 九、时间估计

| 阶段 | 时间 |
|------|------|
| 阶段 1：基础设施 | 1-2 天 |
| 阶段 2：属性解析器 | 2-3 天 |
| 阶段 3：元素解析器 | 1-2 天 |
| 阶段 4：数据库解析器 | 1 天 |
| 阶段 5：API 兼容层 | 1 天 |
| 阶段 6：清理 | 1 天 |
| **总计** | **7-10 天** |

---

## 十、后续优化

1. **错误处理增强**：使用自定义错误类型提供更好的错误信息
2. **流式解析**：支持大文件流式解析
3. **并行解析**：利用 rayon 并行处理独立元素
4. **缓存优化**：缓存常用解析结果
