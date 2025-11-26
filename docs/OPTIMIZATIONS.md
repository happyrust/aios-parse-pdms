# 解析功能优化计划与进展

## 背景目标
- 让 PDMS 解析在可配置性、分块解析正确性与健壮性上可落地优化，并为后续性能与可观测性改进打基础。

## 已完成（本次提交）
- **配置贯穿**：新增 `parse_raw_ele_data_with_info` / `parse_ele_data_with_info`，`parse_db` 直接使用外部 `PdmsDatabaseInfo`，避免默认配置覆盖。
- **分块 children 补齐**：`parse_db_with_chunk` 通过新函数 `parse_db_with_chunk_with_info` 携带配置，并将 `DbBasicData` 的 children 映射填回 `PdmsDbData`，不再返回空层级。
- **I/O 健壮性**：`parse_file` 读取数据库文件时增加错误传播上下文，避免静默失败。
- **API 补充**：对分块解析和元素解析提供带配置的接口（`*_with_info` 变体），保持原有接口兼容默认配置。

## 待办与优先级
- **Critical**：进一步清理警告与未使用变量，聚焦解析路径的必要输出；为 UDA 查询加缓存/批量策略降低 DB 压力。
- **High**：分块解析可按 `chunk_refnos` 过滤 children_map 体积；为 `parse_pdms_dir`/`parse_db` 增加并行与流式读取选项。
- **Medium**：完善 pfno 版本号解析（见 `todo.md`）；梳理日志/调试输出，统一到 `log` 宏；补充异常输入与大文件基准测试。
- **Low**：整理 `#[feature]` 清单与 `cfg`，去除已稳定或未使用的特性标记。

## 使用提示
- 已有调用保持不变，若需要自定义 `PdmsDatabaseInfo`：
  - 单文件：`parse_db(input, &db_info, file_name, project)`（已用外部配置）。
  - 分块：`parse_db_with_chunk_with_info(db_basic_data, &db_info, file_name, project, chunk_refnos, ses_range_map, ignore_world_refno)`.
  - 元素：`parse_ele_data_with_info(bytes, &db_info)` / `parse_raw_ele_data_with_info(bytes, &db_info)`.

## 验证
- 本地执行 `cargo check`（大量现存警告，但编译通过）。
