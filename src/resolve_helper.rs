use std::collections::HashMap;
use anyhow::anyhow;
use fixed::types::I24F8;
use nom::Parser;
use regex::Regex;
use rsc::{InterpretError, Num, Variant};
use smol_str::SmolStr;
use crate::direction_parse::parse_expr_to_dir;
use crate::helper::{convert_to_context_key, resolve_axis_param};
use crate::pdms_data::{AxisParam, ScomInfo};
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::parsed_data::{CateBoxImpliedParam, CateBoxParam, CateConeParam, CateDiscParam, CateDishParam, CateExtrusionParam, CateLCylinderParam, CateLineParam, CateProfileParam, CatePyramidParam, CateRectTorusParam, CateRevolutionParam, CateSCylinderParam, CateSlineParam, CateSlopeBottomCylinderParam, CateSnoutParam, CateSphereParam, CateSverParam, CateTorusParam, GmseParamData, SannData};
use crate::pdms_types::EleNode;
use crate::polish_notation::Stack;

#[test]
fn test_expression_regex() {
    let input_exp = "( ( ( -  DESP [1]/2 ) - DESP [2] - ATTRIB CPAR[3]  ) )";
    let new_exp = input_exp.replace("ATTRIB", "");
    let mut map = HashMap::new();

    map.insert("DESP1".to_string(), 1);
    map.insert("CPAR3".to_string(), 2);

    // let regex_str = "";
    let re = Regex::new(r"\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(re.is_match("xx2014-01-01"));

    let re = Regex::new(r"\s+([A-a-zZ0-9]+)\s*\[(\d+)\]").unwrap();
    for cap in re.captures_iter(&new_exp) {
        println!("{} {} {}", &cap[1], &cap[2], &cap[0]);
    }




}

pub fn eval_str_to_f64(input_expr: &str, context: &HashMap<SmolStr, SmolStr>) -> anyhow::Result<f64> {
    if input_expr.trim().to_lowercase() == "unset" {
        return Ok(0.0);  //todo 待验证
    }
    let re = Regex::new(r"\s+([A-a-zZ0-9]+)\s*\[(\d+)\]").unwrap();
    let mut new_exp = input_expr.replace("ATTRIB", "");
    let mut result_exp = new_exp.clone();
    for cap in re.captures_iter(&new_exp) {
        // println!("{} {} {}", &cap[1], &cap[2], &cap[0]);
        let s = &cap[0];
        let k:SmolStr = format!("{}{}", &cap[1], &cap[2]).into();
        if context.contains_key(&k) {
            result_exp = result_exp.replace(s, &context[&k]);
        }
    }
    let re = Regex::new(r"PARAM\s*(\d+)").unwrap();
    let mut new_exp = result_exp.clone();
    for cap in re.captures_iter(&result_exp) {
        // println!("{} {} {}", &cap[1], &cap[2], &cap[0]);
        let s = &cap[0];
        let k:SmolStr = format!("PARA{}", &cap[1]).into();
        if context.contains_key(&k) {
            new_exp = new_exp.replace(s, &context[&k]);
        }
    }


    let seg_strs: Vec<SmolStr> = new_exp.split_whitespace().map(|x| x.trim().into()).collect::<Vec<_>>();
    if seg_strs.len() == 0 {
        // return Err(anyhow!("表达式分段数量为 0".to_string()));
        return Ok(0.0);
    }

    let mut result_string = String::new();
    let mut p_vals = vec![];
    for s in seg_strs {
        let upper_s = s.to_uppercase();
        match upper_s.as_str() {
            "TIMES" | "MULT" => p_vals.push("*".to_string()),
            "DIV" => p_vals.push("/".to_string()),
            "DDHEIGHT" => p_vals.push(context["DDHEIGHT"].to_string()),
            "DDRADIUS" => p_vals.push(context["DDRADIUS"].to_string()),
            "DDANGLE" => p_vals.push(context["DDANGLE"].to_string()),
            _ => {
                if upper_s.ends_with("mm") {
                    p_vals.push(upper_s[..upper_s.len()-2].to_string());
                }else{
                    p_vals.push(upper_s.to_string())
                }
            },
        }
    }

    let mut i = 0;
    while i < p_vals.len() {
        if p_vals[i] == "TWICE" {
            if i + 1 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    let v = val * 2.0f64;
                    result_string.push_str(v.to_string().as_str());
                }
            }
            i += 2;
        } else if p_vals[i] == "TANF" {
            if i + 2 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    if let Ok(angle) = p_vals[i + 2].parse::<f64>() {
                        {
                            let v = val * ((angle / 2.0).to_radians() as f64).tan();
                            result_string.push_str(v.to_string().as_str());
                        }
                    }
                }
            }
            i += 3;
        } else if p_vals[i] == "TAN" || p_vals[i] == "SIN" || p_vals[i] == "COS" {
            if i + 1 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    let v = val.to_radians();
                    result_string.push_str(&p_vals[i]);
                    result_string.push_str(v.to_string().as_str());
                }
            }
            i += 2;
        }else{
            result_string.push_str(&p_vals[i]);
            i += 1;
        }
    }

    // if let Ok(f) = std::panic::catch_unwind(move || unsafe {
    //     if let Ok(val) = tinyexpr::interp(&result_string.to_lowercase()) {
    //         (val * 100.0).round() / 100.0
    //     } else {
    //         if let Ok(mut stack) = Stack::init(&result_string){
    //             return stack.eval();
    //         }else{
    //             dbg!(&context);
    //             dbg!(&input_expr);
    //             dbg!(&result_string);
    //             return 0.0;
    //         }
    //     }
    // }) {
    //     return Ok(f);
    // }else{
    //     return Err(anyhow!(format!("求解失败 {}", input_expr)));
    // }

    if let Ok(val) = tinyexpr::interp(&result_string.to_lowercase()) {
        Ok(I24F8::from_num(val).into())
    } else {
        if let Ok(mut stack) = Stack::init(&result_string){
            return stack.eval().ok_or(anyhow!(format!("求解失败 {}", input_expr)));
        }else{
            dbg!(&context);
            dbg!(&result_string);
            return Err(anyhow!(format!("求解失败 {}", input_expr)));;
        }
    }

}

