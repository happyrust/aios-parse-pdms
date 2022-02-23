use std::collections::{BTreeMap, HashMap};
use smol_str::SmolStr;
use crate::helper::get_attr_strings_db;
use crate::interface::pdms_interface_new::{MResult, PdmsMongoService};
use crate::pdms_data::{AxisParam, GmseParam};
use crate::pdms_types::AttrMap;

///查询gmse的参数
pub async fn query_gmse_params(attr_map: &AttrMap, interface: &mut PdmsMongoService, file_name: SmolStr) -> MResult<Vec<GmseParam>> {
    let mut gmses = vec![];
    let refno = attr_map.get_refno_as_string();
    if let Some(children) = interface.get_children_attr_map(refno, file_name).await? {
        for child in children {
            gmses.push(query_gmse_param(&child.attr).await);
        }
    }
    Ok(gmses)
}

/// 获得gmse的params
pub async fn query_gmse_param(attr_map: &AttrMap) -> GmseParam {
    let mut paxises = get_attr_strings_db(attr_map, &["PAXI", "PAAX", "PBAX", "PCAX"]);
    if let Some(val) = attr_map.get_as_string("PTS") {
        paxises.push(val);
    }
    let centre_line_flag = attr_map.get_bool("CLFL");
    let tube_flag = attr_map.get_bool("TUFL");
    GmseParam {
        attr_map: attr_map.clone(),
        radius: attr_map.get_as_string("PRAD").unwrap_or_default().into(),
        diameters: get_attr_strings_db(attr_map, &["PDIA", "PBDM", "PTDM", "DIAM"]),
        distances: get_attr_strings_db(attr_map, &["PDIS", "PBDI", "PTDI"]),
        height: attr_map.get_as_string("PHEI").unwrap_or_default(),
        offset: attr_map.get_as_string("POFF").unwrap_or_default(),
        // box_lengths: get_attr_strings_db(ele_map, &["PXEL", "PYEL", "PZEL"]),
        box_lengths: get_attr_strings_db(attr_map, &["PXLE", "PYLE", "PZLE"]),
        xyz: get_attr_strings_db(
            attr_map,
            &[
                "PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF",
            ],
        ),
        paxises, // 先pa_axis, 后pb_axis
        centre_line_flag,
        tube_flag,
    }
}

///获得dtse的参数信息
pub async fn query_dtse_params(attr_map: &AttrMap, interface: &mut PdmsMongoService, file_name: SmolStr, context: &mut HashMap<SmolStr, SmolStr>) -> MResult<()> {
    let dtre_refno = attr_map.get_as_string("DTRE").unwrap_or_default();
    if let Some(children) = interface.get_children_attr_map(file_name, dtre_refno).await? {
        for child in children {
            let key = child.attr.get_as_string("DKEY").unwrap_or_default();
            let exp = child.attr.get_as_string("PPRO").unwrap_or_default();
            let default_key = format!("{}_default_expr", key);
            let default_expr = child.attr.get_as_string("DPRO").unwrap_or_default();
            context.insert(key, exp);
            context.insert(default_key.into(), default_expr);
        }
    }
    Ok(())
}