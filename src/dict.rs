//! dabacon **Attribute Data File**（E3D `attlib.dat`：属性/类型字典）离线解析。
//!
//! 复刻 AVEVA E3D `core.dll` 的 `ATTOPE`/`ATGTIX`/`ATRDRC`/`ATNLOG` 读取链
//! （逆向证据见 `teach/learning-records/0003`、`0004`；ADR-004）。目的是**离线**
//! 从字典文件取每个 noun 的分类 flag（`primitive`/`geomset`/…），得到与 core.dll
//! 一致的类型分类，导出 `noun_flags.json` 供 gen-model 的 `NounClassifier` 使用。
//!
//! ## 格式（实测 `attlib.dat`，大端 · 页=512×i32=2KB）
//! - **raw page 1 = 头记录** `v47[0..7]`（8 个区段**逻辑**起始页；ATTOPE 读"逻辑页2"=raw1）。
//!   `v47[4]` = FIELD 区起始、`v47[6]` = NOUN 区起始。**逻辑页 N ↔ raw 页 N−1**。
//! - **NOUN 表**（`ATGTIX`/`sub_55F4FFC`，从 `v47[6]`）：`(key, addr)` 2-int 对，
//!   `addr = 逻辑page*512 + off`；`nounHash → (raw_page=addr/512−1, row_base=addr%512)`。
//! - **FIELD 表**（`sub_55F53B8`，从 `v47[4]`）：连续 4-int 记录 `[field_id, type, defA, defB]`；
//!   `col` = 字段在表中的序号（`ATFIND` 线性查找返回 1-based 位置）；type `1=bool/3=int/4=array`。
//! - **取值**（`ATNLOG`）：`value = raw_page[addr/512−1][row_base + col − 2]`（-2 来自 ATNLOG
//!   `512*slot-514` 相对页起始 `512*slot-512` 的偏移，col 为 1-based）。非 0 非 -1 = 命中；
//!   `0` → base_type 继承；`-1` → 默认表。
//!
//! Rust 的整数除法向下取整（PowerShell `[int]` 会四舍五入，早前踩过坑）——本实现直接用之。

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

/// 每页 i32 个数（core.dll 里 `512 * slot` 的步长）。
pub const PAGE_INTS: usize = 512;
/// 每页字节数 `0x800`。
pub const PAGE_BYTES: usize = PAGE_INTS * 4;

/// index 条目 key 的有效区间 `[531442, 387951929]`（`ATGTIX`）。
pub const KEY_MIN: i32 = 531442;
pub const KEY_MAX: i32 = 387951929;

// ── noun 分类字段的 dabacon field-id（实测，见 0003/0004）──────────────────
pub const FIELD_PRIMITIVE: i32 = 659518;
pub const FIELD_GEOMSET: i32 = 859903;
pub const FIELD_EXTRUSION: i32 = 663225;
pub const FIELD_ISPOINTSETPOINT: i32 = 290555737;
pub const FIELD_GRAPHICS_BEHAVIOUR: i32 = 5099119;
/// base_type 字段 id（ATNLOG 里的 `unk_5DAEB9C`，实测 = 837586）：cell==0 时沿它继承。
pub const FIELD_BASE_TYPE: i32 = 837586;

// ── 其余 DB_Noun 能力字段（core.dll 访问器逐个反编译取得，见 v2 测试计划 §2）──
//
// 取自 `DB_Noun::<accessor>` 的 `internalGetField(field)` 立即数，或 `ReadData`/`ReadDataDab`
// 里 `sub_55BC8DC` 的字段号。`DB_Noun::primaryList` 走 `db_get_element_info(297853135)`
// 而非 dabacon，**不在本文件可得**，故不在此列。

/// `DB_Noun::point`(0x58da1c0)：顶点/点容器 noun（LOOP/PLOO/PAVE/VERT/SPINE…）。
pub const FIELD_POINT: i32 = 661624;
/// `DB_Noun::positiveEquivalent`(0x58da1e0)：负体 noun 的正体等价类型 hash。
pub const FIELD_POSITIVE_EQUIVALENT: i32 = 778791;
/// `DB_Noun::changeType`(0x58d7630)：变化等价类的目标 noun hash（0 表示自成一类）。
pub const FIELD_CHANGE_TYPE: i32 = 76272573;
/// `DB_Noun::spatialMap`(0x58db9d0)：空间索引参与方式。
pub const FIELD_SPATIAL_MAP: i32 = 847458;
/// `DB_Noun::secondaryHierarchy`(0x58db900)：owner 链之外的次层级归属。
pub const FIELD_SECONDARY_HIERARCHY: i32 = 65664829;
/// `DB_Noun::defined`(0x58d7860)：定义级别；`== 4` 即 `isPseudo`。
pub const FIELD_DEFINED: i32 = 713101;
/// `DB_Noun::validc`(0x58dbc10)：校验分类。
pub const FIELD_VALIDC: i32 = 45889870;
/// `DB_Noun::visible`(0x58dbc30)。
pub const FIELD_VISIBLE: i32 = 722704;
/// `DB_Noun::toplevel`(0x58dba70)。
pub const FIELD_TOPLEVEL: i32 = 661628;
/// `DB_Noun::pickable`(0x58da1a0)。
pub const FIELD_PICKABLE: i32 = 750400;
/// `DB_Noun::world`(0x58dbc50)。
pub const FIELD_WORLD: i32 = 843594;
/// `DB_Noun::defaultVolumeQuery`(0x58d7840)。
pub const FIELD_DEFAULT_VOLUME_QUERY: i32 = 89369995;
/// `DB_Noun::clasherWithin`(0x58d7670)。
pub const FIELD_CLASHER_WITHIN: i32 = 206078421;
/// `DB_Noun::clasherSection`(0x58d7650)。
pub const FIELD_CLASHER_SECTION: i32 = 46622793;
/// `DB_Noun::modifiable`(0x58da0e0)。
pub const FIELD_MODIFIABLE: i32 = 621476;
/// `DB_Noun::statusEligible`(0x58dba10)。
pub const FIELD_STATUS_ELIGIBLE: i32 = 204468292;
/// `DB_Noun::isCloneable`(0x58d9df0)。
pub const FIELD_IS_CLONEABLE: i32 = 3475470;
/// `DB_Noun::requiresMarine`(0x58db8e0)。
pub const FIELD_REQUIRES_MARINE: i32 = 259611633;
/// `DB_Noun::isProtected`(0x58da070)。
pub const FIELD_IS_PROTECTED: i32 = 212119090;
/// `DB_Noun::deleteMemberOnCopy`(0x58d7880)。
pub const FIELD_DELETE_MEMBER_ON_COPY: i32 = 193546290;
/// `DB_Noun::spoolerModifiable`(0x58db9f0)。
pub const FIELD_SPOOLER_MODIFIABLE: i32 = 208122411;
/// `DB_Noun::psOwner`(0x58db510)。
pub const FIELD_PS_OWNER: i32 = 266716114;
/// `DB_Noun::psNext`(0x58db4f0)。
pub const FIELD_PS_NEXT: i32 = 300373315;
/// `DB_Noun::psFirstMember`(0x58db4d0)。
pub const FIELD_PS_FIRST_MEMBER: i32 = 297966157;

/// `DB_Noun::primaryList` 的字段号。**不在 dabacon 字典里**——`ReadDataDab`(0x58d7100) 用
/// `db_get_element_info(hash, 297853135)` 从运行库读取。列在此仅为记录来源，
/// [`AttrDataFile`] 查询它必然返回 `None`。
pub const FIELD_PRIMARY_LIST_NOT_IN_DICT: i32 = 297853135;
/// noun 继承链上溯上限（防环）。
const MAX_INHERIT_DEPTH: usize = 32;

/// 头记录在 raw page 1；其 8 个 i32 是各区段的**逻辑**起始页。
pub const HEADER_RAW_PAGE: usize = 1;
/// `v47` 下标：FIELD 区起始。
pub const HDR_FIELD_START: usize = 4;
/// `v47` 下标：NOUN 区起始。
pub const HDR_NOUN_START: usize = 6;

/// 字段类型码（`dword_6C21070[col]`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Bool,  // 1 → `==1` 为真
    Int,   // 3
    Array, // 4
    Other(i32),
}
impl FieldType {
    pub fn from_code(c: i32) -> Self {
        match c {
            1 => FieldType::Bool,
            3 => FieldType::Int,
            4 => FieldType::Array,
            x => FieldType::Other(x),
        }
    }
}

/// 一个字段的定位：1-based 列号 + 类型 + 默认值。
#[derive(Debug, Clone, Copy)]
pub struct FieldSlot {
    pub col: usize,
    pub ty: FieldType,
    pub default: i32,
}

/// 一个 noun 记录的定位（raw 页 + 行基址）。
#[derive(Debug, Clone, Copy)]
pub struct NounSlot {
    pub raw_page: usize,
    pub row_base: usize,
}

/// 已解析的 Attribute Data File。
pub struct AttrDataFile {
    pages: Vec<[i32; PAGE_INTS]>,
    header: [i32; 8],
    noun_index: HashMap<i32, NounSlot>,
    field_index: HashMap<i32, FieldSlot>,
}

impl AttrDataFile {
    pub fn open(path: &Path) -> Result<Self> {
        let bytes =
            std::fs::read(path).map_err(|e| anyhow!("open Attribute Data File {:?}: {e}", path))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let pages = split_pages(bytes);
        if pages.len() <= HEADER_RAW_PAGE {
            return Err(anyhow!("Attribute Data File 页数不足（{}）", pages.len()));
        }
        let mut header = [0i32; 8];
        header.copy_from_slice(&pages[HEADER_RAW_PAGE][0..8]);

        let field_index = build_field_index(&pages, logical_to_raw(header[HDR_FIELD_START]));
        let noun_index = build_noun_index(&pages, logical_to_raw(header[HDR_NOUN_START]));

        Ok(Self {
            pages,
            header,
            noun_index,
            field_index,
        })
    }

