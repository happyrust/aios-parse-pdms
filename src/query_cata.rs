use crate::db_tool::db1_dehash;
use crate::helper::*;
use crate::pdms_data::{AxisParam, GmParam, ScomInfo};
use crate::parsed_data::geo_params_data::CateGeoParam::TubeImplied;
use crate::parsed_data::{CateTubeImpliedParam, GmseParamData, GeomsInfo};
use crate::pdms_types::AttrVal::IntArrayType;
use crate::pdms_types::{AttrVal, PdmsRefno, RefU64};
use crate::AttrMap;
use dashmap::DashMap;
use std::collections::{BTreeMap, HashMap};
use anyhow::anyhow;
use log::{error, info};
use smol_str::SmolStr;
use crate::data_interface::PdmsDataInterface;

pub const DDHEIGHT_STR: &'static str = "DDHEIGHT";
pub const DDRADIUS_STR: &'static str = "DDRADIUS";
pub const DDANGLE_STR: &'static str = "DDANGLE";


///求解design component
pub fn resolve_desi_comp<T: PdmsDataInterface>(
    refno: RefU64,
    interface: &T,
) -> anyhow::Result<GeomsInfo> {
    let attr_map = interface.get_ele_attr(refno)?;
    let mut scom_ref = None;
    if let Ok(catref) = attr_map.get_foreign_refno("CATR") {
        scom_ref = Some(catref);
    } else {
        let spre_ref = attr_map.get_foreign_refno("SPRE")?;
        let spre = interface.get_ele_attr(spre_ref)?;
        if spre.contains_attr_name("CATR") {
            scom_ref = Some(spre.get_foreign_refno("CATR")?);
        }
    };
    let scom_ref = scom_ref.ok_or(anyhow!("SCOM ref not exist".to_string()))?;
    let scom_info = query_scom_info(scom_ref, interface)?;
    let mut context: HashMap<SmolStr, SmolStr> = HashMap::new();

    let mut desp = attr_map.get_f64_vec("DESP")?;
    for i in 0..desp.len() {
        context.insert(
            format!("DESP{}", i + 1).into(),
            desp[i].to_string().into(),
        );
    }
    let mut desp = attr_map.get_f64_vec("DESI")?;
    for i in 0..desp.len() {
        context.insert(
            format!("DESI{}", i + 1).into(),
            desp[i].to_string().into(),
        );
    }
    // let mut desp = attr_map.get_f64_vec("OPAR")?;
    // for i in 0..desp.len() {
    //     context.insert(
    //         format!("OPAR{}", i + 1).into(),
    //         desp[i].to_string().into(),
    //     );
    // }


    let height = attr_map.get_as_string("HEIG").unwrap_or("0.0".into());
    context.insert(DDHEIGHT_STR.into(), height.clone());
    context.insert("HEIG".into(), height);

    let angle = attr_map.get_as_string("ANGL").unwrap_or("0.0".into());
    context.insert(DDANGLE_STR.into(), angle.clone());
    context.insert("ANGL".into(), angle);

    let radi = attr_map.get_as_string("RADI").unwrap_or("0.0".into());
    context.insert(DDRADIUS_STR.into(), radi.clone());
    context.insert("RADI".into(), radi);

    //dbg!(&context);
    let mut geom_info = resolve_cata_comp(&scom_info, interface, Some(context));
    if geom_info.is_err() {
        error!("{:?}",geom_info.as_ref().err());
        error!("{:?}",attr_map.to_string_hashmap());
    }
    geom_info
}


///整合SCOM对应的临时数据
pub fn query_scom_info<T: PdmsDataInterface>(
    refno: RefU64,
    interface: &T,
) -> anyhow::Result<ScomInfo> {
    let attr_map = interface.get_ele_attr(refno)?;
    let type_noun = attr_map.get_type_cloned();
    let is_sprf = type_noun == "SPRF";
    let ptref_name = if is_sprf { "PSTR" } else { "PTRE" };
    let ptre_refno = attr_map.get_foreign_refno(ptref_name)?;
    let mut axis_params = vec![];
    let mut axis_param_numbers = vec![];
    if let Ok(ptre_am) = interface.get_ele_attr(ptre_refno) {
        let axis_param_map = query_axis_params(&ptre_am, interface)?;
        axis_params = axis_param_map.values().cloned().collect::<Vec<_>>();
        axis_param_numbers = axis_param_map.keys().cloned().collect::<Vec<_>>();
    }

    let gmref_name = if is_sprf { "GSTR" } else { "GMRE" };
    let gmse_refno = attr_map.get_foreign_refno(gmref_name)?;
    let mut gm_params = vec![];
    let gmse_am = interface.get_ele_attr(gmse_refno)?;
    gm_params = query_gm_params(&gmse_am, interface)?;

    Ok(ScomInfo {
        gtype: attr_map.get_as_string("GTYP")?,
        dtse_params: vec![],
        gm_params,
        axis_params,
        params: attr_map
            .get_as_string("PARA")?
            .replace("\n", " ")
            .replace("  ", " ").into(),
        axis_param_numbers,
        attr_map,
    })
}

