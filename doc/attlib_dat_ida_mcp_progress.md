# attlib.dat（IDA-Pro-MCP）解析进展记录

## 2025-12-29 阶段进展 01：确认当前 IDA 工程载入对象

### 结论

- 通过 `ida-pro-mcp` 读取 IDB 元信息确认：当前打开的 IDA 工程为 `core.dll`（模块名 `core.dll`，基址 `0x10000000`，包含 `.text/.rdata/.data` 等典型 PE 段）。
- 因此 **当前 IDB 并非 `attlib.dat` 纯数据文件**，后续若要直接对 `attlib.dat` 做“按字节/按段/按偏移表”的格式推断，需要在 IDA 中把 `attlib.dat` 作为 binary 数据单独载入（新建一个 IDB/或新建一个临时工程）。

### 已做的验证（MCP 输出摘要）

- `mcp1_idb_meta`：module=`core.dll`，base=`0x10000000`
- `mcp1_segments`：存在 `.text` 段（r-x）与 `.rdata/.data` 段
- `mcp1_strings(count=200)`：字符串集中于程序/库文本（如错误信息、模块名），不是 attlib.dat 的段标识符抽样

### 下一步计划（需要你配合的操作）

- 在 IDA 中打开 `attlib.dat`：
  - File -> Open -> 选择 `attlib.dat`
  - 以“Binary file”方式载入（不是 PE/ELF）
  - 常见设置建议：
    - Processor type：一般选 `metapc`（或保持默认，不影响纯数据分析）
    - Loading address：`0x0` 或 `0x10000000` 均可（关键是后续偏移一致）
    - 勾选创建新数据库（新 `.i64`）

- 载入后我将用 MCP 继续：
  - 扫描 `segments`（通常会是一段大 `.data` 或单段）
  - `strings`/`analyze_strings` 找到段 key（如 ATGTDF/ATGTIX/ATGTSX/ATNAIN）在文件中的位置
  - 以这些位置为锚点反推出：文件头（页表/计数/偏移表）、记录布局、字符串池

### 风险与注意

- `attlib.dat` 可能是“大端 + 2048B 页（512*u32）”的页式结构；载入后会用“按 u32 模式”验证（例如相邻值范围、0xFFFFFFFF 分隔符、段指针表）。

## 2025-12-29 阶段进展 02：定位 core.dll 内 attlib.dat 解析管线（入口/分段/读页）

### 入口与总控

- `RFLDFL_Init_SysLibs` @ `0x105d44c0`
  - 明确调用：`RFLDFL_Load_AttlibDat("/%AVEVA_DESIGN_EXE%/attlib.dat", 30, ...)`
- `RFLDFL_Load_AttlibDat` @ `0x10851210`（IDA 名称同函数在 `0x10851888` 处也有标注）
  - 关键字符串：
    - "READONLY, DB, BL "
    - `OLD, READ`
    - "Unable to open the Attribute Data File - "（错误路径）
  - 关键被调函数（已由 MCP 列出 callee）：
    - `PDFINI`（文件初始化/校验入口）
    - `FHFIND`/`FHQERR`（文件查找/错误）

### 分段解析函数（以段名字符串为锚点反推）

- `sub_10852A64` @ `0x10852a64`（xrefs 到段名 `ATGTIX`）
  - 作用：解析/索引相关（推测对应“Attribute Global Index”）
- `sub_10852E20` @ `0x10852e20`（xrefs 到段名 `ATGTDF`）
  - 作用：解析全局定义表（推测对应“Attribute Global Definition/Definitions Table”）
- `sub_108533B4` @ `0x108533b4`（xrefs 到段名 `ATGTSX`）
  - 作用：解析语法/约束段（推测对应“Attribute/Noun Syntax Table”）
- `sub_1084F7C0` @ `0x1084f7c0`（xrefs 到段名 `ATNAIN`）
  - 作用：解析 Noun-Attribute Index（至少还提及 `ATNATX/ATNALO`）
- `sub_10851DA8` @ `0x10851da8`（xrefs 到段名 `ATGTLT`）
  - 作用：段/表的某种“目录/定位/跳转”逻辑（用于驱动后续多段解析）

### 读页/记录缓存相关（用于证明页式结构）

- `sub_1044FC20` @ `0x1044fc20`（字符串："Read attlib Page number "）
  - 作用：按页读取 attlib 内容（疑似 2048B 页读取器/解码器入口）