    pub fn header(&self) -> &[i32; 8] {
        &self.header
    }
    pub fn noun_count(&self) -> usize {
        self.noun_index.len()
    }
    pub fn field_count(&self) -> usize {
        self.field_index.len()
    }
    /// 该 noun hash 是否存在于 noun 索引（区分"命中且 flag=false"与"根本没这个 noun"）。
    pub fn has_noun(&self, noun_hash: i32) -> bool {
        self.noun_index.contains_key(&noun_hash)
    }

    #[inline]
    fn cell(&self, raw_page: usize, idx: usize) -> Option<i32> {
        self.pages.get(raw_page).and_then(|p| p.get(idx).copied())
    }

    /// `ATNLOG`(`sub_55BC98B`) **两级取值** + base_type 继承 + 默认兜底。
    ///
    /// 关键：col 索引到的**不是值、是记录内偏移**（早前 archive 误把它当值 → 读到"名字/偏移"区）：
    /// 1. `off = page[row_base + col − 2]`（step1，col 索引的槽表，−2 为 ATNLOG 槽对齐）；
    /// 2. `off == -1` → 默认表兜底；`off == 0` → 沿 `base_type` 继承（读基类 noun 再查）；否则
    /// 3. `value = page[row_base + off − 2]`（step2，才是真正的 bool/int 值）。
    pub fn raw_field(&self, noun_hash: i32, field_id: i32) -> Option<i32> {
        let fs = *self.field_index.get(&field_id)?;
        let mut cur = noun_hash;
        for _ in 0..MAX_INHERIT_DEPTH {
            let ns = *self.noun_index.get(&cur)?;
            let off = self.slot_at(&ns, fs.col)?;
            if off == -1 {
                return Some(fs.default); // -1 → 默认表兜底
            }
            if off != 0 {
                return self.value_at(&ns, off); // 命中：off 指向真值
            }
            // off == 0 → base_type 继承：base_type 字段的真值 = 基类 noun hash
            let bts = *self.field_index.get(&FIELD_BASE_TYPE)?;
            let bt_off = self.slot_at(&ns, bts.col)?;
            if bt_off <= 0 {
                return Some(fs.default); // 无 base_type → 默认
            }
            match self.value_at(&ns, bt_off) {
                Some(base) if base != 0 && base != -1 => cur = base, // 上溯基类
                _ => return Some(fs.default),
            }
        }
        None // 继承链过深（疑似环）
    }

    /// step1：col 索引的槽表项（记录内偏移；-1=默认 / 0=继承）。`page[row_base + col − 2]`。
    #[inline]
    fn slot_at(&self, ns: &NounSlot, col: usize) -> Option<i32> {
        self.cell(ns.raw_page, ns.row_base + col.saturating_sub(2))
    }

    /// step2：用偏移 `off` 取记录内真值。`page[row_base + off − 2]`。
    #[inline]
    fn value_at(&self, ns: &NounSlot, off: i32) -> Option<i32> {
        if off <= 0 {
            return None;
        }
        self.cell(ns.raw_page, ns.row_base + (off as usize).saturating_sub(2))
    }

    pub fn field_bool(&self, noun_hash: i32, field_id: i32) -> bool {
        self.raw_field(noun_hash, field_id) == Some(1)
    }
    pub fn field_int(&self, noun_hash: i32, field_id: i32) -> Option<i32> {
        self.raw_field(noun_hash, field_id)
    }

    pub fn noun_flags(&self, noun_hash: i32) -> NounFlags {
        NounFlags {
            noun_hash,
            noun_name: dehash_noun(noun_hash),
            primitive: self.field_bool(noun_hash, FIELD_PRIMITIVE),
            geomset: self.field_bool(noun_hash, FIELD_GEOMSET),
            extrusion: self.field_bool(noun_hash, FIELD_EXTRUSION),
            is_pointset_point: self.field_bool(noun_hash, FIELD_ISPOINTSETPOINT),
            graphics_behaviour: self.field_int(noun_hash, FIELD_GRAPHICS_BEHAVIOUR),
        }
    }

    pub fn all_noun_flags(&self) -> Vec<NounFlags> {
        let mut hashes: Vec<i32> = self.noun_index.keys().copied().collect();
        hashes.sort_unstable();
        hashes.into_iter().map(|h| self.noun_flags(h)).collect()
    }

    /// 读取一个 noun 的**完整**能力集（`DB_Noun` 全部可从 dabacon 取得的访问器）。
    pub fn noun_capabilities(&self, noun_hash: i32) -> NounCapabilities {
        let b = |f: i32| self.field_bool(noun_hash, f);
        let i = |f: i32| self.field_int(noun_hash, f);
        NounCapabilities {
            noun_hash,
            noun_name: dehash_noun(noun_hash),
            primitive: b(FIELD_PRIMITIVE),
            geomset: b(FIELD_GEOMSET),
            extrusion: b(FIELD_EXTRUSION),
            point: b(FIELD_POINT),
            is_pointset_point: b(FIELD_ISPOINTSETPOINT),
            graphics_behaviour: i(FIELD_GRAPHICS_BEHAVIOUR),
            change_type: i(FIELD_CHANGE_TYPE),
            positive_equivalent: i(FIELD_POSITIVE_EQUIVALENT),
            spatial_map: i(FIELD_SPATIAL_MAP),
            secondary_hierarchy: i(FIELD_SECONDARY_HIERARCHY),
            defined: i(FIELD_DEFINED),
            validc: i(FIELD_VALIDC),
            visible: b(FIELD_VISIBLE),
            toplevel: b(FIELD_TOPLEVEL),
            pickable: b(FIELD_PICKABLE),
            world: b(FIELD_WORLD),
            default_volume_query: b(FIELD_DEFAULT_VOLUME_QUERY),
            clasher_within: b(FIELD_CLASHER_WITHIN),
            clasher_section: b(FIELD_CLASHER_SECTION),
            modifiable: b(FIELD_MODIFIABLE),
            status_eligible: b(FIELD_STATUS_ELIGIBLE),
            is_cloneable: b(FIELD_IS_CLONEABLE),
            requires_marine: b(FIELD_REQUIRES_MARINE),
            is_protected: b(FIELD_IS_PROTECTED),
            delete_member_on_copy: b(FIELD_DELETE_MEMBER_ON_COPY),
            spooler_modifiable: b(FIELD_SPOOLER_MODIFIABLE),
            ps_owner: i(FIELD_PS_OWNER),
            ps_next: i(FIELD_PS_NEXT),
            ps_first_member: i(FIELD_PS_FIRST_MEMBER),
        }
    }

    pub fn all_noun_capabilities(&self) -> Vec<NounCapabilities> {
        let mut hashes: Vec<i32> = self.noun_index.keys().copied().collect();
        hashes.sort_unstable();
        hashes
            .into_iter()
            .map(|h| self.noun_capabilities(h))
            .collect()
    }
}

/// 一个 noun 的分类 flag（导出到 `noun_flags.json` 的一行）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NounFlags {
    pub noun_hash: i32,
    pub noun_name: String,
    pub primitive: bool,
    pub geomset: bool,
    pub extrusion: bool,
    pub is_pointset_point: bool,
    pub graphics_behaviour: Option<i32>,
}

/// 导出 `noun_flags.json`。
pub fn export_noun_flags(attr_file: &Path, out_json: &Path) -> Result<()> {
    let df = AttrDataFile::open(attr_file)?;
    let flags = df.all_noun_flags();
    let json = serde_json::to_string_pretty(&flags)?;
    std::fs::write(out_json, json.into_bytes())
        .map_err(|e| anyhow!("write {:?}: {e}", out_json))?;
    Ok(())
}

/// 一个 noun 的**完整**能力集：`DB_Noun` 里所有可从 dabacon 取到的访问器。
///
/// 字段名与 `DB_Noun` 的访问器一一对应（蛇形化）。`primaryList` 不在其中——它走
/// `db_get_element_info`，见 [`FIELD_PRIMARY_LIST_NOT_IN_DICT`]。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NounCapabilities {
    pub noun_hash: i32,
    pub noun_name: String,

    // ── 直接几何能力 ────────────────────────────────────────────────────────
    pub primitive: bool,
    pub geomset: bool,
    pub extrusion: bool,
    /// 顶点/点容器（LOOP/PLOO/PAVE/VERT/SPINE…）——「不能当生成根」的权威名单来源。
    pub point: bool,
    pub is_pointset_point: bool,

    // ── 生成与变化路由 ──────────────────────────────────────────────────────
    pub graphics_behaviour: Option<i32>,
    /// 变化等价类目标 noun hash；0 表示自成一类。
    pub change_type: Option<i32>,
    /// 负体 → 正体等价类型 hash；0 表示不是负体。
    pub positive_equivalent: Option<i32>,
    pub spatial_map: Option<i32>,
    pub secondary_hierarchy: Option<i32>,
    /// `== 4` 即 `DB_Noun::isPseudo`。
    pub defined: Option<i32>,
    pub validc: Option<i32>,

    // ── 显示与交互 ──────────────────────────────────────────────────────────
    pub visible: bool,
    pub toplevel: bool,
    pub pickable: bool,
    pub world: bool,
    pub default_volume_query: bool,
    pub clasher_within: bool,
    pub clasher_section: bool,

    // ── 生命周期与权限 ──────────────────────────────────────────────────────
    pub modifiable: bool,
    pub status_eligible: bool,
    pub is_cloneable: bool,
    pub requires_marine: bool,
    pub is_protected: bool,
    pub delete_member_on_copy: bool,
    pub spooler_modifiable: bool,

    // ── pointset 遍历链 ─────────────────────────────────────────────────────
    pub ps_owner: Option<i32>,
    pub ps_next: Option<i32>,
    pub ps_first_member: Option<i32>,
}

impl NounCapabilities {
    /// 三个直接几何能力的并集——生成能力基线。
    pub fn has_direct_geometry(&self) -> bool {
        self.primitive || self.geomset || self.extrusion
    }