pub fn query_axis_params<T: PdmsDataInterface>(
    attr_map: &AttrMap,
    interface: &T,
) -> anyhow::Result<BTreeMap<i32, AxisParam>> {
    // 查找ptse
    let mut map = BTreeMap::new();
    let refno = attr_map.get_refno()?;
    let children = interface.get_ele_children_attrs(refno);
    for child in children {
        let number = child.get_as_string("NUMB")?.parse::<i32>().unwrap_or(-1);
        map.entry(number).or_insert(get_axis_param(&child)?);
    }
    Ok(map)
}

///查询gmse的参数
pub fn query_gm_params<T: PdmsDataInterface>(
    attr_map: &AttrMap,
    interface: &T,
) -> anyhow::Result<Vec<GmParam>> {
    let mut gms = vec![];
    let refno = attr_map.get_refno().unwrap();
    let children = interface
        .get_ele_children_attrs(refno);
    for child in children {
        let has_chidren = child.get_type_cloned() == "SPRO";//todo add other types
        gms.push(query_gm_param(&child, interface, has_chidren)?);
    }
    Ok(gms)
}


///对元件库的SCOM Element进行求值计算
pub fn resolve_cata_comp<T: PdmsDataInterface>(
    scom_info: &ScomInfo,
    interface: &T,
    context: Option<HashMap<SmolStr, SmolStr>>,
) -> anyhow::Result<GeomsInfo> {
    let mut cur_context = context.unwrap_or_default();
    //默认值
    cur_context
        .entry(DDHEIGHT_STR.into())
        .or_insert("0.0".into());
    cur_context
        .entry(DDRADIUS_STR.into())
        .or_insert("0.0".into());
    cur_context
        .entry(DDANGLE_STR.into())
        .or_insert("0.0".into());
    //获取DTSE的expression
    process_dtse_params(&scom_info.attr_map, interface, &mut cur_context);

    //保温层厚度
    cur_context.insert("IPARA0".into(), "0".into());
    cur_context.insert("IPARA".into(), "0".into());
    //PARA
    let params = scom_info.attr_map.get_f64_vec("PARA")?;
    for i in 0..params.len() {
        cur_context.insert(format!("OPAR{}", i + 1).into(), params[i].to_string().into());
        cur_context.insert(format!("CPAR{}", i + 1).into(), params[i].to_string().into());
        cur_context.insert(format!("PARA{}", i + 1).into(), params[i].to_string().into());
        cur_context.insert(format!("PARAM{}", i + 1).into(), params[i].to_string().into());
        cur_context.insert(format!("IPARA{}", i + 1).into(), "0".to_string().into());
    }
    //求解AXIS的数据
    let axis_map = resolve_axis_params(scom_info, &cur_context);

    //dbg!(&scom_info.gm_params);
    let geometries = resolve_gms(&scom_info.gm_params, &cur_context, &axis_map, None)?;
    //dbg!(&geometries);
    Ok(GeomsInfo {
        geometries,
        axis_map,
        tubi_bore: None,
    })
}

