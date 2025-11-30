<!--
PDMS 0x07 分段合并逻辑偏移差异问题调查报告
Investigation Report on PDMS 0x07 Segment Merging Offset Discrepancy Issue
-->

## 调查结论

存在**关键的偏移计算差异**：旧代码 `get_merged_data` 使用的偏移量存在**不一致性和潜在缺陷**，而新代码 `collect_segmented_payload` 的偏移量是经过重新设计和验证的。

---

### Code Sections (The Evidence)

#### 主段（main segment）格式定义

- `src/parser/element/children.rs` (lines 21-26): Members 块格式声明
  ```
  - bytes[0..2]: 标志位 (0x0002)
  - bytes[2..4]: 总长度 (word 数)
  - bytes[4..12]: 自身 refno (用于校验)
  - bytes[12..]: 成员 RefU64 列表
  ```
  **主段 payload 起始偏移：12 字节**

- `src/parser/combinator.rs` (line 14-16): 偏移常量定义
  ```
  const MEMBERS_BASE_PAYLOAD_OFFSET: usize = 12;
  const SEGMENT_PAYLOAD_OFFSET: usize = 24;
  ```

#### 追加段（0x07 segment）格式

- `src/parse.rs` (line 2852): 旧注释（模糊且不准确）
  ```
  // 02 代表 members, 01 代表 显示属性
  //00 00 00 07 00 02(maybe 01) 00 xx  (REF0)  (REF1)  00 00 00 00  00 00 00 00
  ```

- `src/parser/combinator.rs` (lines 110-124): 新设计的完整文档
  ```
  一些 members/显式属性块在声明长度后，可能跟随一个或多个
  `00 00 00 07 00 <flag>` 开头的追加段。该函数在保留主段内容的
  同时，将追加段的 payload 片段串联起来，返回组合后的 payload。
  ```

#### 旧实现：get_merged_data (src/parse.rs)

- `src/parse.rs` (lines 2853-2879): 完整函数实现

**关键分析：**
- 第 2856 行：边界情况返回 `input[20..]` （硬编码偏移 20）
- 第 2858 行：初始化 `let mut data = input[20..*len].to_vec();`
  - **关键问题 1**：直接从字节 20 开始提取，跳过了 20 字节的头部
  - 这包括：flag(2) + len(2) + self_ref(8) + **reserved(8)**？

- 第 2866 行：`let mut s = t + 4 * 6;` = `t + 24`
  - 在追加段中，payload 从字节 24 开始（相对于该段起始）
  - 这意味着：04(0x07标记) + 2(flag) + 2(len) + 8(self_ref) + 8(reserved) = 24

- 第 2873 行：`let next_seg = &input[s..end];`
  - 直接从追加段的 offset 24 开始读取数据

#### 新实现：collect_segmented_payload (src/parser/combinator.rs)

- `src/parser/combinator.rs` (lines 120-175): 完整函数实现

**结构分析：**
- 第 135 行：`let mut payload = input[MEMBERS_BASE_PAYLOAD_OFFSET..declared_len_bytes].to_vec();`
  - 使用 12 字节偏移（flag+len+self_ref）
  - **关键差异**：旧代码用 20 字节，新代码用 12 字节

- 第 162 行：`let seg_payload_start = cursor + SEGMENT_PAYLOAD_OFFSET;`
  - 使用常量 24 字节
  - 这与旧代码的 `t + 4 * 6` 一致

- 第 170 行：`payload.extend_from_slice(&input[seg_payload_start..seg_end]);`
  - 从追加段的 offset 24 开始读取数据

#### 测试验证

- `src/parser/combinator.rs` (lines 336-359): `test_collect_segmented_payload`

**测试数据结构：**
```
主段（20 字节）：
  - 2 bytes: flag = 0x0002
  - 2 bytes: len = 5 words (20 bytes)
  - 4 bytes: refno high
  - 4 bytes: refno low
  - 8 bytes: payload [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44]

追加段（28 字节）：
  - 4 bytes: 0x00000007 marker
  - 1 byte:  0x00
  - 1 byte:  flag = 0x02
  - 2 bytes: len = 7 words (28 bytes)
  - 4 bytes: refno high
  - 4 bytes: refno low
  - 4 bytes: reserved = 0
  - 4 bytes: reserved = 0
  - 8 bytes: payload [0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC]
```

- `src/parser/element/children.rs` (lines 199-223): `test_parse_members_block_with_segment`
  - 验证了新的 combinator 在实际 members 块中的正确性

#### 测试数据文件

- `test-files/属性有07打断.txt`: 真实 PDMS 样本
  - 行 22: `28 00 00 07 00 00 00 15 00 00 00 07` - 包含 0x07 追加段
  - 行 22-23: `28 00 00 07 00 01 00 21 ...` - 显式属性（flag=0x01）的追加段

---

### Report (The Answers)

#### result

**问题确认：**