- `sub_1045037C` @ `0x1045037c`（字符串：`ATTLIB Record Cache`）
  - 作用：输出/查看记录缓存内容（可用于反推出缓存结构字段）

### 下一步验证点

- 在 `RFLDFL_Load_AttlibDat` 内继续定位：
  - 文件头读取（8 * DWORD 等）的位置与字段语义
  - 触发上述分段解析函数（`sub_10852A64/sub_10852E20/sub_108533B4/sub_1084F7C0`）的调用顺序与参数（尤其是“段起始页/偏移/计数”）
- 在 `sub_1044FC20` 内确认：
  - 页大小（2048/4096）、端序（大端/小端）、以及记录读取方式（u32 数组还是字节流）

## 2025-12-29 阶段进展 03：确认关键 call site 与页大小（0x800=2048）

### 在 RFLDFL_Load_AttlibDat 内的关键调用点（来自反汇编）

- **读页函数 `sub_1044FC20` 的调用点**
  - `0x108514d0`：`call sub_1044FC20`
  - `0x108514f4`：`call sub_1044FC20`
  - 说明：该函数接受一个“页号/记录号”指针作为输入，并通过 `FHDBRN` 读取对应页的数据到全局缓冲区（见下节）。

- **ATGTIX 分段解析 `sub_10852A64` 的调用点**
  - `0x108515c0`：`call sub_10852A64`

- **ATGTDF 分段解析 `sub_10852E20` 的调用点**
  - `0x10851650`：`call sub_10852E20`

> 注：`sub_108533B4(ATGTSX)` 也在 `RFLDFL_Load_AttlibDat` 内被调用（`xrefs_to(0x108533b4)` 指向 `0x108516bf` 与 `0x1085182a`），后续可在这两个地址附近查看压栈参数以恢复其入参语义。

### 页大小与缓存策略（来自 sub_1044FC20 反汇编）

- **页大小确认**
  - `sub_1044FC20` 中存在：`imul edx, 800h` / `imul eax, 800h`
  - 这表明“页号 * 0x800”参与计算页缓冲地址，强烈指向 **每页 2048 字节**（0x800）

- **缓存索引数组（可用于反推结构）**
  - `dword_10F5C7A0[...]`：缓存槽位对应的“页号/记录号”
  - `dword_10F5D9A0[...]`：缓存槽位的“最近使用计数/时间戳”（与 `dword_10F5EBA0` 递增相关）
  - `dword_10F5EBA0`：全局访问计数（每次读页递增）
  - `dword_10F5EBA4`：缓存槽位数量上限增长，比较常量 `0x3E8`（1000）
  - `dword_10F5EBA8`：最近命中的槽位索引（fast path）

- **真正的 IO 读取点**
  - `sub_1044FC20` 在构造地址后调用 `FHDBRN(...)` 读取页面：
    - 其读取目标缓冲区来自 `dword_11C2A860 + 0x1F4000` 这一片全局区域
    - 这也解释了 `RFLDFL_Load_AttlibDat` 里从 `dword_11C2A860` 直接按页取 8 个 DWORD 的逻辑（见阶段 02 的“文件头读取 8*DWORD”描述）

## 2025-12-29 阶段进展 04：还原分段解析函数的入参语义与记录布局

本阶段基于 `RFLDFL_Load_AttlibDat` 中的实际 `call` 前压栈赋值，以及对应解析函数的反编译结果，确认三段核心表的“输入页号/输出数组/计数与上限”参数含义，并提炼出每条记录在页内的布局。

### ATGTSX（Syntax 表）- `sub_108533B4`

- **调用点与参数（以 `0x108516bf` 为例）**
  - `a2`：起始页号指针
    - 在 `0x10851665..0x1085166d`：从 `var_28 + 0x0C` 取值到 `var_BC`，并传入 `&var_BC`
    - 反编译中 `for (i = *a2; ; ++i) FHDBRN(..., &i, ...)`：逐页向后扫
  - `a3/a4/a5`：三个输出数组（均为 `u32` 数组）
  - `a6`：最大可写入条目数（上限指针）
  - `a7`：实际写入条目数（计数指针），函数开头会置 0