// fn mk_callback<'a, F>(f: F) -> Callback<'a>
//     where F: Fn<(&'a mut State,),Output=()> + 'static {
//     Box::new(f) as Callback
// }

#[test]
fn test_eval_expression() {
    let result_string = "TWICE 90";
    let mut interpreter = rsc::Interpreter::default();
    // let mut eval_fn_map = HashMap::new();
    // eval_fn_map.insert("sin".to_string(), Box::new(|n: f64|{
    //     n.to_radians().sin()
    // }));
    // interpreter.set_var(String::from("sin"), Variant::Function(|name, args| {
    //     if args.len() < 1 {
    //         Err(InterpretError::TooFewArgs(name, 1))
    //     }  else {
    //         Ok( eval_fn_map.get("sin").map(|func| (func)(args[0])).unwrap() ) // get the only argument and double it
    //     }
    // }));
    interpreter.set_var(String::from("sin"), Variant::Function(|name, args| {
        if args.len() < 1 {
            Err(InterpretError::TooFewArgs(name, 1))
        }  else {
            Ok( args[0].to_radians().sin() ) // get the only argument and double it
        }
    }));
    interpreter.set_var(String::from("tan"), Variant::Function(|name, args| {
        if args.len() < 1 {
            Err(InterpretError::TooFewArgs(name, 1))
        }  else {
            Ok( args[0].to_radians().tan() ) // get the only argument and double it
        }
    }));
    interpreter.set_var(String::from("tanf"), Variant::Function(|name, args| {
        if args.len() < 2 {
            Err(InterpretError::TooFewArgs(name, 2))
        }  else {
            Ok( args[0] * ((args[1] / 2.0).to_radians()).tan() ) // get the only argument and double it
        }
    }));
    interpreter.set_var(String::from("twice"), Variant::Function(|name, args| {
        if args.len() < 1 {
            Err(InterpretError::TooFewArgs(name, 1))
        }  else {
            Ok( args[0] * 2.0 ) // get the only argument and double it
        }
    }));
    if let Ok(f) = std::panic::catch_unwind(move || unsafe {
        // if let Ok(val) = tinyexpr::interp(&result_string.to_lowercase()) {
        let mut value = None;
        match rsc::tokenize(&result_string.to_lowercase()) {
            Ok(tokens) => match rsc::parse(&tokens) {
                Ok(expr) => match interpreter.eval(&expr) { // Step 3: interprets the Expr
                    Ok(result) => {
                        value = Some(result);
                    },
                    Err(interpret_error) => eprintln!("{:?}:{:?}", &expr, interpret_error),
                },
                _ => {}
            }
            _ => {}
        }
        value
    }){
        println!("{:?}", f);
    }

}