    /// `DB_Noun::isPseudo`：`defined == 4`。
    pub fn is_pseudo(&self) -> bool {
        self.defined == Some(4)
    }

    /// 该 noun 所属变化等价类的目标 hash：字典值为 0/缺失时按 core.dll `ReadData`
    /// 的回填规则取自身 hash。
    pub fn change_class_hash(&self) -> i32 {
        match self.change_type {
            Some(h) if h != 0 => h,
            _ => self.noun_hash,
        }
    }

    /// 负体的正体等价 hash（不是负体时为 `None`）。
    pub fn positive_equivalent_hash(&self) -> Option<i32> {
        self.positive_equivalent.filter(|h| *h != 0 && *h != -1)
    }
}

/// 导出 `noun_caps.json`（紧凑，供 [`default_noun_capabilities`] 内嵌）。
pub fn export_noun_capabilities(attr_file: &Path, out_json: &Path) -> Result<()> {
    let df = AttrDataFile::open(attr_file)?;
    let caps = df.all_noun_capabilities();
    let json = serde_json::to_string(&caps)?;
    std::fs::write(out_json, json.into_bytes())
        .map_err(|e| anyhow!("write {:?}: {e}", out_json))?;
    Ok(())
}

/// 全局默认能力矩阵：加载 crate 内嵌的 `noun_caps.json`（由 [`export_noun_capabilities`]
/// 从 `attlib.dat` 生成）。解析失败退化为空表。
pub fn default_noun_capabilities() -> &'static NounCapabilityTable {
    static DEFAULT_CAPS: OnceLock<NounCapabilityTable> = OnceLock::new();
    DEFAULT_CAPS.get_or_init(|| {
        NounCapabilityTable::from_json_str(include_str!("../noun_caps.json")).unwrap_or_default()
    })
}

/// 全量能力矩阵的查询视图。
#[derive(Debug, Clone, Default)]
pub struct NounCapabilityTable {
    by_hash: HashMap<i32, NounCapabilities>,
    name_to_hash: HashMap<String, i32>,
}

impl NounCapabilityTable {
    pub fn from_caps(caps: Vec<NounCapabilities>) -> Self {
        let mut by_hash = HashMap::with_capacity(caps.len());
        let mut name_to_hash = HashMap::with_capacity(caps.len());
        for c in caps {
            let key = c.noun_name.trim().to_ascii_uppercase();
            if !key.is_empty() {
                name_to_hash.insert(key, c.noun_hash);
            }
            by_hash.insert(c.noun_hash, c);
        }
        Self {
            by_hash,
            name_to_hash,
        }
    }

    pub fn from_json_str(s: &str) -> Result<Self> {
        let caps: Vec<NounCapabilities> =
            serde_json::from_str(s).map_err(|e| anyhow!("parse noun_caps.json: {e}"))?;
        Ok(Self::from_caps(caps))
    }