- **页内记录布局（每条记录 3 个 u32）**
  - 终止规则：遇到 `0` 或 `0xFFFFFFFF`（-1）停止；遇到页尾 `0` 继续读下一页
  - 记录三元组：
    - `out_key[*] = w0`
    - `out_v1[*]  = w1`
    - `out_v2[*]  = w2`
  - 其中 `w0` 来自 `var80C[j-1]`，`w1` 来自紧随其后的 `var80C[j]`，`w2` 来自下一字（由循环增量表达式写入）

### ATGTIX（全局索引）- `sub_10852A64`

- **总体行为**
  - 从 `*a2` 指定的起始页开始，`FHDBRN` 读取 `512*u32`
  - 扫描每页中的“哈希/编码”值 `v15`，要求落在范围 `[531442, 387951929]`，否则报错

- **页内记录布局（每条记录 2 个 u32）**
  - 记录二元组：
    - `hash_or_code = w0`
    - `disp = w1`（随后被拆为 `page = disp / 512` 与 `offset = disp % 512`）
  - 输出：
    - `out_hash[*] = hash_or_code`
    - `out_page[*] = disp / 512`
    - `out_off[*]  = disp % 512`

### ATGTDF（全局定义）- `sub_10852E20`

- **总体行为**
  - 同样按页读取并扫描 `v18`（范围检查同上）
  - 每条记录至少包含 3 个 u32：`id/hash`、`tag`、`kind`
  - `kind` 仅接受 `1` 或 `2`：
    - `1`：该记录不引用扩展数据（输出索引为 0）
    - `2`：该记录引用扩展数据区（会在“扩展数组”里占用一段）

- **页内记录布局（固定头 + 可变尾）**
  - 记录头（3*u32）：
    - `w0 = id_or_hash`
    - `w1 = tag_or_type`
    - `w2 = kind(1|2)`
  - 若 `kind == 2`：
    - 分配 `ext_index = ++*a10`，并将 `out_ext_index[*] = ext_index`
    - 若 `w1 == 4`：下一字 `w3` 表示 `n`，随后紧跟 `n` 个 u32 进入扩展数组
    - 否则：仅再读 1 个 u32 进入扩展数组

### 下一步计划

- 把上述三段的“页内记录布局”映射到你现有的 Python 解析器：
  - 优先实现 ATGTIX/ATGTDF/ATGTSX 的纯数据抽取（不做语义解释），先把 `out_*` 数组 dump 出来
  - 再结合 `ATNAIN`（`sub_1084F7C0`）去建立 noun->attr 的索引关系

## 2025-12-29 阶段进展 05：ATNAIN/ATNATX/ATNALO 的查找与映射生成逻辑

### 关键函数与职责

- `sub_1084F7C0` @ `0x1084f7c0`
  - 一个“多入口”的查询函数：通过不同入口设置 `var_8C` 与段名字符串，分别处理：
    - `ATNAIN`（入口 1）
    - `ATNATX`（入口 2）
    - `ATNALO`（入口 3）
  - 其内部会调用：
    - `sub_1044FC20`（读页）
    - `sub_10450144`（ATFIND：线性匹配）
    - `sub_104501F8`（ATCHOP：二分定位）

- `sub_1084F1A5` @ `0x1084f1a5`
  - 段名 `ATNLOG` 相关的查找/加载逻辑
  - 在特殊条件下会调用 `sub_10851DA8(ATGTLT)` 作为目录/跳转辅助

- `DB_Noun::internalGetField` @ `0x10457aa0`（入口点 `0x10457b0f`）
  - `sub_1084F7C0` 的主要上层调用者：这说明 `ATNAIN/ATNATX/ATNALO` 最终服务于 “DB_Noun 字段/属性获取”

### 依赖的数据表（RFLDFL_Load_AttlibDat 第二轮解析产物）

在 `RFLDFL_Load_AttlibDat` 中会有一轮把 ATGT* 解析结果写入 `unk_11BFA080`（在反汇编里以 `offset unk_11BFA080 + 0x...` 形式出现）。`sub_1084F7C0/sub_1084F1A5` 会直接用这一组表进行查找：

- `unk_11BFA080 + 0x18000`：一个按 key 排序的数组（用于 `ATFIND` 线性匹配）
- `unk_11BFA080 + 0x307D4`：该数组长度（count）
- `unk_11BFA080 + 0x18190`：与 key 对应的“类型/模式”数组（用于校验 `var_60` 是否匹配）
- `unk_11BFA080 + 0x307D0`：另一个用于二分的“chop 表”（用于 `ATCHOP` 二分定位）
- `unk_11BFA080 + 0x8000 / +0x10000`：与 `ATGTIX` 相同语义的 page/off（用于把逻辑记录定位到页内）