// pub fn eval_str_to_f64_old(input_expr: &str, context: &HashMap<SmolStr, SmolStr>) -> Option<f64> {
//     if input_expr.trim() == "unset" {
//         return Some(0.0);
//     }
//     //dbg!(&input_expr);
//     let _has_desparam = false;
//     let mut exp = input_expr.trim_end_matches('\0')
//         .to_owned().replace("[", " ").replace("]", " ").replace("  ", " ");
//     if exp.len() < 1 {
//         return Some(0.0);
//     }
//
//     if exp.len() >= 2 && exp.chars().nth(0).unwrap_or_default() == '('
//         && exp.chars().nth(exp.len() - 1).unwrap_or_default() == ')' {
//         exp = exp[1..exp.len() - 1].to_string();
//     }
//     let seg_strs: Vec<SmolStr> = exp.split_whitespace().map(|x| x.trim().into()).collect::<Vec<_>>();
//     if seg_strs.len() == 0 {
//         return None;
//     }
//     //dbg!(&seg_strs);
//     let mut p_vals: Vec<SmolStr> = Vec::new();
//     let mut i = 0;
//     while i < seg_strs.len() {
//         let mut key = SmolStr::default();
//         let mut s = seg_strs[i].as_str();
//         if s.len() == 0 { continue; }
//
//         let mut slice_index = 0;
//         if s.len() >= 2 {
//             let first_char = s.chars().next().unwrap();
//             if first_char == '-' ||
//                 first_char == '+' ||
//                 first_char == '/' ||
//                 first_char == '*' {
//                 p_vals.push(first_char.to_string().into());
//                 let mut test_str = String::new();
//                 p_vals.iter().for_each(|x|{
//                     test_str.push_str(x.as_str());
//                     test_str.push_str(" ");
//                 });
//                 //dbg!(&test_str);
//                 slice_index = 1;
//             }
//         }
//         s = &s[slice_index..];
//         if (s == "PARAM" || s == "IPARAM") && i < seg_strs.len() {
//             key = convert_to_context_key(s, &mut i, &seg_strs).unwrap_or_default();
//         } else if s == "ATTRIB" && i < seg_strs.len() - 1 {
//             i += 1;
//             let s_n = seg_strs[i].as_str();
//             if s_n == "RPRO" {
//                 let dtse_key: SmolStr = seg_strs[i + 1].as_str().into();
//                 if context.contains_key(&dtse_key) {
//                     if let Ok(r) = eval_str_to_f64(&context[&dtse_key], context) {
//                         p_vals.push(r.to_string().into());
//                     } else {
//                         let default_key: SmolStr = format!("{}_default_expr", dtse_key).into();
//                         if context.contains_key(&default_key) {
//                             if let Ok(r) = eval_str_to_f64(&context[&default_key], context) {
//                                 p_vals.push(r.to_string().into());
//                             }else{
//                                 // eprintln!()
//                             }
//                         }
//                     }
//                 }
//                 i += 2;
//                 continue;
//             } else {
//                 key = convert_to_context_key(s_n, &mut i, &seg_strs).unwrap_or_default();
//             }
//         } else if s == "DESIGN" || s == "DESP" {
//             let dtse_key = format!("{} {}", s, seg_strs[i + 1]);
//             key = convert_to_context_key(&dtse_key, &mut i, &seg_strs).unwrap_or_default();
//         }
//         // //dbg!(&context);
//         if context.contains_key(&key) {
//             if key == "ANGL" {
//                 key = context[&key].as_str().into();
//                 key = key.trim_end_matches('\0').to_owned().replace("[", " ").replace("]", " ").replace("  ", " ").into();
//                 if key.len() >= 2 && key.chars().nth(0).unwrap_or_default() == '('
//                     && key.chars().nth(key.len() - 1).unwrap_or_default() == ')' {
//                     key = key[1..key.len() - 1].into();
//                 }
//                 let seg_strs: Vec<SmolStr> = key.split_whitespace().map(|x| x.trim().into()).collect::<Vec<_>>();
//                 if seg_strs.len() == 0 {
//                     return None;
//                 }
//                 let mut j = 0;
//                 while j < seg_strs.len() {
//                     let s = seg_strs[j].as_str();
//                     key = convert_to_context_key(s, &mut j, &seg_strs).unwrap_or_default();
//                     j += 1;
//                 }
//                 if context.contains_key(&key) {
//                     p_vals.push(context[&key].clone());
//                 }
//             } else {
//                 if context.contains_key(&key) {
//                     p_vals.push(context[&key].clone());
//                 }
//             }
//             i += 1;
//             continue;
//         }
//
//         let upper_s: SmolStr = s.to_uppercase().into();
//         match upper_s.as_str() {
//             "TIMES" | "MULT" => p_vals.push("*".into()),
//             "DIV" => p_vals.push("/".into()),
//             "DDHEIGHT" => p_vals.push(context["DDHEIGHT"].as_str().into()),
//             "DDRADIUS" => p_vals.push(context["DDRADIUS"].as_str().into()),
//             "DDANGLE" => p_vals.push(context["DDANGLE"].as_str().into()),
//             _ => p_vals.push(upper_s),
//         }
//         i += 1;
//     }
//     //对TWICE、tanf、tan做单独处理
//     let mut need_del_keys = vec![];
//
//     for i in 0..p_vals.len() {
//         if p_vals[i] == "TWICE" {
//             need_del_keys.push(i);
//             if i + 1 < p_vals.len() {
//                 if let Ok(val) = p_vals[i + 1].parse::<f64>() {
//                     let v = val * 2.0f64;
//                     p_vals[i + 1] = v.to_string().into();
//                 }
//             }
//         } else if p_vals[i] == "TANF" {
//             need_del_keys.push(i);
//             need_del_keys.push(i + 1);
//             if i + 2 < p_vals.len() {
//                 if let Ok(val) = p_vals[i + 1].parse::<f64>() {
//                     if let Ok(angle) = p_vals[i + 2].parse::<f64>() {
//                         {
//                             let v = val * ((angle / 2.0).to_radians() as f64).tan();
//                             p_vals[i + 2] = v.to_string().into();
//                         }
//                     }
//                 }
//             }
//         } else if p_vals[i] == "TAN" || p_vals[i] == "SIN" || p_vals[i] == "COS" {
//             if i + 2 < p_vals.len() {
//                 if let Ok(val) = p_vals[i + 2].parse::<f64>() {
//                     let v = val.to_radians();
//                     p_vals[i + 2] = v.to_string().into();
//                 }
//             }
//         }
//         // 单位处理，mm为基本单位
//         if p_vals[i].contains("mm") {
//             p_vals[i] = p_vals[i].replace("mm", "").into();
//         }
//     }
//     for v in need_del_keys {
//         p_vals.remove(v);
//     }
//     if p_vals.is_empty() {
//         return None;
//     }
//     let mut result_string = String::new();
//     let mut start_idx = 0;
//     let mut i = start_idx;
//     while i < p_vals.len() {
//         if (p_vals[i] == "SUM" || p_vals[i] == "DIFFERENCE") && i < p_vals.len() - 2 {
//             if p_vals[i] == "SUM" {
//                 result_string.push_str(&format!(
//                     "({} {} {})",
//                     p_vals[i + 1],
//                     "+",
//                     p_vals[i + 2]
//                 ));
//             } else {
//                 result_string.push_str(&format!(
//                     "({} {} {})",
//                     p_vals[i + 1],
//                     "-",
//                     p_vals[i + 2]
//                 ));
//             }
//             i += 3;
//             continue;
//         } else {
//             result_string.push_str(&p_vals[i]);
//         }
//         result_string.push(' ');
//         i += 1;
//     }
//     //dbg!(&result_string);
//     // let mut ns = fasteval::EmptyNamespace;
//     let mut interpreter = rsc::Interpreter::default();
//
//     interpreter.set_var(String::from("double"), Variant::Function(|name, args| {
//         if args.len() < 1 {
//             Err(InterpretError::TooFewArgs(name, 1))
//         } else if args.len() > 1 {
//             Err(InterpretError::TooManyArgs(name, 1))
//         } else {
//             Ok(args[0] * 2.0) // get the only argument and double it
//         }
//     }));
//
//     if let Ok(f) = std::panic::catch_unwind(move || unsafe {
//         // if let Ok(val) = tinyexpr::interp(&result_string.to_lowercase()) {
//         let mut value = None;
//         match rsc::tokenize(&result_string.to_lowercase()) {
//             Ok(tokens) => match rsc::parse(&tokens) {
//                 Ok(expr) => match interpreter.eval(&expr) { // Step 3: interprets the Expr
//                     Ok(result) => {
//                         value = Some(result);
//                     },
//                     Err(interpret_error) => eprintln!("{:?}", interpret_error),
//                 },
//                 _ => {}
//             }
//             _ => {}
//         }
//         if value.is_some() {
//             (value.unwrap() * 100.0).round() / 100.0
//         } else {
//             if let Ok(mut stack) = Stack::init(&result_string){
//                 return stack.eval();
//             }else{
//                 if result_string.as_str() != "UNSET" {
//                     dbg!(&context);
//                     dbg!(&input_expr);
//                     dbg!(&result_string);
//                 }
//                 return 0.0;
//             }
//         }
//     }) {
//         return Some(f);
//     }
//     return None;
// }