    pub fn len(&self) -> usize {
        self.by_hash.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    pub fn get(&self, noun: &str) -> Option<&NounCapabilities> {
        let h = *self.name_to_hash.get(&noun.trim().to_ascii_uppercase())?;
        self.by_hash.get(&h)
    }
    pub fn by_hash(&self, noun_hash: i32) -> Option<&NounCapabilities> {
        self.by_hash.get(&noun_hash)
    }

    pub fn iter(&self) -> impl Iterator<Item = &NounCapabilities> {
        self.by_hash.values()
    }

    /// 命中谓词的 noun 名（大写、升序、去重、跳过无名项）。
    pub fn nouns_where(&self, pred: impl Fn(&NounCapabilities) -> bool) -> Vec<String> {
        let mut v: Vec<String> = self
            .by_hash
            .values()
            .filter(|c| pred(c) && !c.noun_name.trim().is_empty())
            .map(|c| c.noun_name.trim().to_ascii_uppercase())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// 顶点/点容器名单（`point == true`）。
    pub fn point_nouns(&self) -> Vec<String> {
        self.nouns_where(|c| c.point)
    }

    /// 直接几何能力名单（`primitive ∪ geomset ∪ extrusion`）。
    pub fn direct_geometry_nouns(&self) -> Vec<String> {
        self.nouns_where(|c| c.has_direct_geometry())
    }

    /// 伪类型名单（`defined == 4`）。
    pub fn pseudo_nouns(&self) -> Vec<String> {
        self.nouns_where(|c| c.is_pseudo())
    }

    /// 负体 → 正体名字映射（只含字典登记了 `positiveEquivalent` 的 noun）。
    pub fn positive_equivalents(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .by_hash
            .values()
            .filter_map(|c| {
                let target = c.positive_equivalent_hash()?;
                let to = self.by_hash.get(&target)?;
                (!c.noun_name.trim().is_empty() && !to.noun_name.trim().is_empty()).then(|| {
                    (
                        c.noun_name.trim().to_ascii_uppercase(),
                        to.noun_name.trim().to_ascii_uppercase(),
                    )
                })
            })
            .collect();
        v.sort_unstable();
        v
    }

    /// 一个 noun 的变化等价类名（目标 noun 未登记在字典时用 `db1_dehash` 还原，
    /// 例如抽象类 `LINEAR`/`MULTC`/`INLINE`/`PCONN`）。
    pub fn change_class_name(&self, noun: &str) -> Option<String> {
        let c = self.get(noun)?;
        let h = c.change_class_hash();
        Some(match self.by_hash.get(&h) {
            Some(t) if !t.noun_name.trim().is_empty() => t.noun_name.trim().to_ascii_uppercase(),
            _ => dehash_noun(h).trim().to_ascii_uppercase(),
        })
    }

    /// 全部非平凡变化等价类：`类名 → 成员名单`（只含 `change_type` 非 0 的 noun）。
    pub fn change_classes(&self) -> std::collections::BTreeMap<String, Vec<String>> {
        let mut out: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        for c in self.by_hash.values() {
            let Some(h) = c.change_type.filter(|h| *h != 0) else {
                continue;
            };
            if c.noun_name.trim().is_empty() {
                continue;
            }
            let class = match self.by_hash.get(&h) {
                Some(t) if !t.noun_name.trim().is_empty() => {
                    t.noun_name.trim().to_ascii_uppercase()
                }
                _ => dehash_noun(h).trim().to_ascii_uppercase(),
            };
            out.entry(class)
                .or_default()
                .push(c.noun_name.trim().to_ascii_uppercase());
        }
        for v in out.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        out
    }
}

// ── NounClassifier（ADR-004 阶段 3）──────────────────────────────────────────
//
// 以 core.dll dabacon 字典的 per-noun flag 为准的类型分类器，复刻 `DB_Noun` 的
// `primitive/geomset/extrusion/isPointsetPoint/graphicsBehaviour`(+`hashValue`/`findNoun`)。
// 数据来自 `noun_flags.json`（`AttrDataFile` 从 `attlib.dat` 解析导出）。
//
// ⚠️ 注意：dict 的 `primitive` 语义 = "设计级几何叶子 noun"（含**管件** ELBO/VALV/TUBI…），
// 与 gen-model 现有的**生成路由**名单（`GNERAL_PRIM_NOUN_NAMES`→prim_model）**不等价**——
// 管件必须走 catalogue/piping 路径。故本分类器是**权威 flag 源**，替换路由名单须逐个核对
// （见 `crosscheck_curated_noun_lists` / 分类器的 divergence 报告），不能盲替。

/// 以 dict flag 为准的 noun 分类器。
#[derive(Debug, Clone, Default)]
pub struct NounClassifier {
    by_hash: HashMap<i32, NounFlags>,
    /// 归一化(大写去空)名 → noun_hash。
    name_to_hash: HashMap<String, i32>,
}

impl NounClassifier {
    /// 从一组 `NounFlags` 构建（如 `AttrDataFile::all_noun_flags`）。
    pub fn from_flags(flags: Vec<NounFlags>) -> Self {
        let mut by_hash = HashMap::with_capacity(flags.len());
        let mut name_to_hash = HashMap::with_capacity(flags.len());
        for f in flags {
            let key = f.noun_name.trim().to_ascii_uppercase();
            if !key.is_empty() {
                name_to_hash.insert(key, f.noun_hash);
            }
            by_hash.insert(f.noun_hash, f);
        }
        Self {
            by_hash,
            name_to_hash,
        }
    }

    /// 从 `noun_flags.json` 文本构建。
    pub fn from_json_str(s: &str) -> Result<Self> {
        let flags: Vec<NounFlags> =
            serde_json::from_str(s).map_err(|e| anyhow!("parse noun_flags.json: {e}"))?;
        Ok(Self::from_flags(flags))
    }

    /// 从 `noun_flags.json` 路径构建。
    pub fn from_json_path(path: &Path) -> Result<Self> {
        let s = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("read noun_flags.json {:?}: {e}", path))?;
        Self::from_json_str(&s)
    }

    /// 直接从 `attlib.dat` 解析并构建（离线，无需先导出 json）。
    pub fn from_attr_file(attr_file: &Path) -> Result<Self> {
        Ok(Self::from_flags(
            AttrDataFile::open(attr_file)?.all_noun_flags(),
        ))
    }

    pub fn len(&self) -> usize {
        self.by_hash.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// db1 hash（复刻 `DB_Noun::hashValue`：noun 名 → hash）。
    pub fn hash_value(noun: &str) -> i32 {
        aios_core::tool::db_tool::db1_hash(noun.trim().to_ascii_uppercase().as_str()) as i32
    }

    /// 按 noun 名取 flag（大小写/空白不敏感）。
    pub fn flags(&self, noun: &str) -> Option<&NounFlags> {
        let h = *self.name_to_hash.get(&noun.trim().to_ascii_uppercase())?;
        self.by_hash.get(&h)
    }

    /// 复刻 `DB_Noun::findNoun`：按 noun hash 定位分类记录。
    pub fn find_noun(&self, noun_hash: i32) -> Option<&NounFlags> {
        self.by_hash.get(&noun_hash)
    }

    /// noun 是否存在于字典。
    pub fn contains(&self, noun: &str) -> bool {
        self.name_to_hash
            .contains_key(&noun.trim().to_ascii_uppercase())
    }

    // ── 逐 noun 分类谓词（未知 noun → false，保守）──────────────────────────
    pub fn primitive(&self, noun: &str) -> bool {
        self.flags(noun).map_or(false, |f| f.primitive)
    }
    pub fn geomset(&self, noun: &str) -> bool {
        self.flags(noun).map_or(false, |f| f.geomset)
    }
    pub fn extrusion(&self, noun: &str) -> bool {
        self.flags(noun).map_or(false, |f| f.extrusion)
    }
    pub fn is_pointset_point(&self, noun: &str) -> bool {
        self.flags(noun).map_or(false, |f| f.is_pointset_point)
    }
    pub fn graphics_behaviour(&self, noun: &str) -> Option<i32> {
        self.flags(noun).and_then(|f| f.graphics_behaviour)
    }

    // ── 集合访问器（返回命中某 flag 的 noun 名，升序）——供需要"名单"的消费点 ──
    fn nouns_where(&self, pred: impl Fn(&NounFlags) -> bool) -> Vec<String> {
        let mut v: Vec<String> = self
            .by_hash
            .values()
            .filter(|f| pred(f) && !f.noun_name.trim().is_empty())
            .map(|f| f.noun_name.trim().to_ascii_uppercase())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }
    pub fn primitive_nouns(&self) -> Vec<String> {
        self.nouns_where(|f| f.primitive)
    }
    pub fn geomset_nouns(&self) -> Vec<String> {
        self.nouns_where(|f| f.geomset)
    }
    pub fn extrusion_nouns(&self) -> Vec<String> {
        self.nouns_where(|f| f.extrusion)
    }

    /// 负体候选名单（**启发式**：`N` 前缀 ∩ 几何(primitive/geomset/extrusion)）。
    ///
    /// ⚠️ dict 的 5 个已 RE flag 里**没有**显式 negative 字段（ADR-006 未决），故负体只能
    /// 启发式推断。实测（Phase 1-A）：curated `TOTAL_NEG`(23) 是本集合的**干净子集**；本启发式
    /// 相对 `TOTAL_NEG` **多出** `NBXI/NPOLYH/NSLC/NTUB`(候选真负体) + **误纳** `NOZZ`(实为喷嘴正体)。
    /// ⇒ 生产用途**不要**直接用本集合当负体名单；应 = `TOTAL_NEG ∪ {NBXI,NPOLYH,NSLC,NTUB}`
    /// 并排除 `NOZZ`，或将来 RE dict negative 字段做数据驱动。本方法仅供交叉核对/发现用。
    pub fn negative_candidate_nouns(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .by_hash
            .values()
            .filter(|f| f.primitive || f.geomset || f.extrusion)
            .map(|f| f.noun_name.trim().to_ascii_uppercase())
            .filter(|n| !n.is_empty() && n.starts_with('N'))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

/// 全局默认 `NounClassifier`：加载 **crate 内嵌**的 `noun_flags.json`（与 `all_attr_info.json`
/// 同款内嵌，无运行期文件依赖）。首次调用懒加载；解析失败退化为空分类器（谓词全 false，
/// 保守不误判）。gen-model 侧后续（ADR-006 阶段 3）用它把路由判定从硬编码名单迁到 dict flag。
pub fn default_noun_classifier() -> &'static NounClassifier {
    static DEFAULT_CLASSIFIER: OnceLock<NounClassifier> = OnceLock::new();
    DEFAULT_CLASSIFIER.get_or_init(|| {
        NounClassifier::from_json_str(include_str!("../noun_flags.json")).unwrap_or_default()
    })
}

/// gen-model 全部几何路由/覆盖名单的并集（近似「顶层任一路径能处理」的 noun 集）。
///
/// 缺口清单（`export_stage3_gap_report`）与运行期覆盖观测共用本函数，避免两处口径漂移。
pub fn routing_coverage_nouns() -> &'static BTreeSet<String> {
    static COVERAGE: OnceLock<BTreeSet<String>> = OnceLock::new();
    COVERAGE.get_or_init(|| {
        use aios_core::pdms_types::{
            CATA_GEO_NAMES, CATA_WITHOUT_REUSE_GEO_NAMES, GENRAL_NEG_NOUN_NAMES,
            GENRAL_POS_NOUN_NAMES, GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES,
            PIPING_NOUN_NAMES, POHE_GEO_NAMES, PRIMITIVE_NOUN_NAMES, TOTAL_CATA_GEO_NOUN_NAMES,
            TOTAL_CONTAIN_NGMR_GEO_NAEMS, TOTAL_GEO_NOUN_NAMES, TOTAL_LOOP_NOUN_NAMES,
            TOTAL_NEG_NOUN_NAMES, TOTAL_VERT_NOUN_NAMES, USE_CATE_NOUN_NAMES, VISBILE_GEO_NOUNS,
        };
        let mut coverage: BTreeSet<String> = BTreeSet::new();
        for list in [
            PRIMITIVE_NOUN_NAMES.as_slice(),
            GNERAL_PRIM_NOUN_NAMES.as_slice(),
            GNERAL_LOOP_OWNER_NOUN_NAMES.as_slice(),
            USE_CATE_NOUN_NAMES.as_slice(),
            PIPING_NOUN_NAMES.as_slice(),
            GENRAL_NEG_NOUN_NAMES.as_slice(),
            TOTAL_NEG_NOUN_NAMES.as_slice(),
            TOTAL_VERT_NOUN_NAMES.as_slice(),
            TOTAL_LOOP_NOUN_NAMES.as_slice(),
            GENRAL_POS_NOUN_NAMES.as_slice(),
            TOTAL_GEO_NOUN_NAMES.as_slice(),
            TOTAL_CATA_GEO_NOUN_NAMES.as_slice(),
            CATA_GEO_NAMES.as_slice(),
            CATA_WITHOUT_REUSE_GEO_NAMES.as_slice(),
            VISBILE_GEO_NOUNS.as_slice(),
            TOTAL_CONTAIN_NGMR_GEO_NAEMS.as_slice(),
            POHE_GEO_NAMES.as_slice(),
        ] {
            coverage.extend(list.iter().map(|s| s.trim().to_ascii_uppercase()));
        }
        coverage
    })
}

/// dict 认定为几何（`primitive ∪ geomset ∪ extrusion`）却不在任何路由/覆盖名单里的 noun。
///
/// ⚠️ 这是**上界**，不是 bug 列表：gen-model 是层级式生成，管件 / 吊架件 / 暖通 / 桥架等
/// 子孙 noun 在生成根子树深度展开时经 catalogue 渲染，本就不需要进任何顶层名单。真实缺口
/// 只能由运行期观测（`fast_model::coverage_audit`）收敛。
pub fn uncovered_geometry_nouns(clf: &NounClassifier) -> BTreeSet<String> {
    let coverage = routing_coverage_nouns();
    clf.primitive_nouns()
        .into_iter()
        .chain(clf.geomset_nouns())
        .chain(clf.extrusion_nouns())
        .filter(|noun| !coverage.contains(noun))
        .collect()
}

// ── 内部工具 ────────────────────────────────────────────────────────────────

/// 逻辑页 → raw 页（`raw = logical − 1`；raw0 = 文本头，逻辑从 1 起）。
#[inline]
fn logical_to_raw(logical_page: i32) -> usize {
    (logical_page - 1).max(0) as usize
}

/// 按 0x800 切页；每页读成 512 个**大端** i32（不足一页的尾部忽略）。
fn split_pages(bytes: &[u8]) -> Vec<[i32; PAGE_INTS]> {
    let mut pages = Vec::with_capacity(bytes.len() / PAGE_BYTES);
    for chunk in bytes.chunks_exact(PAGE_BYTES) {
        let mut page = [0i32; PAGE_INTS];
        for (i, w) in chunk.chunks_exact(4).enumerate() {
            page[i] = i32::from_be_bytes([w[0], w[1], w[2], w[3]]);
        }
        pages.push(page);
    }
    pages
}

/// NOUN 索引（`ATGTIX`）：从 `raw_start` 起 walk `(key, addr)` 2-int 对，跨页。
/// `addr = 逻辑page*512 + off` → `(raw_page=addr/512−1, row_base=addr%512)`。
fn build_noun_index(pages: &[[i32; PAGE_INTS]], raw_start: usize) -> HashMap<i32, NounSlot> {
    let mut map = HashMap::new();
    let mut page = raw_start;
    'outer: while page < pages.len() {
        let p = &pages[page];
        let mut i = 0;
        while i + 1 < PAGE_INTS {
            let key = p[i];
            if key == -1 {
                break 'outer;
            }
            if key == 0 {
                break; // 本页结束 → 下一页
            }
            if key < KEY_MIN || key > KEY_MAX {
                break;
            }
            let addr = p[i + 1] as i64;
            if addr > 0 {
                let raw_page = (addr / PAGE_INTS as i64 - 1).max(0) as usize;
                let row_base = (addr % PAGE_INTS as i64) as usize;
                map.insert(key, NounSlot { raw_page, row_base });
            }
            i += 2;
        }
        page += 1;
    }
    map
}

/// FIELD 索引（`sub_55F53B8`）：从 `raw_start` 起读连续 4-int 记录
/// `[field_id, type, defA, defB]`，col = 1-based 序号；跨页直到遇到非 key 终止。
///
/// FIELD 记录是**变长流**（`ATGTDF`/`sub_55F53B8`，每页从 offset 0 起、0 结尾）：
/// 每条 = `[key, type, hasDef]`，若 `hasDef==2` 再跟默认：`type==4`(数组)→`[len, len×elem]`；
/// 否则→`[default]`；`hasDef==1` 无默认负载。`col` = 1-based 出现序号；`type` 1/3/4。
fn build_field_index(pages: &[[i32; PAGE_INTS]], raw_start: usize) -> HashMap<i32, FieldSlot> {
    let mut map = HashMap::new();
    let mut col = 0usize;
    let mut page = raw_start;
    'outer: while page < pages.len() {
        let p = &pages[page];
        let mut i = 0usize;
        while i < PAGE_INTS {
            let key = p[i];
            if key == -1 {
                break 'outer;
            }
            if key == 0 {
                break; // 本页结束 → 下一页
            }
            if key < KEY_MIN || key > KEY_MAX {
                break; // 异常，停本页
            }
            if i + 2 >= PAGE_INTS {
                break; // 记录头跨页 → 下一页（页尾以 0 填充，记录不跨页）
            }
            let kind = p[i + 1]; // type (v19)：1/3/4
            let has_def = p[i + 2]; // v20：1=无默认 / 2=有默认
            let mut j = i + 3;
            let mut default = 0i32;
            if has_def == 2 {
                if kind == 4 {
                    // 数组默认：len + len 个元素
                    if j >= PAGE_INTS {
                        break;
                    }
                    let len = p[j].max(0) as usize;
                    default = p[j]; // 存长度作占位（数组默认对分类 bool/int 无关）
                    j += 1 + len;
                } else {
                    if j >= PAGE_INTS {
                        break;
                    }
                    default = p[j];
                    j += 1;
                }
            }
            col += 1; // 1-based
            map.insert(
                key,
                FieldSlot {
                    col,
                    ty: FieldType::from_code(kind),
                    default,
                },
            );
            i = j;
        }
        page += 1;
    }
    map
}