### 查找流程（抽象）

以 `sub_1084F7C0` 中一段典型路径为例（可在其汇编 `0x1084f98b..0x1084fb7a` 附近观察）：

1. **ATFIND：在 key 数组中找到匹配项**
   - `sub_10450144(a1_key_ptr, table_keys, table_count_ptr, out_index_ptr)`
   - 若未找到则返回 0（out_index=0）

2. **类型校验**
   - 取 `table_type[index]` 与当前入口要求的 `var_60` 做一致性检查（例如 `ATNAIN/ATNATX/ATNALO` 的模式不同）

3. **ATCHOP：在另一个有序表中二分定位页/偏移信息**
   - `sub_104501F8(key_ptr, chop_table, chop_count_ptr, out_pos)`
   - 其返回 `pos` 用于从 `page/off` 数组提取页面位置

4. **读页 + 取页内 u32 字段**
   - 通过 `sub_1044FC20` 读到目标页
   - 然后用 `page_base + (word_index * 4)` 方式取某些控制字段
   - 若该字段为 `0`/`0xFFFFFFFF`，会走 fallback 分支继续找下一跳（见 `0x1084fac1..0x1084fb7a` 的循环）

### 目前可落地的解析建议

- 若你的目标是“离线解析 attlib.dat 构建 noun->attrs 映射”，更建议优先复刻 `ATGTIX/ATGTDF/ATGTSX` 的抽取逻辑（阶段 04 已给出明确记录布局），再按 `sub_1084F7C0` 的查找链路把 `ATNAIN/ATNATX/ATNALO` 的跳转/多级索引机制复刻出来。
- 其中 `sub_10450144`（线性匹配）与 `sub_104501F8`（二分定位）在伪代码层面已经足够明确，可直接按其语义实现。

### 常量/上限（从只读数据区读到）

- `unk_10AB2354`（u32=8192）：ATGTIX/ATGTSX 等大表的 max entries 上限
- `unk_10AB2358`（u32=200）：ATGTDF 扩展数组 max 上限（第二轮表）
- `unk_10AB235C`（u32=100）：ATGTDF 记录数 max 上限（第二轮表）
- `unk_10AB23FC/unk_10AB2424/unk_10AB2454`（u32=512）：页内 u32 数（512*u32=2048B）

## 2025-12-29 阶段进展 06：落地到 Rust 解析器（src/parser/attlib/mod.rs）

### 已完成的代码落地

- 在 `src/parser/attlib/mod.rs` 中新增并解析三张“原始段表”以便与 core.dll 行为对齐：
  - `atgtix: Vec<AtgtixEntry>`（2*u32 记录：`code + disp(page/off)`）
  - `atgtdf: Vec<AtgtdfEntry>`（3*u32 头 + 可变尾，含 `kind==2` 的 ext_index）
  - `atgtsx: Vec<AtgtsxEntry>`（3*u32 记录：`key/v1/v2`）

- 在 `parse_attlib_file` 中：
  - 仍然读取目录页 `page 1`
  - 额外对目录页里的“候选起始页”做启发式探测（`guess_atgtix_start_page/guess_atgtdf_start_page/guess_atgtsx_start_page`），以避免 dir_page 下标假设不一致时解析失败

### 当前的已知差距（待继续）

- `parse_atnain` 目前仍是“直接扫 page 里 3 元组”的简化版本；而从 core.dll (`sub_1084F7C0`) 来看，`ATNAIN/ATNATX/ATNALO` 更像是借助 `ATFIND/ATCHOP + ATGTIX(page/off)` 的多级索引查找。
- 下一步如果目标是严格复刻 core.dll 行为，应优先把 `sub_10450144(ATFIND)` 与 `sub_104501F8(ATCHOP)` 的语义在 Rust 中实现，并基于 `ATGTIX` 的 page/off 定位读取，逐步替换当前的简化 `parse_atnain`。

## 2025-12-29 阶段进展 07：补充解析规则梳理（段表布局/索引解码）与 Python 解析器定位问题

### Python 解析器文件定位现状

