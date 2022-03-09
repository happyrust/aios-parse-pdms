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
use smol_str::SmolStr;
use crate::data_interface::PdmsDataInterface;

pub const DDHEIGHT_STR: &'static str = "DDHEIGHT";
pub const DDRADIUS_STR: &'static str = "DDRADIUS";
pub const DDANGLE_STR: &'static str = "DDANGLE";



///求解design component
pub async fn resolve_desi_comp<T: PdmsDataInterface>(
    refno: &RefU64,
    interface: &T,
) -> Option<GeomsInfo> {

    let attr_map = interface.get_ele_attr_async(refno).await;
    if attr_map.is_none() { return None; }
    let attr_map = attr_map.unwrap();
    let mut desp = get_attr_value_int_vec(&attr_map, "DESP");

    let spre_ref = attr_map.get_foreign_refno("SPRE").unwrap_or_default();
    let mut scom_ref = Some(spre_ref);
    if let Some(spre) = interface.get_ele_attr_async(&spre_ref).await{
        if spre.contains_attr_name("CATR") {
            scom_ref = spre.get_foreign_refno("CATR");
        }
    }
    if scom_ref.is_none() { return None; }
    let scom_ref = scom_ref.unwrap();
    let scom_info = query_scom_info(&scom_ref, interface).await;
    if scom_info.is_none() { return None; }
    let mut context: HashMap<SmolStr, SmolStr> = HashMap::new();
    for i in 0..desp.len() {
        context.insert(
            format!("DESP{}", i + 1).into(),
            desp[i].to_string().into(),
        );
    }
    context.insert(DDHEIGHT_STR.into(), attr_map.get_as_string("HEIG").unwrap_or("0.0".into()));
    context.insert(DDANGLE_STR.into(), attr_map.get_as_string("ANGL").unwrap_or("0.0".into()));
    context.insert(DDRADIUS_STR.into(), attr_map.get_as_string("RADI").unwrap_or("0.0".into()));

    dbg!(&context);
    let mut geom_info = resolve_cata_comp(scom_info.as_ref().unwrap(), interface, Some(context)).await;
    Some(geom_info)
}


///整合SCOM对应的临时数据
pub async fn query_scom_info<T: PdmsDataInterface>(
    refno: &RefU64,
    interface: &T,
) -> Option<ScomInfo> {
    if let Some(attr_map) = interface.get_ele_attr_async(refno).await {
        let type_noun = attr_map.get_type_cloned();
        let is_sprf = type_noun == "SPRF";
        // if type_noun == "SPRF" {
        //     let gmss_refno = attr_map.get_foreign_refno("GSTR").unwrap_or_default();
        //     if let Some(gmss_attr) = interface
        //         .get_ele_attr(&gmss_refno)
        //         .await
        //     {
        //         let gmss_refno = gmss_attr.get_refno().unwrap();
        //         // dbg!(gmss_refno.to_refno_str());
        //         let children = interface
        //             .get_ele_children_refs_async(&gmss_refno)
        //             .await;
        //         let mut gm_params = vec![];
        //         for child in children {
        //             let mut gm_param = GmParam::default();
        //             gm_param.visible_flag = true;
        //             let attr_map = interface.get_ele_attr(&child).await.unwrap();
        //             let tmp_type = attr_map.get_type();
        //             if tmp_type.as_str() == "SPRO" {
        //                 let spve = interface
        //                     .get_ele_children_refs_async(&child)
        //                     .await;
        //                 for v in spve {
        //                     let attr_map = interface.get_ele_attr(&v).await.unwrap();
        //                     gm_param.verts.push([
        //                         attr_map.get_as_string("PX").unwrap_or_default(),
        //                         attr_map.get_as_string("PY").unwrap_or_default(),
        //                     ]);
        //                 }
        //             }
        //             gm_params.push(gm_param);
        //             //todo 需要弄清楚是否只有一个有用
        //             return Some(ScomInfo {
        //                 gtype: attr_map.get_as_string("GTYP").unwrap_or_default(),
        //                 dtse_params: vec![],
        //                 gm_params,
        //                 axis_params: vec![],
        //                 params: attr_map
        //                     .get_as_string("PARA").unwrap_or_default()
        //                     .replace("\n", " ")
        //                     .replace("  ", " ").into(),
        //                 axis_param_numbers: vec![],
        //                 attr_map,
        //             });
        //         }
        //     }
        // }

        //todo collect PLIN data
        let ptref_name = if is_sprf { "PSTR" } else { "PTRE"};
        let ptre_refno = attr_map.get_foreign_refno(ptref_name).unwrap_or_default();
        let mut axis_params = vec![];
        let mut axis_param_numbers = vec![];
        if let Some(ptre_am) = interface
            .get_ele_attr_async(&ptre_refno)
            .await
        {
            let axis_param_map = query_axis_params(&ptre_am, interface).await;
            axis_params = axis_param_map.values().cloned().collect::<Vec<_>>();
            axis_param_numbers = axis_param_map.keys().cloned().collect::<Vec<_>>();
        }

        let gmref_name = if is_sprf { "GSTR" } else { "GMRE"};
        let gmse_refno = attr_map.get_foreign_refno(gmref_name).unwrap_or_default();
        let mut gm_params = vec![];
        if let Some(gmse_am) = interface
            .get_ele_attr_async(&gmse_refno)
            .await
        {
            gm_params = query_gm_params(&gmse_am, interface).await;
        }

        return Some(ScomInfo {
            gtype: attr_map.get_as_string("GTYP").unwrap_or_default(),
            dtse_params: vec![],
            gm_params,
            axis_params,
            params: attr_map
                .get_as_string("PARA").unwrap_or_default()
                .replace("\n", " ")
                .replace("  ", " ").into(),
            axis_param_numbers,
            attr_map,
        });
    }
    None
}

