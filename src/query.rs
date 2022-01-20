use std::collections::BTreeMap;
use smol_str::SmolStr;
use crate::interface::pdms_interface_new::{MResult, PdmsInterface};
use crate::pdms_data::AxisParam;
use crate::pdms_types::AttrMap;

// pub async fn query_scom_info(refno: &SmolStr, interface: &mut PdmsInterface) ->MResult<()>{
//     if let Some(attr_map)=interface.get_ele_attr_map_async(refno).await?{
//         let ptre_refno = attr_map.get_as_string("PTRE").unwrap_or_default();
//         let mut axis_params = vec![];
//         let mut axis_param_numbers = vec![];
//         if let Some(ptre_am) = interface.get_ele_attr_map_async(&ptre_refno).await? {
//
//         }
//     }
//     Ok(())
// }

// pub fn query_axis_params(attr_map:AttrMap,interface:&mut PdmsInterface) ->MResult<BTreeMap<i32, AxisParam>>{
//     // 查找ptse
//     let mut map = BTreeMap::new();
//     let refno = attr_map.get_refno();
//     let children = interface
//         .get_children_attr_map_async(refno.as_str())
//         .await?;
//     for child in children {
//         let number = child.get_as_string("NUMB").unwrap_or_default().parse::<i32>().unwrap_or(-1);
//         map.entry(number).or_insert(get_axis_param(&child));
//     }
//     Ok(map)
// }