///获得AxisParam
pub fn get_axis_param(attr_map: &AttrMap) -> anyhow::Result<AxisParam> {
    let type_name = attr_map.get_as_string("TYPE")?;
    let pconnect = attr_map.get_as_string("PCON")?;
    let pbore = attr_map.get_as_string("PBOR")?;
    let refno = attr_map.get_refno();
    let pos = attr_map.get_f64_vec("POS").unwrap_or(vec![0.0, 0.0, 0.0]);
    let r = match type_name.as_ref() {
        "PTAX" => AxisParam {
            attr_map: attr_map.clone(),
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: attr_map.get_as_string("PDIS")?,
            direction: attr_map.get_as_string("PAXI")?,
            pconnect,
            pbore,
        },
        "PTCA" => AxisParam {
            attr_map: attr_map.clone(),
            x: attr_map.get_as_string("PX")?,
            y: attr_map.get_as_string("PY")?,
            z: attr_map.get_as_string("PZ")?,
            distance: "".into(),
            direction: attr_map.get_as_string("PTCD")?,
            pconnect,
            pbore,
        },
        "PTMI" => AxisParam {
            attr_map: attr_map.clone(),
            x: attr_map.get_as_string("PX")?,
            y: attr_map.get_as_string("PY")?,
            z: attr_map.get_as_string("PZ")?,
            distance: "".into(),
            direction: attr_map.get_as_string("PAXI")?,
            pconnect,
            pbore,
        },
        "PTPOS" => AxisParam {
            attr_map: attr_map.clone(),
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: attr_map.get_as_string("PTCP")?,
            direction: attr_map.get_as_string("PTCD")?,
            pconnect,
            pbore,
        },
        _ => AxisParam {
            attr_map: attr_map.clone(),
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: "".into(),
            direction: "".into(),
            pconnect,
            pbore,
        },
    };
    Ok(r)
}

///获得gmse的params
pub fn query_gm_param(attr_map: &AttrMap, interface: &dyn PdmsDataInterface, has_chidren: bool) -> anyhow::Result<GmParam> {
    let mut paxises = attr_map.get_attr_strings(&["PAXI", "PAAX", "PBAX", "PCAX"]);
    if let Ok(val) = attr_map.get_val("PTS") {
        match val {
            IntArrayType(v) => {
                for s in v {
                    paxises.push(s.to_string().into());
                }
            }
            _ => {}
        }
    }
    paxises.push(attr_map.get_as_string("PLAX")?);
    let centre_line_flag = attr_map.get_bool("CLFL").unwrap_or_default();
    let tube_flag = attr_map.get_bool("TUFL").unwrap_or_default();
    let mut verts = vec![];
    let mut dxy = vec![];
    //大部分是顶点数据
    if has_chidren {
        for a in interface.get_ele_children_attrs(attr_map.get_refno().unwrap()) {
            verts.push([a.get_as_string("PX")?, a.get_as_string("PY")?]);
            dxy.push([a.get_as_string("DX")?, a.get_as_string("DY")?]);
        }
    } else {
        verts = vec![[attr_map.get_as_string("PX")?, attr_map.get_as_string("PY")?]];
        dxy = vec![[attr_map.get_as_string("DX")?, attr_map.get_as_string("DY")?]];
    }
    Ok(GmParam {
        refno: attr_map.get_refno()?,
        gm_type: attr_map.get_type_cloned(),
        prad: attr_map.get_as_string("PRAD")?,
        pang: attr_map.get_as_string("PANG")?,
        pwid: attr_map.get_as_string("PWID")?,
        diameters: attr_map.get_attr_strings(&["PDIA", "PBDM", "PTDM", "DIAM"]),
        distances: attr_map.get_attr_strings(&["PDIS", "PBDI", "PTDI"]),
        phei: attr_map.get_as_string("PHEI")?,
        offset: attr_map.get_as_string("POFF")?,
        box_lengths: attr_map.get_attr_strings(&["PXLE", "PYLE", "PZLE"]),
        xyz: attr_map.get_attr_strings(&["PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF"],
        ),
        verts,
        dxy,
        drad: attr_map.get_as_string("DRAD")?,
        dwid: attr_map.get_as_string("DWID")?,
        paxises, // 先pa_axis, 后pb_axis
        centre_line_flag,
        visible_flag: tube_flag,
    })
}

///获得dtse的参数信息
pub fn process_dtse_params<T: PdmsDataInterface>(
    attr_map: &AttrMap,
    interface: &T,
    context: &mut HashMap<SmolStr, SmolStr>,
) -> anyhow::Result<bool> {
    let dtre_refno = attr_map.get_foreign_refno("DTRE")?;
    let children = interface.get_ele_children_attrs(dtre_refno);
    for child in children {
        let key = child.get_as_string("DKEY")?;
        let exp = child.get_as_string("PPRO")?;
        let default_key = format!("{}_default_expr", key);
        let default_expr = child.get_as_string("DPRO")?;
        context.insert(key, exp);
        context.insert(default_key.into(), default_expr);
    }
    Ok(true)
}