- IDE 当前打开了 `attlib_ida_parser.py` 与 `attlib_smart_parser.py`，但在本仓库工作区（`aios-parse-pdms-fork`）磁盘侧未能定位到对应文件：
  - `find_by_name("attlib_ida_parser.py")` 返回 0 结果
  - `find_by_name("attlib_smart_parser.py")` 返回 0 结果
  - `find_by_name("*attlib*.py")` 返回 0 结果
- 推测原因：
  - 文件位于其他未纳入当前四个工作区的目录
  - 或为未保存缓冲区/临时文件（磁盘路径与 IDE 显示不一致）
  - 或被 `.gitignore` / 工具忽略规则排除
- 后续动作：需要用户提供这两个 Python 文件的真实绝对路径，或将关键解析函数片段粘贴出来，才能继续逐行比对其实现与 core.dll 行为。

### 当前已确认的 attlib.dat 基础规则（可作为 Python/Rust 对齐基准）

- 页式存储
  - 每页大小：`0x800 = 2048` 字节
  - 每页按 `u32` 解码：`2048 / 4 = 512` 个 `u32`
  - 端序：大端（`be_u32`）

- 记录边界
  - 记录分隔符：`0xFFFFFFFF`
  - 属性记录区的“记录收集”需要以分隔符切分（通常会先遇到一个分隔符作为同步点，然后开始收集记录内容，遇到下一个分隔符提交上一条记录）。

### 三张段表的页内记录布局（与 core.dll 反编译对齐）

- ATGTIX（全局索引，`sub_10852A64`）
  - 每条记录 2 个 `u32`：`(code_or_hash, disp)`
  - 其中 `disp` 需要拆解：
    - `page = disp / 512`
    - `offset = disp % 512`
  - 终止条件：遇到 `0` 或 `0xFFFFFFFF`，或遇到不在合理范围的 `code/hash`

- ATGTSX（语法表，`sub_108533B4`）
  - 每条记录 3 个 `u32`：`(key, v1, v2)`
  - 终止条件：遇到 `0` 或 `0xFFFFFFFF`；若页尾遇到 `0`，继续读下一页

- ATGTDF（全局定义表，`sub_10852E20`）
  - 记录头：3 个 `u32`：`(id_or_hash, tag_or_type, kind)`
  - `kind` 仅接受 `1|2`
    - `kind==1`：无扩展数据
    - `kind==2`：存在扩展数据，写入 ext 数组，记录携带 `ext_index`
  - `kind==2` 的扩展读取：
    - 若 `tag_or_type == 4`：额外读取一个长度 `n`，随后读取 `n` 个 `u32` 作为扩展
    - 否则：额外读取 1 个 `u32` 作为扩展

### ATNAIN/ATNATX/ATNALO 的查找与映射生成（核心是多级索引）

- `sub_1084F7C0` 同时覆盖 `ATNAIN/ATNATX/ATNALO` 三种入口模式，通过段名与模式参数控制查找。
- 查找依赖二轮加载后驻留在内存的大表（`unk_11BFA080 + ...`）：
  - `ATFIND`（`sub_10450144`）：在有序 key 数组中匹配 key（可线性/等价查找）
  - 类型校验：根据入口模式检查 `table_type[index]`
  - `ATCHOP`（`sub_104501F8`）：在另一张 chop 表中二分定位位置
  - 通过与 `ATGTIX` 同语义的 `page/off` 数组定位页与页内 word，再读页获取最终的 mapping 数据

## 2025-12-29 阶段进展 08：反编译 ATFIND/ATCHOP，精确化 ATNAIN/ATNLOG 多级索引规则

本阶段以 IDA 反编译为准，确认 `ATFIND/ATCHOP` 的精确语义（包括 1-based 下标约定与边界条件），并结合 `sub_1084F7C0/sub_1084F1A5` 的调用方式，还原 ATNAIN/ATNLOG 的查找骨架。

### ATFIND - `sub_10450144`（0x10450144）

- 入口签名（反编译）：`int __cdecl sub_10450144(_DWORD *a1, int a2, int *a3, _DWORD *a4)`
- 语义：在 `a2` 指向的表（`u32` 数组）中，从 1 开始线性扫描，找与 `*a1` 相等的条目。
- 关键约定：
  - `*a3` 是表长度（count）
  - `*a4` 是输出位置（1-based index）
  - 找到则 `*a4 >= 1`；找不到则 `*a4 = 0`
  - 扫描比较等价于：`*a1 == table[*a4 - 1]`