pub async fn query_axis_params<T: PdmsDataInterface>(
    attr_map: &AttrMap,
    interface: &T,
) -> BTreeMap<i32, AxisParam> {
    // 查找ptse
    let mut map = BTreeMap::new();
    let refno = attr_map.get_refno().unwrap_or_default();
    let children = interface
        .get_ele_children_attrs_async(&refno)
        .await;
    for child in children {
        let number = child.get_as_string("NUMB").unwrap_or_default().parse::<i32>().unwrap_or(-1);
        map.entry(number).or_insert(get_axis_param(&child));
    }
    map
}

///查询gmse的参数
pub async fn query_gm_params<T: PdmsDataInterface>(
    attr_map: &AttrMap,
    interface: &T,
) -> Vec<GmParam> {
    let mut gms = vec![];
    let refno = attr_map.get_refno().unwrap();
    let children = interface
        .get_ele_children_attrs_async(&refno)
        .await;
    for child in children {
        let has_chidren = child.get_type_cloned() == "SPRO";//todo add other types
        gms.push(query_gm_param(&child, interface, has_chidren).await);
    }
    gms
}


///对元件库的SCOM Element进行求值计算
pub async fn resolve_cata_comp<T: PdmsDataInterface>(
    scom_info: &ScomInfo,
    interface: &T,
    context: Option<HashMap<SmolStr, SmolStr>>
) -> GeomsInfo {
    let mut cur_context = HashMap::new();
    if context.is_some(){
        cur_context = context.unwrap();
    }
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
    process_dtse_params(&scom_info.attr_map, interface, &mut cur_context).await;

    // dbg!(&scom_info);

    //保温层厚度
    cur_context.insert("IPARAM0".into(), "0".into());
    let params = get_attr_value_f64_vec(&scom_info.attr_map, "PARA").unwrap_or_default();
    for i in 0..params.len() {
        cur_context.insert(format!("PARAM{}", i + 1).into(), params[i].to_string().into());
        cur_context.insert(format!("IPARAM{}", i + 1).into(), "0".to_string().into());
    }
    //求解AXIS的数据
    // dbg!(&scomp_info.axis_params);
    let axis_map = resolve_axis_params(scom_info, &cur_context);
    // dbg!(&axis_map);
    //求解子节点几何模型的数据

    //if gmse
    let geometries = resolve_gms(&scom_info.gm_params, &cur_context, &axis_map, None);
    // dbg!(&geometries);
    GeomsInfo {
        geometries,
        axis_map,
        tubi_bore: None,
    }
}