/// hash → noun 名（best-effort，用 aios_core 的 db1 反哈希）。
fn dehash_noun(noun_hash: i32) -> String {
    aios_core::tool::db_tool::db1_dehash(noun_hash as u32).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_type_from_code() {
        assert_eq!(FieldType::from_code(1), FieldType::Bool);
        assert_eq!(FieldType::from_code(3), FieldType::Int);
        assert_eq!(FieldType::from_code(4), FieldType::Array);
        assert!(matches!(FieldType::from_code(9), FieldType::Other(9)));
    }

    #[test]
    fn split_pages_slices_by_0x800() {
        let b = vec![0u8; PAGE_BYTES * 3 + 7];
        assert_eq!(split_pages(&b).len(), 3);
    }

    #[test]
    fn logical_to_raw_is_minus_one() {
        assert_eq!(logical_to_raw(2168), 2167);
        assert_eq!(logical_to_raw(2830), 2829);
        assert_eq!(logical_to_raw(2), 1);
        assert_eq!(logical_to_raw(0), 0);
    }

    /// 辅助（非断言）：打印已知 noun 的 db1_hash。
    /// `cargo test --lib dict::tests::print_known_noun_hashes -- --nocapture`
    #[test]
    fn print_known_noun_hashes() {
        use aios_core::tool::db_tool::db1_hash;
        for n in [
            "SCYL", "SBOX", "CYLI", "SPHE", "SCOM", "GMSET", "SITE", "ZONE", "EQUI",
        ] {
            let h = db1_hash(n);
            println!("db1_hash({:6}) = {:>12} (0x{:08X})", n, h as i32, h);
        }
    }

    /// 集成（默认 ignore，需本机 E3D 字典）：对真实 attlib.dat 读出已知 noun 的分类 flag，
    /// 并打印诊断（header / primitive 列 / SCYL 记录窗口）以校准 -2 对齐。
    /// `cargo test --lib dict::tests::integration_read_attlib -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat"]
    fn integration_read_attlib() {
        use aios_core::tool::db_tool::db1_hash;
        let path = std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat");
        let df = AttrDataFile::open(path).expect("open attlib.dat");
        println!("header v47 = {:?}", df.header());
        println!(
            "noun_count={} field_count={}",
            df.noun_count(),
            df.field_count()
        );
        for (fid, name) in [
            (FIELD_PRIMITIVE, "primitive"),
            (FIELD_GEOMSET, "geomset"),
            (FIELD_EXTRUSION, "extrusion"),
            (FIELD_GRAPHICS_BEHAVIOUR, "graphicsBehaviour"),
            (FIELD_BASE_TYPE, "base_type"),
        ] {
            if let Some(s) = df.field_index.get(&fid) {
                println!(
                    "field {name}({fid}) col={} type={:?} default={}",
                    s.col, s.ty, s.default
                );
            } else {
                println!("field {name}({fid}) NOT FOUND in field index");
            }
        }
        // 设计图元(应 primitive=true) / 目录(geomset) / 容器(primitive=false) 混合抽查。
        for n in [
            "BOX", "CYLI", "SPHE", "CONE", "DISH", "CTOR", "PYRA", "SNOU", // 设计图元
            "SCYL", "SBOX", "SCOM", "GMSET", // 目录/几何集
            "PANE", "EXTR", // 挤出
            "SITE", "ZONE", "WORL", "EQUI", "PIPE", "BRAN", // 容器/设计
        ] {
            let h = db1_hash(n) as i32;
            if let Some(ns) = df.noun_index.get(&h) {
                // 诊断：打印 primitive 字段的 step1 off（帮助确认两级取值）。
                let off_dbg = df
                    .field_index
                    .get(&FIELD_PRIMITIVE)
                    .and_then(|fs| df.slot_at(ns, fs.col));
                let f = df.noun_flags(h);
                println!(
                    "{n:5}({h:>10}) @raw{}:{:<3} prim={} geomset={} extr={} gb={:?} [prim.off={:?}]",
                    ns.raw_page,
                    ns.row_base,
                    f.primitive,
                    f.geomset,
                    f.extrusion,
                    f.graphics_behaviour,
                    off_dbg
                );
            } else {
                println!("{n:5}({h:>10}) NOT FOUND in noun index");
            }
        }
    }

    /// 严谨交叉核对：用 `aios_core::pdms_types` 的 curated noun 名单校验 dict 分类。
    /// 设计图元名单 → 应 `primitive=true`；元件库几何名单 → 应 `geomset=true`；
    /// 挤出 owner 名单 → 应 `extrusion=true`。打印每组「命中/总数 + 一致率 + 不符项」，
    /// 并对最高置信的 `PRIMITIVE_NOUN_NAMES`（8 个经典设计图元）硬断言全部 primitive=true。
    /// `cargo test --lib dict::tests::crosscheck_curated_noun_lists -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat"]
    fn crosscheck_curated_noun_lists() {
        use aios_core::pdms_types::{
            GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES, PRIMITIVE_NOUN_NAMES,
            TOTAL_CATA_GEO_NOUN_NAMES,
        };
        use aios_core::tool::db_tool::db1_hash;

        let path = std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat");
        let df = AttrDataFile::open(path).expect("open attlib.dat");

        // (标签, 名单, 期望谓词, 期望描述)
        let groups: [(&str, &[&str], fn(&NounFlags) -> bool, &str); 4] = [
            (
                "PRIMITIVE(设计图元)",
                &PRIMITIVE_NOUN_NAMES,
                |f| f.primitive,
                "primitive=true",
            ),
            (
                "GNERAL_PRIM(图元+负体)",
                &GNERAL_PRIM_NOUN_NAMES,
                |f| f.primitive,
                "primitive=true",
            ),
            (
                "CATA_GEO(元件库几何)",
                &TOTAL_CATA_GEO_NOUN_NAMES,
                |f| f.geomset,
                "geomset=true",
            ),
            (
                "LOOP_OWNER(挤出)",
                &GNERAL_LOOP_OWNER_NOUN_NAMES,
                |f| f.extrusion,
                "extrusion=true",
            ),
        ];

        for (label, names, pred, want) in groups {
            let mut found = 0usize;
            let mut ok = 0usize;
            let mut missing: Vec<&str> = Vec::new();
            let mut mism: Vec<String> = Vec::new();
            for n in names {
                let h = db1_hash(n) as i32;
                if !df.has_noun(h) {
                    missing.push(n);
                    continue;
                }
                found += 1;
                let f = df.noun_flags(h);
                if pred(&f) {
                    ok += 1;
                } else {
                    mism.push(format!(
                        "{n}(prim={},geomset={},extr={})",
                        f.primitive, f.geomset, f.extrusion
                    ));
                }
            }
            let rate = if found > 0 {
                100.0 * ok as f64 / found as f64
            } else {
                0.0
            };
            println!(
                "[{label}] 期望 {want}: 命中 {found}/{} 一致 {ok}/{found} ({rate:.0}%)",
                names.len()
            );
            if !missing.is_empty() {
                println!("    未在字典找到: {:?}", missing);
            }
            if !mism.is_empty() {
                println!("    不符期望: {:?}", mism);
            }
        }

        // 最高置信硬断言：8 个经典设计图元凡在字典命中者必 primitive=true。
        for n in PRIMITIVE_NOUN_NAMES {
            let h = db1_hash(n) as i32;
            if df.has_noun(h) {
                assert!(df.noun_flags(h).primitive, "设计图元 {n} 应 primitive=true");
            }
        }
    }

    /// 导出交付物 `noun_flags.json`（全部 noun 的分类 flag）到 gen-model 仓库根，
    /// 与 `all_attr_info.json` 同级，供将来 `NounClassifier` 加载（ADR-004 阶段 3）。
    /// `cargo test --lib dict::tests::export_noun_flags_json -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat；产出仓库根 noun_flags.json"]
    fn export_noun_flags_json() {
        let attr = std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat");
        // vendor/aios-parse-pdms/src/dict.rs → 上溯 3 层到 gen-model 根。
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("noun_flags.json"))
            .expect("resolve repo root");
        export_noun_flags(attr, &out).expect("export noun_flags.json");
        let df = AttrDataFile::open(attr).unwrap();
        println!("exported {} nouns → {}", df.noun_count(), out.display());
    }

    /// `NounClassifier` 分歧图：把 dict flag 与 gen-model 的**生成路由名单**逐一对比，
    /// 明确「盲替会错路由」的风险集合（尤其管件），指导阶段 3 逐个迁移。
    /// `cargo test --lib dict::tests::classifier_divergence_map -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat"]
    fn classifier_divergence_map() {
        use aios_core::pdms_types::{
            GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES, PIPING_NOUN_NAMES,
            TOTAL_CATA_GEO_NOUN_NAMES, USE_CATE_NOUN_NAMES,
        };
        use std::collections::HashSet;

        let path = std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat");
        let clf = NounClassifier::from_attr_file(path).expect("build classifier");
        let up = |s: &str| s.trim().to_ascii_uppercase();
        let set = |xs: &[&str]| -> HashSet<String> { xs.iter().map(|s| up(s)).collect() };

        let piping = set(&PIPING_NOUN_NAMES);
        let use_cate = set(&USE_CATE_NOUN_NAMES);

        println!(
            "=== NounClassifier 分歧图 === 字典 noun={} | primitive={} geomset={} extrusion={}",
            clf.len(),
            clf.primitive_nouns().len(),
            clf.geomset_nouns().len(),
            clf.extrusion_nouns().len()
        );

        // 通用：list vs flag 双向差集 + 对 extras 按 PIPING/USE_CATE 归桶。
        let report = |label: &str, list: &HashSet<String>, flag_nouns: Vec<String>| {
            let flag: HashSet<String> = flag_nouns.into_iter().collect();
            let mut in_list_no_flag: Vec<&String> = list.difference(&flag).collect();
            in_list_no_flag.sort();
            let mut extras: Vec<&String> = flag.difference(list).collect();
            extras.sort();
            let extra_piping: Vec<&&String> =
                extras.iter().filter(|n| piping.contains(**n)).collect();
            let extra_cate: Vec<&&String> = extras
                .iter()
                .filter(|n| use_cate.contains(**n) && !piping.contains(**n))
                .collect();
            let extra_other: Vec<&&String> = extras
                .iter()
                .filter(|n| !piping.contains(**n) && !use_cate.contains(**n))
                .collect();
            println!("\n[{label}] 名单 {} 个", list.len());
            println!(
                "  名单内但 flag=false（盲替会漏路由）: {:?}",
                in_list_no_flag
            );
            println!("  flag=true 但不在名单 共 {} 个:", extras.len());
            println!(
                "     ├ 属 PIPING（盲替会误路由到该路径）: {:?}",
                extra_piping
            );
            println!("     ├ 属 USE_CATE: {:?}", extra_cate);
            println!(
                "     └ 其它 {} 个（样本≤40）: {:?}",
                extra_other.len(),
                extra_other.iter().take(40).collect::<Vec<_>>()
            );
        };

        report(
            "GNERAL_PRIM→prim_model vs primitive",
            &set(&GNERAL_PRIM_NOUN_NAMES),
            clf.primitive_nouns(),
        );
        report(
            "GNERAL_LOOP_OWNER→loop_model vs extrusion",
            &set(&GNERAL_LOOP_OWNER_NOUN_NAMES),
            clf.extrusion_nouns(),
        );
        report(
            "TOTAL_CATA_GEO vs geomset",
            &set(&TOTAL_CATA_GEO_NOUN_NAMES),
            clf.geomset_nouns(),
        );

        // 管件在各 flag 下的归类（决定管件该走哪条路由）。
        println!("\n[PIPING 各 flag 归类]");
        for n in PIPING_NOUN_NAMES {
            match clf.flags(n) {
                Some(f) => println!(
                    "  {n:5} prim={} geomset={} extr={} gb={:?}",
                    f.primitive, f.geomset, f.extrusion, f.graphics_behaviour
                ),
                None => println!("  {n:5} NOT FOUND"),
            }
        }
    }

    /// 仓库根 `noun_flags.json` 路径（`parse_pdms_db` → 上溯 2 层）。
    fn repo_noun_flags_json() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("noun_flags.json"))
            .expect("resolve repo root")
    }

    /// 守护：gen-model 的**生成路由名单**必须与 dict flag 一致（名单 ⊆ 对应 flag）。
    /// 加载已提交的 `noun_flags.json`（非 ignore、随常规构建运行，防名单漂移 / dict 变更）。
    /// NSBO/NSCY 属元件库负体（geomset），故 GNERAL_PRIM 用"primitive∪geomset"（皆几何）判。
    /// 若 `noun_flags.json` 尚未生成则跳过（软失败，不误伤无字典环境）。
    #[test]
    fn routing_lists_are_dict_validated() {
        use aios_core::pdms_types::{
            GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES, PRIMITIVE_NOUN_NAMES,
            TOTAL_CATA_GEO_NOUN_NAMES,
        };
        let json = repo_noun_flags_json();
        let clf = match NounClassifier::from_json_path(&json) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "skip routing guard: {e}（noun_flags.json 未生成，跑 export_noun_flags_json）"
                );
                return;
            }
        };
        let check = |label: &str, names: &[&str], pred: &dyn Fn(&str) -> bool| {
            let mut viol = Vec::new();
            for n in names {
                if !clf.contains(n) {
                    viol.push(format!("{n}(字典缺失)"));
                } else if !pred(n) {
                    viol.push(format!("{n}(flag=false)"));
                }
            }
            assert!(
                viol.is_empty(),
                "[{label}] 路由名单与 dict flag 不符: {viol:?}"
            );
        };
        check("PRIMITIVE⊆primitive", &PRIMITIVE_NOUN_NAMES, &|n| {
            clf.primitive(n)
        });
        check(
            "GNERAL_PRIM⊆primitive∪geomset",
            &GNERAL_PRIM_NOUN_NAMES,
            &|n| clf.primitive(n) || clf.geomset(n),
        );
        check(
            "GNERAL_LOOP_OWNER⊆extrusion",
            &GNERAL_LOOP_OWNER_NOUN_NAMES,
            &|n| clf.extrusion(n),
        );
        check(
            "TOTAL_CATA_GEO⊆geomset",
            &TOTAL_CATA_GEO_NOUN_NAMES,
            &|n| clf.geomset(n),
        );
    }

    /// Phase 1-B：全局 `default_noun_classifier()` 加载 crate 内嵌 `noun_flags.json`
    /// （非 ignore，随常规构建跑，验证内嵌数据可用 + 分类正确）。
    #[test]
    fn default_classifier_loads_and_spot_checks() {
        let c = default_noun_classifier();
        assert!(c.len() > 1000, "内嵌分类器为空? len={}", c.len());
        assert!(
            c.primitive("BOX") && c.primitive("CYLI"),
            "BOX/CYLI 应 primitive"
        );
        assert!(
            c.geomset("SCYL") && c.geomset("SBOX"),
            "SCYL/SBOX 应 geomset"
        );
        assert!(
            c.primitive("ELBO"),
            "管件 ELBO 也是 primitive(设计级几何叶子)"
        );
        assert!(
            !c.primitive("SITE") && !c.geomset("SITE"),
            "SITE 容器非几何"
        );
        // 负体候选启发式（Phase 1-A 结论）：含真负体候选，也记录 NOZZ 假阳性。
        let negs: std::collections::HashSet<String> =
            c.negative_candidate_nouns().into_iter().collect();
        for n in ["NBOX", "NPOLYH", "NSLC", "NTUB"] {
            assert!(negs.contains(n), "负体候选应含 {n}");
        }
        assert!(
            negs.contains("NOZZ"),
            "NOZZ 被 N-前缀启发式误纳(已知假阳性 → 负体源不能纯用本启发式)"
        );
    }

    /// 守护：运行期覆盖观测与 `docs/plans/stage3-noun-routing-gaps.md` 用同一口径。
    /// 名单或字典任一漂移都会在这里先失败，而不是让运行期告警悄悄换了含义。
    #[test]
    fn uncovered_geometry_nouns_match_the_gap_report_snapshot() {
        let clf = default_noun_classifier();
        if clf.len() == 0 {
            return; // 无内嵌字典的环境软跳过，不误伤
        }
        assert_eq!(routing_coverage_nouns().len(), 122, "路由覆盖并集漂移");
        let uncovered = uncovered_geometry_nouns(clf);
        assert_eq!(uncovered.len(), 291, "dict 几何 − 路由覆盖 的差集漂移");
        for covered in ["BOX", "CYLI", "TUBI", "NOZZ"] {
            assert!(!uncovered.contains(covered), "{covered} 已被路由名单覆盖");
        }
        for gap in ["POINSP", "AIDLIN", "HPLATE", "CTRAY"] {
            assert!(uncovered.contains(gap), "{gap} 应属名单外几何");
        }
    }

    /// RE 负体字段：暴力扫全部 dict 字段，找出"能把负体(NBOX/NCYL…)与正体(BOX/CYLI…)
    /// 干净二分"的字段号（负体全同值 A、正体全同值 B、A≠B）——即 dabacon 的负体标志。
    /// `cargo test --lib dict::tests::find_negative_field -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat"]
    fn find_negative_field() {
        use aios_core::tool::db_tool::db1_hash;
        let df = AttrDataFile::open(std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat"))
            .expect("open attlib.dat");
        // 设计负体 vs 对应正体（都在 noun 索引里）。
        let negs = [
            "NBOX", "NCYL", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NCTO",
        ];
        let poss = [
            "BOX", "CYLI", "SBOX", "CONE", "SNOU", "PYRA", "DISH", "CTOR",
        ];
        let nh: Vec<i32> = negs.iter().map(|n| db1_hash(n) as i32).collect();
        let ph: Vec<i32> = poss.iter().map(|n| db1_hash(n) as i32).collect();
        let mut fids: Vec<i32> = df.field_index.keys().copied().collect();
        fids.sort_unstable();
        println!(
            "扫描 {} 个字段，负体{}个/正体{}个",
            fids.len(),
            nh.len(),
            ph.len()
        );
        let mut hits = 0;
        for fid in fids {
            let nv: Vec<Option<i32>> = nh.iter().map(|&h| df.raw_field(h, fid)).collect();
            let pv: Vec<Option<i32>> = ph.iter().map(|&h| df.raw_field(h, fid)).collect();
            let neg_uniform = nv.iter().all(|x| *x == nv[0]);
            let pos_uniform = pv.iter().all(|x| *x == pv[0]);
            if neg_uniform && pos_uniform && nv[0] != pv[0] {
                let ty = df.field_index.get(&fid).map(|f| f.ty);
                println!(
                    "★ 二分字段 {fid} type={ty:?}: 负体={:?} 正体={:?}",
                    nv[0], pv[0]
                );
                hits += 1;
            }
        }
        println!("共 {hits} 个候选二分字段");
    }

    /// 产出「gen-model 漏路由的几何 noun」缺口清单 → `docs/plans/stage3-noun-routing-gaps.md`。
    /// 供人工审：哪些是真几何(应补生成) vs 构造辅助(AID*/A*)本就不生成。
    /// `cargo test --lib dict::tests::export_stage3_gap_report -- --ignored --nocapture`
    #[test]
    #[ignore = "生成 docs/plans/stage3-noun-routing-gaps.md（需 noun_flags.json）"]
    fn export_stage3_gap_report() {
        use aios_core::pdms_types::{
            GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES, PIPING_NOUN_NAMES,
            TOTAL_CATA_GEO_NOUN_NAMES, USE_CATE_NOUN_NAMES,
        };
        use std::collections::BTreeSet;

        let clf = NounClassifier::from_json_path(&repo_noun_flags_json())
            .or_else(|_| {
                NounClassifier::from_attr_file(std::path::Path::new(
                    r"D:\AVEVA\Everything3D3.1\attlib.dat",
                ))
            })
            .expect("build classifier (noun_flags.json 或 attlib.dat)");

        let up = |s: &str| s.trim().to_ascii_uppercase();
        let set = |xs: &[&str]| -> BTreeSet<String> { xs.iter().map(|s| up(s)).collect() };
        let piping = set(&PIPING_NOUN_NAMES);
        let use_cate = set(&USE_CATE_NOUN_NAMES);
        let prim_list = set(&GNERAL_PRIM_NOUN_NAMES);

        let prim: BTreeSet<String> = clf.primitive_nouns().into_iter().collect();
        let geoms: BTreeSet<String> = clf.geomset_nouns().into_iter().collect();
        let extr: BTreeSet<String> = clf.extrusion_nouns().into_iter().collect();

        let fmt =
            |xs: &BTreeSet<String>| -> String { xs.iter().cloned().collect::<Vec<_>>().join(", ") };
        let diff = |a: &BTreeSet<String>, b: &BTreeSet<String>| -> BTreeSet<String> {
            a.difference(b).cloned().collect()
        };

        // primitive=true 但不在 GNERAL_PRIM：按 PIPING / USE_CATE / 其它归桶。
        let prim_extra = diff(&prim, &prim_list);
        let prim_piping: BTreeSet<String> = prim_extra.intersection(&piping).cloned().collect();
        let prim_cate: BTreeSet<String> = prim_extra
            .iter()
            .filter(|n| use_cate.contains(*n) && !piping.contains(*n))
            .cloned()
            .collect();
        let prim_other: BTreeSet<String> = prim_extra
            .iter()
            .filter(|n| !piping.contains(*n) && !use_cate.contains(*n))
            .cloned()
            .collect();
        let extr_extra = diff(&extr, &set(&GNERAL_LOOP_OWNER_NOUN_NAMES));
        let geom_extra = diff(&geoms, &set(&TOTAL_CATA_GEO_NOUN_NAMES));

        // gen-model 全部几何路由/覆盖名单的并集（近似"任一路径能处理"的 noun 集）。
        let coverage = routing_coverage_nouns();
        // dict 认定"几何" = primitive ∪ geomset ∪ extrusion。
        let mut dict_geo: BTreeSet<String> = BTreeSet::new();
        dict_geo.extend(prim.iter().cloned());
        dict_geo.extend(geoms.iter().cloned());
        dict_geo.extend(extr.iter().cloned());
        let uncovered = uncovered_geometry_nouns(&clf);
        // 前缀直方图（辅助人工判：AID*/FE* 多为不渲染；A*/H*/C* 多为真几何）。
        let mut prefix_hist: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for n in &uncovered {
            let p: String = n.chars().take(2).collect();
            *prefix_hist.entry(p).or_default() += 1;
        }
        let hist_str = prefix_hist
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join("  ");
        let uncovered_section = format!(
            "\n## 4. 真·未被任何 gen-model 路由/覆盖名单收录的 dict 几何 noun\n\n\
             > 覆盖并集 = PRIMITIVE/GNERAL_PRIM/LOOP_OWNER/USE_CATE/PIPING/NEG/VERT/LOOP/POS/TOTAL_GEO/\n\
             > TOTAL_CATA_GEO/CATA_GEO_NAMES/CATA_WITHOUT_REUSE/VISBILE_GEO_NOUNS/NGMR/POHE（共 {} noun）。\n\
             > dict 几何(primitive∪geomset∪extrusion)={}；**未覆盖={}**（仍非全是漏 bug：AID*/FE* 等本不渲染）。\n\
             > ⚠️ **方法学限制(勿当 bug 列表)**：gen-model 几何是**层级式**生成——顶层只查根(BRAN/HANG/EQUI/prim owner/USE_CATE)，其子孙\n\
             > (管件/吊架件/暖通 HV*/桥架 CT*…)在根子树深度展开时经 catalogue(SPRE→GMSET→PARA)生成，不需进任何顶层名单；\n\
             > 故\"不在覆盖名单\"≠\"不渲染\"，本节是**上界**、高估真实缺口。真·漏几何须走**动态**验证(对真实项目跑生成、比对 dict 认几何却未产出 mesh 的 noun)。\n\n\
             前缀直方图：{}\n\n{}\n",
            coverage.len(),
            dict_geo.len(),
            uncovered.len(),
            hist_str,
            fmt(&uncovered),
        );

        let mut md = format!(
            "# 阶段 3 · noun 路由缺口清单（dict flag 认定几何、但 gen-model 路由名单未覆盖）\n\n\
             > 自动产出：`parse_pdms_db::dict::tests::export_stage3_gap_report`（数据源 `noun_flags.json`）。\n\
             > 关联：`docs/plans/db-noun-classifier.md`（阶段 3）、`teach/learning-records/0004`。\n\
             > 用途：**人工审**——判断每个 noun 是真几何(应补生成路由) vs 构造辅助(AID*/A* 关联/占位，本就不生成)。**本清单不改任何行为。**\n\n\
             字典规模：noun={} | primitive={} geomset={} extrusion={}\n\n\
             ---\n\n\
             ## 1. `extrusion=true` 但不在 `GNERAL_LOOP_OWNER_NOUN_NAMES`（loop_model 路由）—— 共 {} 个\n\n{}\n\n\
             ## 2. `geomset=true` 但不在 `TOTAL_CATA_GEO_NOUN_NAMES` —— 共 {} 个\n\n{}\n\n\
             ## 3. `primitive=true` 但不在 `GNERAL_PRIM_NOUN_NAMES`（prim_model 路由）\n\n\
             ### 3a. 属 PIPING（现走 piping/cata 路径，**不应**进 prim_model）—— {} 个\n\n{}\n\n\
             ### 3b. 属 USE_CATE（现走 cata_model）—— {} 个\n\n{}\n\n\
             ### 3c. 其它（**需人工判**：真几何 vs 构造辅助）—— {} 个\n\n{}\n",
            clf.len(),
            prim.len(),
            geoms.len(),
            extr.len(),
            extr_extra.len(),
            fmt(&extr_extra),
            geom_extra.len(),
            fmt(&geom_extra),
            prim_piping.len(),
            fmt(&prim_piping),
            prim_cate.len(),
            fmt(&prim_cate),
            prim_other.len(),
            fmt(&prim_other),
        );
        md.push_str(&uncovered_section);

        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| {
                p.join("docs")
                    .join("plans")
                    .join("stage3-noun-routing-gaps.md")
            })
            .expect("resolve out path");
        std::fs::write(&out, md.into_bytes()).expect("write gap report");
        println!(
            "gap report → {}\n  extrusion 缺口={} geomset 缺口={} primitive其它缺口={} | 真未覆盖={}",
            out.display(),
            extr_extra.len(),
            geom_extra.len(),
            prim_other.len(),
            uncovered.len()
        );
    }

    // ── 批次 A：noun 能力矩阵（v2 测试计划 §4 批次 A）──────────────────────
    //
    // 数据源是 crate 内嵌的 `noun_caps.json`（由 `regenerate_noun_caps_snapshot`
    // 从 `attlib.dat` 导出），因此 A-DICT-02…06 无需本机 E3D 即可跑。

    /// 重新生成内嵌快照。改了字段集合后手动跑一次，再提交 `noun_caps.json`。
    /// `cargo test -p parse_pdms_db regenerate_noun_caps_snapshot -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat；导出后需提交 noun_caps.json"]
    fn regenerate_noun_caps_snapshot() {
        let attlib = std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat");
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("noun_caps.json");
        export_noun_capabilities(attlib, &out).expect("export noun_caps.json");
        let table =
            NounCapabilityTable::from_json_str(&std::fs::read_to_string(&out).expect("read back"))
                .expect("parse exported");
        println!("noun_caps.json → {} ({} noun)", out.display(), table.len());
    }

    /// A-DICT-01：31 个能力字段里，除 `primaryList` 外全部能从真实字典读出；
    /// 且 Rust 读出的结果与内嵌快照逐条一致。
    /// `cargo test -p parse_pdms_db a_dict_01 -- --ignored --nocapture`
    #[test]
    #[ignore = "需本机 D:/AVEVA/Everything3D3.1/attlib.dat"]
    fn a_dict_01_all_capability_fields_readable_and_snapshot_matches() {
        let df = AttrDataFile::open(std::path::Path::new(r"D:\AVEVA\Everything3D3.1\attlib.dat"))
            .expect("open attlib.dat");

        let in_dict = [
            ("primitive", FIELD_PRIMITIVE),
            ("geomset", FIELD_GEOMSET),
            ("extrusion", FIELD_EXTRUSION),
            ("isPointsetPoint", FIELD_ISPOINTSETPOINT),
            ("graphicsBehaviour", FIELD_GRAPHICS_BEHAVIOUR),
            ("point", FIELD_POINT),
            ("positiveEquivalent", FIELD_POSITIVE_EQUIVALENT),
            ("changeType", FIELD_CHANGE_TYPE),
            ("spatialMap", FIELD_SPATIAL_MAP),
            ("secondaryHierarchy", FIELD_SECONDARY_HIERARCHY),
            ("defined", FIELD_DEFINED),
            ("validc", FIELD_VALIDC),
            ("visible", FIELD_VISIBLE),
            ("toplevel", FIELD_TOPLEVEL),
            ("pickable", FIELD_PICKABLE),
            ("world", FIELD_WORLD),
            ("defaultVolumeQuery", FIELD_DEFAULT_VOLUME_QUERY),
            ("clasherWithin", FIELD_CLASHER_WITHIN),
            ("clasherSection", FIELD_CLASHER_SECTION),
            ("modifiable", FIELD_MODIFIABLE),
            ("statusEligible", FIELD_STATUS_ELIGIBLE),
            ("isCloneable", FIELD_IS_CLONEABLE),
            ("requiresMarine", FIELD_REQUIRES_MARINE),
            ("isProtected", FIELD_IS_PROTECTED),
            ("deleteMemberOnCopy", FIELD_DELETE_MEMBER_ON_COPY),
            ("spoolerModifiable", FIELD_SPOOLER_MODIFIABLE),
            ("psOwner", FIELD_PS_OWNER),
            ("psNext", FIELD_PS_NEXT),
            ("psFirstMember", FIELD_PS_FIRST_MEMBER),
        ];
        for (name, fid) in in_dict {
            assert!(
                df.field_index.contains_key(&fid),
                "字段 {name}({fid}) 应存在于 dabacon FIELD 表"
            );
        }
        // primaryList 走 db_get_element_info，不该出现在字典里——若哪天出现了，
        // 说明 ReadDataDab 的结论要重审。
        assert!(
            !df.field_index.contains_key(&FIELD_PRIMARY_LIST_NOT_IN_DICT),
            "primaryList 不应出现在 dabacon 字典（core.dll 用 db_get_element_info 读）"
        );

        let live = df.all_noun_capabilities();
        let snapshot = default_noun_capabilities();
        assert_eq!(live.len(), snapshot.len(), "快照与实时字典的 noun 数不一致");
        for c in &live {
            let s = snapshot
                .by_hash(c.noun_hash)
                .unwrap_or_else(|| panic!("快照缺 noun_hash={}", c.noun_hash));
            assert_eq!(s, c, "noun {} 的能力与快照不一致", c.noun_name);
        }
    }

    /// A-DICT-02：能力计数快照不漂移（数值来自 E3D 3.1 的 attlib.dat）。
    #[test]
    fn a_dict_02_capability_counts_do_not_drift() {
        let t = default_noun_capabilities();
        let n = |p: fn(&NounCapabilities) -> bool| t.iter().filter(|c| p(c)).count();

        assert_eq!(t.len(), 1931, "noun 总数");
        assert_eq!(n(|c| c.primitive), 347, "primitive");
        assert_eq!(n(|c| c.geomset), 44, "geomset");
        assert_eq!(n(|c| c.extrusion), 38, "extrusion");
        assert_eq!(t.direct_geometry_nouns().len(), 395, "直接几何并集");
        assert_eq!(n(|c| c.point), 44, "point");
        assert_eq!(n(|c| c.is_pointset_point), 4, "isPointsetPoint");
        assert_eq!(t.positive_equivalents().len(), 12, "positiveEquivalent");
        assert_eq!(
            n(|c| c.change_type.is_some_and(|v| v != 0)),
            117,
            "changeType 非零"
        );
        assert_eq!(t.pseudo_nouns().len(), 10, "isPseudo");
        assert_eq!(
            n(|c| c.graphics_behaviour.is_some_and(|v| v != 0)),
            279,
            "graphicsBehaviour 非零"
        );
    }

    /// A-DICT-03：负体 → 正体映射精确等于字典登记的 12 对。
    ///
    /// 这 12 对是布尔减法的权威来源：几何 noun 里叫 `N*` 的远不止 12 个，
    /// 按名字前缀推断会既多算又少算。
    #[test]
    fn a_dict_03_positive_equivalent_pairs_are_exact() {
        const EXPECTED: &[(&str, &str)] = &[
            ("NBOX", "BOX"),
            ("NCON", "CONE"),
            ("NCTO", "CTOR"),
            ("NCYL", "CYLI"),
            ("NDIS", "DISH"),
            ("NPOLYH", "POLYHE"),
            ("NPYR", "PYRA"),
            ("NREV", "EXTR"),
            ("NRTO", "RTOR"),
            ("NSLC", "SLCY"),
            ("NSNO", "SNOU"),
            ("NXTR", "EXTR"),
        ];
        let actual = default_noun_capabilities().positive_equivalents();
        let expected: Vec<(String, String)> = EXPECTED
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        assert_eq!(actual, expected);
    }

    /// A-DICT-04：变化等价类精确等于 25 类；4 个抽象类由 db1 反哈希还原。
    #[test]
    fn a_dict_04_change_classes_are_exact() {
        let classes = default_noun_capabilities().change_classes();
        assert_eq!(classes.len(), 25, "变化等价类数量");

        // 抽象类（目标 hash 不在 noun 索引里，只能反哈希得名）。
        for abstract_class in ["LINEAR", "MULTC", "INLINE", "PCONN"] {
            assert!(
                classes.contains_key(abstract_class),
                "缺抽象变化类 {abstract_class}"
            );
        }

        // 结构专业的等价类：同类成员必须走同一条变化处理路径。
        assert_eq!(
            classes.get("PANE").map(Vec::as_slice),
            Some(["FLOOR", "GWALL", "SCREED"].map(String::from).as_slice())
        );
        assert_eq!(
            classes.get("GENSEC").map(Vec::as_slice),
            Some(["GENSEC", "WALL"].map(String::from).as_slice())
        );
        assert_eq!(
            classes.get("SCTN").map(Vec::as_slice),
            Some(["SCTN", "STWALL"].map(String::from).as_slice())
        );
        assert_eq!(
            classes.get("SUPPO").map(Vec::as_slice),
            Some(["REST", "SUPPO"].map(String::from).as_slice())
        );
        assert_eq!(
            classes.get("LINEAR").map(Vec::as_slice),
            Some(
                [
                    "DUCT", "FTUB", "OFST", "PLAT", "PLEN", "REDU", "STRT", "TAPE", "TRNS",
                    "TRREDU",
                ]
                .map(String::from)
                .as_slice()
            )
        );

        // 单点查询与整表一致。
        assert_eq!(
            default_noun_capabilities().change_class_name("FLOOR"),
            Some("PANE".to_string())
        );
        assert_eq!(
            default_noun_capabilities().change_class_name("VALV"),
            Some("INLINE".to_string())
        );
        // change_type 为 0 的 noun 自成一类。
        assert_eq!(
            default_noun_capabilities().change_class_name("EQUI"),
            Some("EQUI".to_string())
        );
    }

    /// A-DICT-05：手写的 loop/vert 容器名单必须是字典 `point==true`(44) 的子集。
    ///
    /// 超出部分说明手写名单收了字典不认为是点容器的类型，`is_loop_container_noun`
    /// 会把本该当生成根的元素误判为容器。反过来，字典里多出的 40 个 point noun
    /// 是**潜在漏判**——本测试打印出来供人工审，不断言（补进名单前需逐个确认它们
    /// 确实不该当生成根）。
    #[test]
    fn a_dict_05_curated_loop_lists_subset_of_dict_point() {
        use aios_core::pdms_types::{TOTAL_LOOP_NOUN_NAMES, TOTAL_VERT_NOUN_NAMES};

        let point = default_noun_capabilities().point_nouns();
        assert_eq!(point.len(), 44, "point noun 数");

        let curated: Vec<String> = TOTAL_LOOP_NOUN_NAMES
            .iter()
            .chain(TOTAL_VERT_NOUN_NAMES.iter())
            .map(|n| n.trim().to_ascii_uppercase())
            .collect();
        let extra: Vec<&String> = curated.iter().filter(|n| !point.contains(n)).collect();
        assert!(
            extra.is_empty(),
            "手写 loop/vert 名单超出字典 point 集合：{extra:?}\n字典 point={point:?}"
        );

        let uncovered: Vec<&String> = point.iter().filter(|n| !curated.contains(n)).collect();
        println!(
            "curated loop/vert = {curated:?}\n字典 point 未收录 {} 个（待人工审）：{uncovered:?}",
            uncovered.len()
        );
    }

    /// A-DICT-06：整型能力字段的值域封闭——出现新值说明字典换版，需重新解读语义。
    #[test]
    fn a_dict_06_integer_capability_domains_are_closed() {
        let t = default_noun_capabilities();
        let domain = |get: fn(&NounCapabilities) -> Option<i32>| {
            let mut v: Vec<i32> = t.iter().map(|c| get(c).unwrap_or(0)).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        assert_eq!(domain(|c| c.graphics_behaviour), vec![0, 1, 2, 3]);
        assert_eq!(domain(|c| c.spatial_map), vec![0, 1, 2, 3, 5, 7]);
        assert_eq!(domain(|c| c.secondary_hierarchy), vec![0, 1, 2]);
        assert_eq!(domain(|c| c.defined), vec![1, 4]);
        assert_eq!(domain(|c| c.validc), vec![0, 1, 2]);
    }
}
