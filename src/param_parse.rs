use std::collections::HashMap;
use regex::Regex;
use crate::direction_parse::parse_expr_to_dir;
use crate::get_attr_tool::{convert_to_context_key, resolve_axis_param};
use crate::pdms_origin_data::{AxisParam, ScomParamStr};
use crate::pdms_parsed_data::geo_params_data::CateGeoParams;
use crate::pdms_parsed_data::{CateBoxImpliedParam, CateBoxParam, CateConeParam, CateCylinderParam, CateDiscParam, CateDishParam, CateExtrusionParam, CateLineParam, CatePyramidParam, CateRectTorusParam, CateRevolutionParam, CateSlineParam, CateSlopeBottomCylinderParam, CateSnoutParam, CateSphereParam, CateSverParam, CateTorusParam, GeoParamsData, GmseParamData};
use crate::pdms_types::ElementData;
use crate::polish_notation::Stack;

pub fn parse_design_param_to_hashmap(text: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();
    let mut index = 1;
    for d in text.replace("\n", " ").split(" ") {
        if d != "" && d != " " {
            params.insert(format!("DESP{}", index), d.to_string());
            index += 1;
        }
    }
    params
}

pub fn eval_str_to_f64(input_expr: &str, context: &HashMap<String, String>) -> Option<f64> {
    if input_expr.trim() == "unset"{
        return Some(0.0);
    }
    let mut has_desparam = false;
    let mut exp = input_expr.trim_end_matches('\0').to_owned().replace("[", " ").replace("]", " ").replace("  ", " ");
    if exp.len() < 1{
        return Some(0.0);
    }
    if exp.len() >= 2 && exp.chars().nth(0).unwrap_or_default() == '('
        && exp.chars().nth(exp.len() -1).unwrap_or_default() == ')' {
        exp = exp[1..exp.len()-1].to_string();
    }
    let seg_strs = exp.split_whitespace().map(|x| x.trim().to_owned()).collect::<Vec<_>>();
    if seg_strs.len() == 0 {
        return None;
    }
    // dbg!(&context);
    // dbg!(&seg_strs);
    let mut p_vals = Vec::new();
    let mut i = 0;
    let mut twice_flag = false; //翻倍
    let mut tanf_flag = false; //翻倍
    let mut tan_flag = false; //翻倍
    while i < seg_strs.len() {
        let mut key = "".to_string();
        let s = seg_strs[i].as_str();
        if (s == "PARAM" || s == "IPARAM") && i < seg_strs.len() {
            key = convert_to_context_key(s, &mut i, &seg_strs).unwrap_or_default();
        } else if s == "ATTRIB" && i < seg_strs.len() - 1 {
            i += 1;
            let s_n = seg_strs[i].as_str();
            if s_n == "RPRO" {
                let dtse_key = seg_strs[i + 1].as_str();
                if context.contains_key(dtse_key) {
                    if let Some(r) = eval_str_to_f64(&context[dtse_key], context) {
                        p_vals.push(r.to_string());
                    } else {
                        let default_key = format!("{}_default_expr", dtse_key);
                        if context.contains_key(&default_key) {
                            if let Some(r) = eval_str_to_f64(&context[&default_key], context) {
                                p_vals.push(r.to_string());
                            }
                        }
                    }
                }
                i += 2;
                continue;
            } else {
                key = convert_to_context_key(s_n, &mut i, &seg_strs).unwrap_or_default();
            }
        }
        if context.contains_key(&key) {
            p_vals.push(context[&key].clone());
            i += 1;
            continue;
        }

        let upper_s = s.to_uppercase();
        match upper_s.as_str() {
            "TIMES" | "MULT" => p_vals.push("*".to_string()),
            "DIV" => p_vals.push("/".to_string()),
            "DDHEIGHT" => p_vals.push(context["DDHEIGHT"].to_string()),
            "DDRADIUS" => p_vals.push(context["DDRADIUS"].to_string()),
            "DDANGLE" => p_vals.push(context["DDANGLE"].to_string()),
            _ => p_vals.push(upper_s),
        }
        i += 1;
    }

    //对TWICE、tanf、tan做单独处理
    let mut need_del_keys = vec![];
    for i in 0..p_vals.len() {
        if p_vals[i] == "TWICE" {
            need_del_keys.push(i);
            if i + 1 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    let v = val * 2.0f64;
                    p_vals[i + 1] = v.to_string();
                }
            }
        } else if p_vals[i] == "TANF" {
            need_del_keys.push(i);
            need_del_keys.push(i + 1);
            if i + 2 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    if let Ok(angle) = p_vals[i + 2].parse::<f64>() {
                        {
                            let v = val * ((angle / 2.0).to_radians() as f64).tan();
                            p_vals[i + 2] = v.to_string();
                        }
                    }
                }
            }
        } else if p_vals[i] == "TAN" || p_vals[i] == "SIN" || p_vals[i] == "COS" {
            if i + 2 < p_vals.len() {
                if let Ok(val) = p_vals[i + 2].parse::<f64>() {
                    let v = val.to_radians();
                    p_vals[i + 2] = v.to_string();
                }
            }
        }
        // 单位处理，mm为基本单位
        if p_vals[i].contains("mm") {
            p_vals[i] = p_vals[i].replace("mm", "");
        }
    }
    for v in need_del_keys {
        p_vals.remove(v);
    }
    if p_vals.is_empty() {
        return None;
    }
    let mut result_string = String::new();
    let mut start_idx = 0;
    let mut i = start_idx;
    while i < p_vals.len() {
        if (p_vals[i] == "SUM" || p_vals[i] == "DIFFERENCE") && i < p_vals.len() - 2 {
            if p_vals[i] == "SUM" {
                result_string.push_str(&format!(
                    "({} {} {})",
                    p_vals[i + 1],
                    "+",
                    p_vals[i + 2]
                ));
            } else {
                result_string.push_str(&format!(
                    "({} {} {})",
                    p_vals[i + 1],
                    "-",
                    p_vals[i + 2]
                ));
            }
            i += 3;
            continue;
        } else {
            result_string.push_str(&p_vals[i]);
        }
        result_string.push(' ');
        i += 1;
    }
    // ////dbg!(&result_string);
    let mut ns = fasteval::EmptyNamespace;
    ////dbg!(&result_string);
    if let Ok(f) = std::panic::catch_unwind(move || unsafe {
        if let Ok(val) = fasteval::ez_eval(&result_string, &mut ns) {
            ( val * 100.0 ).round() /100.0
        } else {
            let mut stack = Stack::new(&result_string);
            stack.eval()
        }
    }) {
        // ////dbg!(f);
        return Some(f);
    }
    // ////dbg!("None");
    return None;
}