### ATCHOP - `sub_104501F8`（0x104501F8）

- 入口签名（反编译）：`int __cdecl sub_104501F8(_DWORD *a1, _DWORD *a2, int *a3, int *a4)`
- 语义：在 `a2` 指向的有序表（`u32` 数组）中执行二分定位，输出精确命中的位置（同样为 1-based）。
- 关键约定与边界条件：
  - `*a3` 是表长度（count）
  - `*a4` 是输出位置（1-based index），未命中输出 0
  - 若 `count < 1`，直接 `*a4 = 0`
  - 特判开头/结尾：
    - 若 `*a1 <= a2[0]`，则 `*a4 = (*a1 >= a2[0]) ? 1 : 0`
    - 若 `*a1 >= a2[count-1]`，则 `*a4 = (*a1 <= a2[count-1]) ? count : 0`
  - 主体二分：维护区间 `[v7, v8]`（初始 `v7=1, v8=count`），当 `v7+1 < v8` 时二分；仅在“精确相等”时返回中点，否则最终返回 0。

### ATNAIN/ATNATX/ATNALO - `sub_1084F7C0`（0x1084F7C0）调用骨架

- 入口签名（反编译）：`int __cdecl sub_1084F7C0(int *a1, _DWORD *a2, int *a3, int *a4, int a5, int *a6, _DWORD *a7)`
- 关键输入/输出（结合代码推断）：
  - `a1`：主 key（循环变量 `i` 的初值）
  - `a2`：次 key（用于 `ATFIND` 做类型/模式校验）
  - `a3`：基准下标/起点（用于 `*v15 = v31 - *v12 + 1` 的差值计算）
  - `a4`：输出容量上限（与 `*v15` 取 min）
  - `a5`：输出数组地址（写入结果）
  - `a6`：输出数量（`*a6`）
  - `a7`：错误码输出（`*a7`），出现异常时写入 51/52/54/55/56/63/64/65 等

- 查找流程（关键点）：
  1. 校验 `*a1` 与 `*a2` 的 hash/code 范围（`531442..387951929`），否则错误码 63/51。
  2. `ATFIND(a2, key_table, &key_count, &v21)` 获取 `v21`（1-based）；若 `v21<=0` 错误码 51。
  3. 类型/模式校验：`v20(模式常量) == type_table[v21-1]`，不匹配错误码 52。
  4. 循环：以 `i = *a1` 为初值，执行：
     - `ATCHOP(&i, chop_table, &chop_count, &v24)`，若 `v24<=0` 错误码 54。
     - 从 `page/off` 表取定位信息：
       - `v25 = page_table[v24-1]`
       - `v26 = off_table[v24-1]`
     - 读页：`sub_1044FC20(&v25, &v27, a2)`，读到页号 `v27`（<=0 则错误码 56）。
     - 在页内通过 `v21`（来自 ATFIND 的索引）取某个控制字段：`v28 = page_words[512*v27 - 514 + v26 + v21]`
       - 若 `v28==0`，则尝试通过“备用 key”再次 `ATFIND` 得到 `v29`，并用 `v29` 读取另一个控制字段作为跳转；若仍为 0 或 -1 则错误码 55。
       - 若 `v28!=-1`，则令 `v30 = v26 + v28 - 1`，并继续从页内读取数据（`v31 = page_words[512*v27 - 513 + v30]` 等）。
     - 下一跳：循环尾部会令 `i = page_words[512*v27 - 513 + v30]`（相当于链表/树跳转）。

### ATNLOG - `sub_1084F1A5`（0x1084F1A5）复用同一套索引链

- `sub_1084F1A5` 的结构与 `sub_1084F7C0` 高度相似：
  - 先 `ATFIND` 得到索引 `v16`
  - 类型匹配后进入循环：`ATCHOP` -> `page/off` -> 读页 -> 页内偏移解析
  - 不同之处在于其输出模式（`v14`=1/3/4 等）决定：输出单值/布尔/或输出数组。

## 2025-12-29 阶段进展 09：IDA 关键函数语义化重命名（便于持续逆向与落地实现）

### 已完成的函数重命名（MCP 执行成功）

- `sub_10450144`（0x10450144）-> `attlib_ATFIND_linear_find_1based`
- `sub_104501F8`（0x104501F8）-> `attlib_ATCHOP_bsearch_exact_1based`
- `sub_1044FC20`（0x1044FC20）-> `attlib_read_page_cached_0x800`

