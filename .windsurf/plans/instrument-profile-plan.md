# 数据库保存性能分析优化计划

本计划旨在为当前的解析保存流程添加 instrument 性能分析，用于识别性能瓶颈。

## 当前状况分析

从终端输出可以看到，当前解析保存流程包含以下阶段：
1. 文件读取阶段
2. 生成参考号类型位置表 (`gen_ref_type_pos_table`)
3. 解析子成员阶段 (`Parsing children members`)
4. 数据库保存阶段（包含多个批次）

## 实施计划

### 阶段1：添加 tracing 依赖和基础设施
1. 在 `Cargo.toml` 中添加 tracing 相关依赖
2. 创建性能分析模块 `src/perf.rs`
3. 设置 tracing subscriber 和 instrument 宏

### 阶段2：为关键函数添加 instrument
1. 为 `parse_db` 函数添加 instrument
2. 为 `gen_ref_type_pos_table` 函数添加 instrument
3. 为 `parse_ele_membs` 和相关解析函数添加 instrument
4. 为 `insert_into_table_with_chunks` 函数添加 instrument

### 阶段3：创建性能报告系统
1. 创建性能数据收集结构
2. 实现性能统计和报告功能
3. 添加性能瓶颈识别逻辑

### 阶段4：集成到现有解析流程
1. 修改 `parse_pdms_dir` 函数集成性能分析
2. 添加性能报告输出
3. 创建性能分析测试用例

## 技术细节

### tracing 配置
- 使用 `tracing` 和 `tracing-subscriber` 库
- 配置 `tracing-chrome` 用于生成性能分析文件
- 使用 `#[instrument]` 宏自动跟踪函数执行时间

### 性能指标
- 函数执行时间
- 数据库操作耗时
- 内存使用情况
- 并发处理效率

### 输出格式
- 控制台实时输出
- JSON 格式详细报告
- Chrome tracing 格式文件

## 预期收益

1. **识别瓶颈**：精确定位耗时最长的操作
2. **优化指导**：为性能优化提供数据支持
3. **监控能力**：建立长期性能监控体系
4. **调试辅助**：在开发过程中快速定位性能问题

## 实施优先级

1. **高优先级**：数据库保存操作分析
2. **中优先级**：解析流程分析
3. **低优先级**：文件读取操作分析

## 风险评估

- **低风险**：添加 instrument 不会影响现有功能
- **性能影响**：tracing 可能带来轻微性能开销（<5%）
- **兼容性**：需要确保与现有日志系统兼容
