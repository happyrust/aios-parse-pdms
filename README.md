# aios-parse-pdms

PDMS (Plant Design Management System) 数据库解析器 - Rust 实现

## 项目概述

高性能 PDMS 数据库文件解析库,用于解析和处理工厂设计管理系统的二进制数据格式。

## 技术栈

- **语言**: Rust (Edition 2024, Nightly)
- **解析器**: [nom](https://github.com/rust-bakery/nom) 8.0 (解析组合子)
- **并发**: [rayon](https://github.com/rayon-rs/rayon) (数据并行)
- **异步**: [tokio](https://tokio.rs/) (异步运行时)
- **数学**: [glam](https://github.com/bitshifter/glam-rs) 0.30.8 (向量运算)

## 核心功能

- ✅ **元素解析** - PDMS 元素的完整解析
- ✅ **属性解析** - 隐式/显式属性的提取
- ✅ **表达式解析** - 数学表达式和轴向数据
- ✅ **0x07 分段合并** - 多段数据块的正确合并
- ✅ **Members 块解析** - 子元素引用列表
- ✅ **高性能并行处理** - 使用 rayon 并行解析

## 项目结构

```
src/
├── main.rs              # 主入口
├── lib.rs               # 库入口
├── parse.rs             # 核心解析逻辑 (旧实现)
├── parse_explict_tools.rs  # 显式属性工具
├── parser/              # 新模块化架构 ✨
│   ├── combinator.rs    # 解析组合子
│   ├── attribute/       # 属性解析模块
│   │   ├── implicit.rs  # 隐式属性解析 (NEW)
│   │   ├── expression.rs # 表达式解析
│   │   ├── explicit.rs  # 显式属性解析
│   │   └── axis.rs      # 轴向数据解析
│   ├── element/         # 元素解析模块
│   │   └── children.rs  # Members 块解析
│   ├── database/        # 数据库解析模块
│   └── numeric.rs       # 数值解析工具
├── consts.rs            # 常量定义
├── error_types.rs       # 错误类型
└── test_cases/          # 测试用例 (15个文件)

llmdoc/                  # 项目文档
├── guides/              # 使用指南
│   └── parser-module-integration.md  # 模块集成指南
└── agent/               # 调研报告
    ├── pdms-0x07-segment-offset-investigation.md
    ├── pdms_implicit_attribute_investigation.md
    └── pdms_expression_dual_track_investigation.md

examples/
└── new_parser_usage.rs  # 新模块使用示例
```

## 快速开始

### 安装

```bash
git clone <repository-url>
cd aios-parse-pdms
cargo build --release
```

### 使用示例

```rust
use parse_pdms_db::parser::attribute::implicit::{
    parse_implicit_attr_value, ImplicitAttrOffset,
};
use aios_core::pdms_types::DbAttributeType;

// 创建属性偏移信息
let attr_offset = ImplicitAttrOffset {
    name: "EXAMPLE".to_string(),
    offset: 0,
    attr_type: DbAttributeType::INTEGER,
};

// 解析数据
let data = vec![0x00, 0x00, 0x00, 0x2A]; // 42
let (_, value) = parse_implicit_attr_value(&data, &attr_offset, false, 0, 1)?;

match value {
    NamedAttrValue::IntegerType(v) => println!("整数: {}", v),
    _ => {}
}
```

更多示例见 [examples/new_parser_usage.rs](examples/new_parser_usage.rs)

## 运行示例

```bash
# 运行新模块使用示例
cargo run --example new_parser_usage
```

## 测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test parser::attribute::implicit

# 运行所有 parser 模块测试
cargo test parser::
```

**测试覆盖**:
- 70+ parser 模块测试
- 185+ 总测试用例

## 架构演进

### 第一阶段 (已完成 ✅)

- [x] 删除 v2 死代码 (~221 行)
- [x] 修复 0x07 段偏移错误 (20→12 字节)
- [x] 统一分段合并逻辑

### 第二阶段 (已完成 ✅)

- [x] 创建 `parser::attribute::implicit` 模块
- [x] 实现 9 种类型解析器
- [x] 添加 f32/f64 混合模式支持
- [x] 修复所有编译错误
- [x] 通过所有测试

### 第三阶段 (已完成 ✅)

- [x] 分析新旧实现差异
- [x] 编写集成指南文档
- [x] 创建完整使用示例
- [x] 验证新模块功能

### 第四阶段 (未来)

- [ ] 添加 feature flag 切换
- [ ] 集成测试验证
- [ ] 性能基准测试
- [ ] 完全切换到新实现

详见 [llmdoc/guides/parser-module-integration.md](llmdoc/guides/parser-module-integration.md)

## 关键修复

### 1. 0x07 分段偏移错误

**问题**: 旧代码使用 20 字节偏移,导致丢失 8 字节数据
**修复**: 新实现使用正确的 12 字节偏移 (flag + len + self_ref)

详见 [llmdoc/agent/pdms-0x07-segment-offset-investigation.md](llmdoc/agent/pdms-0x07-segment-offset-investigation.md)

### 2. 表达式解析重复实现

**问题**: 存在 v1/v2/v3 三个版本,代码重复
**修复**: 统一使用 v1 生产版本,删除 v2 死代码

详见 [llmdoc/agent/pdms_expression_dual_track_investigation.md](llmdoc/agent/pdms_expression_dual_track_investigation.md)

### 3. 隐式属性解析耦合

**问题**: 表达式处理和类型解析混合在一起
**修复**: 模块化设计,职责分离

详见 [llmdoc/agent/pdms_implicit_attribute_investigation.md](llmdoc/agent/pdms_implicit_attribute_investigation.md)

## 性能特性

- **零拷贝解析**: 使用 nom 的 zero-copy 特性
- **并行处理**: rayon 数据并行加速
- **内存优化**: 避免不必要的克隆和分配
- **安全偏移**: 使用 `saturating_sub` 避免溢出

## 配置

项目使用 TOML 配置文件:

- `DbOption.toml` - 默认数据库选项
- `DbOption_E3dSample.toml` - E3D 示例配置

## 依赖版本

主要依赖:

```toml
nom = "8.0"
glam = { version = "0.30.8", features = ["serde"] }
tokio = { version = "1.37.0", features = ["full"] }
rayon = "1.10.0"
aios_core = { path = "../rs-core" }
```

## 贡献

本项目处于活跃开发中。如需贡献:

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交改动 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## Git 提交历史

关键提交:

- `10a8818` - feat: 完成隐式属性解析模块 (parser::attribute::implicit)
- `c012b53` - docs: 添加 parser 模块集成指南和使用示例
- (更早提交) - refactor: 删除 v2 死代码并修复 0x07 偏移

## 许可证

[项目许可证信息]

## 致谢

- PDMS 格式研究和逆向工程
- Rust 社区的优秀工具链
- nom 解析器组合子库

---

**项目状态**: 🟢 活跃开发中

**最后更新**: 2025-11-30

🤖 部分文档由 [Claude Code](https://claude.com/claude-code) 生成
