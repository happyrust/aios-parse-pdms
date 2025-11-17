# 项目概览：aios-parse-pdms

## 项目目的
PDMS数据库解析器，用于解析PDMS（Plant Design Management System）数据库文件。这是一个用Rust编写的高性能解析库。

## 技术栈
- **语言**: Rust (edition 2024, nightly channel)
- **主要依赖**:
  - `nom`: 解析组合子库，用于语法解析
  - `serde`: 序列化/反序列化
  - `tokio`: 异步运行时
  - `aios_core`: 核心数据库操作库 (GitHub)
  - `rayon`: 数据并行处理
  - `dashmap`: 并发哈希表

## 代码结构
```
src/
├── main.rs           # 主入口点
├── lib.rs            # 库入口，导出公共API
├── parse.rs          # 核心解析模块 (136KB)
├── parse_explict_tools.rs  # 显式工具解析
├── consts.rs         # 常量定义
├── error_types.rs    # 错误类型定义
└── test_cases/       # 测试用例目录
    ├── test_expression.rs      # 表达式解析测试
    ├── test_parse_element.rs   # 元素解析测试  
    ├── test_data.rs           # 数据解析测试
    └── ... (15个测试文件)
```

## 项目特性
- **compression**: 压缩支持 (默认启用)
- **web**: Web编译支持
- **debug_parse**: 调试解析模式

## 配置文件
- `DbOption.toml`: 数据库选项配置
- `DbOption_E3dSample.toml`: E3D示例配置
- `rust-toolchain.toml`: 指定nightly工具链

## 核心API
- `parse_pdms_dir()`: 解析PDMS目录
- `parse_db()`: 解析数据库
- `parse_file()`: 解析单个文件