- `sub_10852A64`（0x10852A64）-> `attlib_parse_ATGTIX_table`
- `sub_10852E20`（0x10852E20）-> `attlib_parse_ATGTDF_table`
- `sub_108533B4`（0x108533B4）-> `attlib_parse_ATGTSX_table`

- `sub_1084F7C0`（0x1084F7C0）-> `attlib_query_ATNAIN_ATNATX_ATNALO`
- `sub_1084F1A5`（0x1084F1A5）-> `attlib_query_ATNLOG`
- `sub_10851DA8`（0x10851DA8）-> `attlib_jump_table_ATGTLT`

- `RFLDFL_Load_AttlibDat`（0x10851210）-> `RFLDFL_Load_AttlibDat`（保持原名）

### 备注：后续全局表定位被 MCP 连接中断

- 在继续通过 MCP 搜索/定位 `unk_11BFA080`、`dword_11C02080`、`dword_11C0A080`、`dword_11C12210` 等关键全局数组的定义与写入点时，出现 “Failed to connect to IDA Pro / Timeout” 报错，推测 IDA 的 MCP 插件服务中断。
- 下一步需要在 IDA 中重新启动 MCP 服务（按报错提示：`Edit -> Plugins -> MCP` 或对应快捷键），恢复连接后才能继续做：
  - 全局数组的 xrefs_to / data_ref 搜索
  - 第二轮加载产物表（key/type/chop/page/off）的填充流程还原

## 2025-12-29 阶段进展 10：为 IDA 关键函数补充入口注释（已写入 IDB）

本阶段通过 `mcp1_set_comments` 已把关键函数的“用途/输入输出/关键约定（1-based）/关键全局表”写入 IDA 注释，便于后续持续逆向与落地实现。

### 已写入注释的函数与要点

- `attlib_ATFIND_linear_find_1based`（0x10450144）
  - 线性查找，输出位置为 1-based，未命中返回 0。

- `attlib_ATCHOP_bsearch_exact_1based`（0x104501F8）
  - 二分“精确命中”，输出位置为 1-based，未命中返回 0；不是 lower_bound。

- `attlib_read_page_cached_0x800`（0x1044FC20）
  - 0x800=2048B 页读取（512*u32），带缓存；数据落在全局页缓冲区（如 `dword_11C2A860` 相关区域）。

- `attlib_parse_ATGTIX_table`（0x10852A64）
  - ATGTIX 记录布局：2*u32 (code_or_hash, disp)，disp 拆为 page/off（/512 与 %512）。

- `attlib_parse_ATGTDF_table`（0x10852E20）
  - ATGTDF：3*u32 头 + 可变扩展；kind∈{1,2}；tag==4 走 n+ n*u32，否则 1*u32。

- `attlib_parse_ATGTSX_table`（0x108533B4）
  - ATGTSX：3*u32 (key,v1,v2)，遇 0 或 0xFFFFFFFF 终止；页尾 0 可能跨页继续。

- `attlib_query_ATNAIN_ATNATX_ATNALO`（0x1084F7C0）
  - 多级索引链路：ATFIND(线性)->类型校验->ATCHOP(精确二分)->page/off->读页->页内跳转。
  - 记录了典型错误码：51/52/54/55/56/63/64/65（具体含义需结合调用场景继续细化）。

- `attlib_query_ATNLOG`（0x1084F1A5）
  - 与 ATNAIN 查询高度相似，复用同一套 ATFIND/ATCHOP + page/off + 读页链路；mode 决定输出单值/布尔/数组。

- `attlib_jump_table_ATGTLT`（0x10851DA8）
  - ATGTLT 相关跳转/目录辅助逻辑（被部分查询路径调用）。

- `RFLDFL_Load_AttlibDat`（0x10851210）
  - attlib.dat 加载总入口：打开文件/读头/分段解析；会构建第二轮驻留表供查询链使用。

### 备注：MCP 连接短暂恢复但仍不稳定

- 注释写入阶段 MCP 连接正常，但在后续继续做全局数组定位（`mcp1_search` 等）时再次超时断开。
- 下一步继续分析前，建议在 IDA 中重新启动 MCP 服务（`Edit -> Plugins -> MCP`），并避免在短时间内发起过多并行查询；后续我将优先使用“少量、串行”的 xrefs/搜索调用以降低超时概率。
