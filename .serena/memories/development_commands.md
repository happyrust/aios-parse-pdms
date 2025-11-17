# 开发命令指南

## 基础构建命令
```bash
# 标准构建
cargo build

# 发布构建
cargo build --release

# 运行主程序
cargo run

# 运行测试
cargo test

# 运行特定测试模块
cargo test test_expression
cargo test test_cases::test_parse_element
```

## 特性驱动编译
```bash
# 默认特性（包含压缩）
cargo build

# 仅Web构建
cargo build --features web

# 调试解析模式
cargo build --features debug_parse

# 禁用压缩
cargo build --no-default-features
```

## 代码质量检查
```bash
# 格式化代码
cargo fmt

# Clippy静态分析
cargo clippy

# 安全检查
cargo audit

# 文档生成
cargo doc --open
```

## 测试命令
```bash
# 所有测试
cargo test

# 运行特定测试用例
cargo test test_expression_parser
cargo test test_binary_parser

# 显示测试输出
cargo test -- --nocapture

# 运行基准测试（如果有）
cargo bench
```

## 依赖管理
```bash
# 检查过时依赖
cargo outdated

# 更新依赖
cargo update

# 树形显示依赖
cargo tree
```

## 配置相关
```bash
# 使用不同配置文件
cargo run -- --config DbOption_E3dSample.toml

# 检查配置
cargo run -- --check-config
```

## 调试命令
```bash
# 启用调试信息
RUST_LOG=debug cargo run

# GDB调试
cargo build && rust-gdb target/debug/parse_pdms_db

# 内存检查
cargo run +nightly +Z sanitizer=address
```