///获得AxisParam
pub fn get_axis_param(attr_map: &AttrMap) -> AxisParam {
    let type_name = attr_map.get_as_string("TYPE").unwrap_or_default();
    let pconnect = attr_map.get_as_string("PCON").unwrap_or_default();
    let pbore = attr_map.get_as_string("PBOR").unwrap_or_default();
    let refno = attr_map.get_refno();
    let pos = get_attr_value_f64_vec(attr_map, "POS").unwrap_or(vec![0.0, 0.0, 0.0]);
    match type_name.as_ref() {
        "PTAX" => AxisParam {
            attr_map: attr_map.clone(),
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: attr_map.get_as_string("PDIS").unwrap_or_default(),
            direction: attr_map.get_as_string("PAXI").unwrap_or_default(),
            pconnect,
            pbore,
        },
        "PTCA" => AxisParam {
            attr_map: attr_map.clone(),
            x: attr_map.get_as_string("PX").unwrap_or_default(),
            y: attr_map.get_as_string("PY").unwrap_or_default(),
            z: attr_map.get_as_string("PZ").unwrap_or_default(),
            distance: "".into(),
            direction: attr_map.get_as_string("PTCD").unwrap_or_default(),
            pconnect,
            pbore,
        },
        "PTMI" => AxisParam {
            attr_map: attr_map.clone(),
            x: attr_map.get_as_string("PX").unwrap_or_default(),
            y: attr_map.get_as_string("PY").unwrap_or_default(),
            z: attr_map.get_as_string("PZ").unwrap_or_default(),
            distance: "".into(),
            direction: attr_map.get_as_string("PAXI").unwrap_or_default(),
            pconnect,
            pbore,
        },
        "PTPOS" => AxisParam {
            attr_map: attr_map.clone(),
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: attr_map.get_as_string("PTCP").unwrap_or_default(),
            direction: attr_map.get_as_string("PTCD").unwrap_or_default(),
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
    }
}

///获得gmse的params
pub async fn query_gm_param(attr_map: &AttrMap, interface: &dyn PdmsDataInterface, has_chidren: bool) -> GmParam {
    // dbg!(attr_map.to_string_hashmap());
    let mut paxises = get_attr_strings_db(attr_map, &["PAXI", "PAAX", "PBAX", "PCAX"]);
    if let Some(val) = attr_map.get_val("PTS") {
        match val {
            IntArrayType(v) => {
                for s in v {
                    paxises.push(s.to_string().into());
                }
            }
            _ => {}
        }
    }
    paxises.push(attr_map.get_as_string("PLAX").unwrap_or_default());
    let centre_line_flag = attr_map.get_bool("CLFL");
    let tube_flag = attr_map.get_bool("TUFL");
    let mut verts = vec![];
    let mut dxy = vec![];
    //大部分是顶点数据
    if has_chidren {
        for a in interface.get_ele_children_attrs_async(&attr_map.get_refno().unwrap()).await{
            verts.push([a.get_as_string("PX").unwrap_or_default(), a.get_as_string("PY").unwrap_or_default()]);
            dxy.push([a.get_as_string("DX").unwrap_or_default(), a.get_as_string("DY").unwrap_or_default()]);
        }
    }else{
        verts = vec![[attr_map.get_as_string("PX").unwrap_or_default(),attr_map.get_as_string("PY").unwrap_or_default()]];
        dxy = vec![[attr_map.get_as_string("DX").unwrap_or_default(),attr_map.get_as_string("DY").unwrap_or_default()]];
    }
    GmParam {
        refno: attr_map.get_refno().unwrap_or_default(),
        gm_type: attr_map.get_type_cloned(),
        prad: attr_map.get_as_string("PRAD").unwrap_or_default(),
        pang: attr_map.get_as_string("PANG").unwrap_or_default(),
        pwid: attr_map.get_as_string("PWID").unwrap_or_default(),
        diameters: get_attr_strings_db(attr_map, &["PDIA", "PBDM", "PTDM", "DIAM"]),
        distances: get_attr_strings_db(attr_map, &["PDIS", "PBDI", "PTDI"]),
        phei: attr_map.get_as_string("PHEI").unwrap_or_default(),
        offset: attr_map.get_as_string("POFF").unwrap_or_default(),
        box_lengths: get_attr_strings_db(attr_map, &["PXLE", "PYLE", "PZLE"]),
        xyz: get_attr_strings_db(
            attr_map,
            &[
                "PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF",
            ],
        ),
        verts,
        dxy,
        drad: attr_map.get_as_string("DRAD").unwrap_or_default(),
        dwid: attr_map.get_as_string("DWID").unwrap_or_default(),
        paxises, // 先pa_axis, 后pb_axis
        centre_line_flag,
        visible_flag: tube_flag,
    }
}

///获得dtse的参数信息
pub async fn process_dtse_params<T:PdmsDataInterface>(
    attr_map: &AttrMap,
    interface: &T,
    context: &mut HashMap<SmolStr, SmolStr>,
){

    let dtre_refno = attr_map.get_foreign_refno("DTRE").unwrap_or_default();
    let children = interface
        .get_ele_children_attrs_async(&dtre_refno)
        .await;
    for child in children {
        let key = child.get_as_string("DKEY").unwrap_or_default();
        let exp = child.get_as_string("PPRO").unwrap_or_default();
        let default_key = format!("{}_default_expr", key);
        let default_expr = child.get_as_string("DPRO").unwrap_or_default();
        context.insert(key, exp);
        context.insert(default_key.into(), default_expr);
    }
}