pub fn parse_gmse_param_to_cate_geo_params(gmse: GmseParamData) -> GeoParamsData {
    let data = match &gmse.self_type[..] {
        "BOXI" => {
            let z_length = if gmse.box_lengths.len() >= 3 {
                gmse.box_lengths[2]
            } else {
                gmse.box_lengths[1]
            };
            Some(CateGeoParams::Boxi(CateBoxImpliedParam {
                axis: Some(gmse.paxises[0].clone()),
                x_length: gmse.box_lengths[0],
                z_length,
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LCYL" => {
            // 圆柱体
            Some(CateGeoParams::Cylinder(CateCylinderParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                height: gmse.distances[1] - gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LINE" => {
            Some(CateGeoParams::Line(CateLineParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                diameter: 0.0, //gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LPYR" => {
            Some(CateGeoParams::Pyramid(CatePyramidParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                pc: Some(gmse.paxises[2].clone()),
                x_bottom: gmse.xyz[0],
                y_bottom: gmse.xyz[1],
                x_top: gmse.xyz[2],
                y_top: gmse.xyz[3],
                dist_to_btm: gmse.distances[0],
                dist_to_top: gmse.distances[1],
                x_offset: gmse.xyz[4],
                y_offset: gmse.xyz[5],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LSNO" => {
            Some(CateGeoParams::Snout(CateSnoutParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                dist_to_btm: gmse.distances[0],
                dist_to_top: gmse.distances[1],
                btm_diameter: gmse.diameters[0],
                top_diameter: gmse.diameters[1],
                offset: gmse.offset,
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SBOX" => {
            Some(CateGeoParams::Box(CateBoxParam {
                size: vec![
                    gmse.box_lengths[0],
                    gmse.box_lengths[1],
                    gmse.box_lengths[2],
                ],
                offset: vec![
                    gmse.xyz[0],
                    gmse.xyz[1],
                    gmse.xyz[2],
                ],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SCON" => {
            // 圆锥
            Some(CateGeoParams::Cone(CateConeParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SCTO" => {
            // 弯管
            Some(CateGeoParams::Torus(CateTorusParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SCYL" => {
            // 圆柱体
            Some(CateGeoParams::Cylinder(CateCylinderParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                height: gmse.height,
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SDIS" => {
            // 圆片
            Some(CateGeoParams::Disc(CateDiscParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SDSH" => {
            Some(CateGeoParams::Dish(CateDishParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                height: gmse.height,
                diameter: gmse.diameters[0],
                radius: gmse.radius,
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SEXT" => {
            Some(CateGeoParams::Extrusion(CateExtrusionParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                height: gmse.height,
                x: gmse.xyz[0],
                y: gmse.xyz[1],
                z: gmse.xyz[2],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SLINE" => {
            //todo
            Some(CateGeoParams::Sline(CateSlineParam {
                start_pt: vec![0.0; 3],
                end_pt: vec![0.0; 3],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SREV" => {
            Some(CateGeoParams::Revolution(CateRevolutionParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                angel: gmse.angle,
                x: gmse.xyz[0],
                y: gmse.xyz[1],
                z: gmse.xyz[2],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SRTO" => { // 如 =15192/210474
            // 截面为矩形的弯管
            Some(CateGeoParams::RectTorus(CateRectTorusParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                height: gmse.height,
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SSLC" => {
            //todo
            Some(CateGeoParams::SlopeBottomCylinder(CateSlopeBottomCylinderParam {
                axis: Some(gmse.paxises[0].clone()),
                height: gmse.height,
                diameter: gmse.diameters[0],
                distance: gmse.distances[0],
                x_shear: 0.0,
                y_shear: 0.0,
                alt_x_shear: 0.0,
                alt_y_shear: 0.0,
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SSPH" => {
            // 球
            Some(CateGeoParams::Sphere(CateSphereParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_center: gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SVER" => {
            Some(CateGeoParams::SVER(CateSverParam{
                x: gmse.xyz[0],
                y: gmse.xyz[1],
                radius: gmse.radius,
            }))
        }
        _ => None,
    };
    GeoParamsData {
        cate_geo_params: data
    }
}

pub fn get_dir_and_pos(axis_str: &AxisParam,
                       ddangle: f64,
                       scom: &ScomParamStr,
                       context: &HashMap<String, String>,
                       data: &ElementData) -> (Vec<f64>, Vec<f64>) {
    //替换掉中间出现dataset的值的这种情况 X ( ATTRIB RPRO ANGL ) Z
    let mut dir_str = axis_str.direction.trim().to_string();
    if dir_str.contains("(") {
        ////dbg!(&dir_str);
        let s: Vec<_> = dir_str.split("(").collect();
        if s.len() > 1 {
            let ss: Vec<_> = s[1].split(")").collect();
            if ss.len() > 1 {
                let val_str = ss[0];
                let val_result = eval_str_to_f64(val_str, context).unwrap_or_default().to_string();
                ////dbg!(val_str);
                ////dbg!(&val_result);
                dir_str = dir_str.replace(val_str, &val_result);
                ////dbg!(&dir_str);
            }
        }
    }
    let mut dir = vec![0.0f64; 3];
    let mut pos = vec![0.0f64; 3];

    let re = Regex::new(r"^P\d+$").unwrap();
    // ////dbg!(dir_str);
    if re.is_match(&dir_str) {
        let pnt_indx = dir_str[1..].parse::<i32>().unwrap_or(i32::MAX);
        // ////dbg!(pnt_indx);
        if let Some(indx) = scom.axis_param_number.iter().position(|&x| x == pnt_indx) {
            if let Some(axis) = resolve_axis_param(&scom.axis_param_collections[indx], scom, context, data) {
                dir = axis.dir.clone();
                pos = axis.pt;
            }
        }
    } else {
        dir = parse_str_axis_to_vec3(&dir_str, ddangle).into();
    }
    return (dir, pos);
}

pub fn parse_str_axis_to_vec3(paxis: &str, ddangle: f64) -> [f64; 3] {
    let paxis = paxis.to_uppercase().replace("AXIS", "").replace(" ", "");
    let mut paxis_str = &paxis[..];
    //含DDANGLE的处理
    if paxis_str.contains("DDANGLE") {
        let angle = ddangle * std::f64::consts::PI / 180.0;
        let mut axises: Vec<&str> = Vec::new();
        for s in paxis_str.split("DDANGLE") {
            let s = s.trim();
            if s != "" {
                axises.push(s);
            }
        }
        if axises.len() != 2 {
            panic!("点集 DDANGLE 参数错误！");
        }
        return match &format!("{}{}", axises[0], axises[1])[..] {
            "XY" => [angle.cos(), angle.sin(), 0.0],
            "XZ" => [angle.cos(), 0.0, angle.sin()],
            "YZ" => [0.0, angle.cos(), angle.sin()],
            "YX" => [angle.sin(), angle.cos(), 0.0],
            "ZX" => [angle.sin(), 0.0, angle.cos()],
            "ZY" => [0.0, angle.sin(), angle.cos()],

            "-XY" => [-angle.cos(), angle.sin(), 0.0],
            "-XZ" => [-angle.cos(), 0.0, angle.sin()],
            "-YZ" => [0.0, -angle.cos(), angle.sin()],
            "-YX" => [angle.sin(), -angle.cos(), 0.0],
            "-ZX" => [angle.sin(), 0.0, -angle.cos()],
            "-ZY" => [0.0, angle.sin(), -angle.cos()],

            "X-Y" => [angle.cos(), -angle.sin(), 0.0],
            "X-Z" => [angle.cos(), 0.0, -angle.sin()],
            "Y-Z" => [0.0, angle.cos(), -angle.sin()],
            "Y-X" => [-angle.sin(), angle.cos(), 0.0],
            "Z-X" => [-angle.sin(), 0.0, angle.cos()],
            "Z-Y" => [0.0, -angle.sin(), angle.cos()],

            "-X-Y" => [-angle.cos(), -angle.sin(), 0.0],
            "-X-Z" => [-angle.cos(), 0.0, -angle.sin()],
            "-Y-Z" => [0.0, -angle.cos(), -angle.sin()],
            "-Y-X" => [-angle.sin(), -angle.cos(), 0.0],
            "-Z-X" => [-angle.sin(), 0.0, -angle.cos()],
            "-Z-Y" => [0.0, -angle.sin(), -angle.cos()],

            _ => panic!("点集 DDANGLE 参数错误！"),
        };
    }
    let v = parse_expr_to_dir(paxis_str);
    [v[0] as f64, v[1] as f64, v[2] as f64]
}