#[test]
pub fn test_expression() {
    let mut ns = fasteval::EmptyNamespace;
    // power ( 0 ,2 )
    //let r = tinyexpr::interp("2+2*2").unwrap();
    let s = tinyexpr::interp(" ( / 2 + 60 )");
    //let s  = fasteval::ez_eval("( 2 ^ 2 )", &mut ns);
    //dbg!(s);
}

pub fn resolve_to_cate_geo_params(gmse: GmseParamData) -> Option<CateGeoParam> {
    let geo = match &gmse.type_name[..] {
        "SANN" => {
            Some(CateGeoParam::Profile(CateProfileParam::SANN(SannData {
                xy: [gmse.verts[0][0], gmse.verts[0][1]],
                dxy: [gmse.dxy[0][0], gmse.dxy[0][1]],
                ptaxis: Some(gmse.paxises[0].clone()),
                pangle: gmse.pang as f32,
                pradius: gmse.prad as f32,
                pwidth: gmse.pwid as f32,
                drad: gmse.drad as f32,
                dwid: gmse.dwid as f32,
            })
            ))
        }
        "SPRO" => {   //structural profile
            Some(CateGeoParam::Profile(CateProfileParam::SPRO(gmse.verts)))
        }
        "BOXI" => {
            let z_length = if gmse.box_lengths.len() >= 3 {
                gmse.box_lengths[2]
            } else {
                gmse.box_lengths[1]
            };
            Some(CateGeoParam::Boxi(CateBoxImpliedParam {
                axis: Some(gmse.paxises[0].clone()),
                x_length: gmse.box_lengths[0],
                z_length,
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LCYL" => {
            // 圆柱体
            Some(CateGeoParam::LCylinder(CateLCylinderParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
                dist_to_top: gmse.distances[1],
            }))
        }
        "SCYL" => {
            // 圆柱体
            Some(CateGeoParam::SCylinder(CateSCylinderParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                height: gmse.phei,
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LINE" => {
            Some(CateGeoParam::Line(CateLineParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                diameter: 0.0, //gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "LPYR" => {
            Some(CateGeoParam::Pyramid(CatePyramidParam {
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
            Some(CateGeoParam::Snout(CateSnoutParam {
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
            Some(CateGeoParam::Box(CateBoxParam {
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
            Some(CateGeoParam::Cone(CateConeParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SCTO" => {
            // 弯管
            Some(CateGeoParam::Torus(CateTorusParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        // "SDIS" => {
        // 圆片
        // Some(CateGeoParam::Disc(CateDiscParam {
        //     axis: Some(gmse.paxises[0].clone()),
        //     dist_to_btm: gmse.distances[0],
        //     diameter: gmse.diameters[0],
        //     centre_line_flag: gmse.centre_line_flag,
        //     tube_flag: gmse.tube_flag,
        // }))
        // }
        "SDSH" => {
            Some(CateGeoParam::Dish(CateDishParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_btm: gmse.distances[0],
                height: gmse.phei,
                diameter: gmse.diameters[0],
                radius: gmse.radius,
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SEXT" => {
            Some(CateGeoParam::Extrusion(CateExtrusionParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                height: gmse.phei,
                x: gmse.xyz[0],
                y: gmse.xyz[1],
                z: gmse.xyz[2],
                verts: vec![],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SLINE" => {
            //todo
            Some(CateGeoParam::Sline(CateSlineParam {
                start_pt: vec![0.0; 3],
                end_pt: vec![0.0; 3],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SREV" => {
            Some(CateGeoParam::Revolution(CateRevolutionParam {
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
            Some(CateGeoParam::RectTorus(CateRectTorusParam {
                pa: Some(gmse.paxises[0].clone()),
                pb: Some(gmse.paxises[1].clone()),
                height: gmse.phei,
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        // "SSLC" => {
            //todo
            // Some(CateGeoParam::SlopeBottomCylinder(CateSlopeBottomCylinderParam {
            //     axis: Some(gmse.paxises[0].clone()),
            //     height: gmse.phei,
            //     diameter: gmse.diameters[0],
            //     distance: gmse.distances[0],
            //     x_shear: 0.0,
            //     y_shear: 0.0,
            //     alt_x_shear: 0.0,
            //     alt_y_shear: 0.0,
            //     centre_line_flag: gmse.centre_line_flag,
            //     tube_flag: gmse.tube_flag,
            // }))
        // }
        "SSPH" => {
            // 球
            Some(CateGeoParam::Sphere(CateSphereParam {
                axis: Some(gmse.paxises[0].clone()),
                dist_to_center: gmse.distances[0],
                diameter: gmse.diameters[0],
                centre_line_flag: gmse.centre_line_flag,
                tube_flag: gmse.tube_flag,
            }))
        }
        "SVER" => {
            Some(CateGeoParam::SVER(CateSverParam {
                x: gmse.xyz[0],
                y: gmse.xyz[1],
                radius: gmse.radius,
            }))
        }
        _ => None,
    };
    geo
}

pub fn resolve_dir_and_pos(axis: &AxisParam,
                           ddangle: f64,
                           scom: &ScomInfo,
                           context: &HashMap<SmolStr, SmolStr>) -> (Vec<f64>, Vec<f64>) {
    //替换掉中间出现dataset的值的这种情况 X ( ATTRIB RPRO ANGL ) Z
    let mut dir_str = axis.direction.trim().to_string();
    // //dbg!(&dir_str);
    if dir_str.contains("(") {
        let s: Vec<_> = dir_str.split("(").collect();
        if s.len() > 1 {
            let ss: Vec<_> = s[1].split(")").collect();
            if ss.len() > 1 {
                let val_str = ss[0];
                let val_result = eval_str_to_f64(val_str, context).unwrap_or_default().to_string();
                dir_str = dir_str.replace(val_str, &val_result);
            }
        }
    }
    let mut dir = vec![0.0f64; 3];
    let mut pos = vec![0.0f64; 3];

    let re = Regex::new(r"^P\d+$").unwrap();
    // //dbg!(&dir_str);
    if re.is_match(&dir_str) {
        let pnt_indx = dir_str[1..].parse::<i32>().unwrap_or(i32::MAX);
        // //dbg!(pnt_indx);
        if let Some(indx) = scom.axis_param_numbers.iter().position(|&x| x == pnt_indx) {
            if let Some(axis) = resolve_axis_param(&scom.axis_params[indx], scom, context) {
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
    [(v[0] as f64 * 100.0).round() / 100.0, (v[1] as f64 * 100.0).round() / 100.0, (v[2] as f64 * 100.0).round() / 100.0]
}