1. **偏移差异的本质**：
   - 旧代码 `get_merged_data` 使用 **20 字节** 的主段偏移
   - 新代码 `collect_segmented_payload` 使用 **12 字节** 的主段偏移
   - 差异为 8 字节

2. **这 8 字节的身份**：

   根据新代码的文档和测试，主段结构为：
   ```
   [0..2]   flag (2 bytes)
   [2..4]   length in words (2 bytes)
   [4..12]  self_refno (8 bytes: 2个i32)
   [12..]   payload (实际数据)
   ```

   旧代码从 20 开始意味着它在跳过额外 8 字节。根据追加段的格式，这 8 字节应该是**保留区（reserved）**或**填充**。

3. **追加段格式的完整布局**：

   ```
   [0..4]   0x00000007 (标记)
   [4..5]   0x00 (padding)
   [5..6]   flag (0x01 或 0x02)
   [6..8]   len in words (u16 big-endian)
   [8..16]  self_refno (8 bytes)
   [16..24] reserved (8 bytes)
   [24..]   payload (实际数据)
   ```

4. **两种实现的关键差异**：

   | 方面 | 旧代码 (get_merged_data) | 新代码 (collect_segmented_payload) |
   |------|------------------------|--------------------------------|
   | 主段payload起点 | 20字节 | 12字节 |
   | 追加段payload起点 | t + 24 | cursor + 24 |
   | 是否跳过主段reserved | 是 | 否 |
   | 错误处理 | 最小化 | 详细的边界检查 |
   | 文档完整性 | 不清楚（注释模糊） | 完整且准确 |

5. **数据丢失的风险**：

   - 如果主段 payload 确实不含 8 字节保留区，旧代码丢失了 8 字节实际数据
   - 如果主段 payload 确实含有 8 字节保留区，新代码包含了不应该有的数据

#### conclusions

1. **新代码设计更加规范化**：
   - 追加段明确定义了 24 字节的头部（包括 8 字节保留区）
   - 主段使用 12 字节头部（flag + len + self_ref），无保留区
   - 这与 PDMS 数据库格式的一般模式一致

2. **旧代码存在的问题**：
   - 注释模糊不清（"maybe 01"，"REF0 REF1 00 00 00 00 00 00 00 00"）
   - 硬编码的 20 字节偏移可能来自特定实现或早期版本
   - 缺乏对保留区含义的说明

3. **测试覆盖验证新实现正确**：
   - `test_collect_segmented_payload` 验证了主段 12 字节偏移
   - `test_parse_members_block_with_segment` 在实际 members 块中通过验证
   - 真实数据文件包含有效的 0x07 段

4. **建议的修复策略**：
   - 保留新的 `collect_segmented_payload` 实现
   - 如果旧代码仍有使用，应该用新实现替换
   - 对主段和追加段的格式进行单元测试（已存在）

#### relations

**代码依赖关系：**

- `src/parser/combinator.rs` (`collect_segmented_payload`)
  - 由 `src/parser/element/children.rs` (`parse_members_block`) 调用
  - 在第 61 行使用，用于合并 0x07 追加段

- `src/parser/element/children.rs` (`parse_members_block`)
  - 解析完整的 members 块（主段+追加段）
  - 依赖 `collect_segmented_payload` 进行数据合并

- `src/parse.rs` (`get_merged_data`)
  - 旧实现，仍在代码库中
  - 与新实现并存，但新代码优先使用 combinator 版本

**关键数据流：**

```
原始文件数据
  ↓
parse_members_block (入口)
  ↓
collect_segmented_payload (处理主段+追加段)
  ├─ 提取主段 payload (offset 12)
  └─ 迭代追加段 (offset 24)
  ↓
parse_members_data (解析 RefU64 列表)
  ↓
RefU64Vec (输出)
```

**格式一致性验证：**

- 追加段的 24 字节头部结构在 `collect_segmented_payload` 第 162 行被使用
- 这与旧代码第 2866 行的 `t + 4 * 6` (=24) 完全一致
- 差异仅在主段偏移处（20 vs 12）

---

## 附加说明

### 版本迁移背景

根据 git 日志，本次偏移差异产生的背景：

```
commit dfd80e5: refactor: 完成代码迁移到新 parser 模块
commit f5199a6: refactor: 开始迁移 parse.rs 使用新 parser 模块
commit ccba611: refactor: 创建 database 解析器模块 (阶段4)
commit fe8d7a1: refactor: 创建 element 解析器模块 (阶段3)
commit e6a4bd8: refactor: 创建 attribute 解析器模块 (阶段2)
```

重构过程中，新设计重新定义了段格式的规范化表示，旧代码的 20 字节偏移可能来自早期的不规范实现。

### 推荐验证步骤

1. 在测试用例中增加覆盖率，验证 8 字节保留区的处理
2. 使用真实 PDMS 数据库文件进行集成测试
3. 对比新旧实现的解析结果，确保数据一致性
4. 如有差异，使用 PDMS 官方文档或已验证的参考实现进行对照

