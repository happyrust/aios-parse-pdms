use std::collections::BTreeMap;

use dashmap::DashMap;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_data::GmseParam;
use crate::pdms_types::{AttrVal, EleNode};
use serde_derive::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Clone, PartialEq, Debug)]
pub struct DesignPipeRequest {
    
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct DesignComponentRequest {
    
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct DesignBranRequest {
    
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct RefnosRequest {
    
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Refnos {
    
    pub refnos: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, Debug, Default)]
pub struct DesignPipe {
    pub name: ::prost::alloc::string::String,
    pub refno: ::prost::alloc::string::String,
    pub brans: ::prost::alloc::vec::Vec<DesignBran>,
}
#[derive(Clone, Debug, Default)]
pub struct DesignBran {
    pub name: ::prost::alloc::string::String,
    pub refno: ::prost::alloc::string::String,
    pub components: ::prost::alloc::vec::Vec<GeomsInfo>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeomsInfo {
    pub geometries: Vec<CateGeoParam>,
    pub axis_map: BTreeMap<i32, CateAxisParam>,
    pub tubi_bore: Option<f32>,
    // pub matrix: glam::f32::Affine3A
}

#[derive(Clone, PartialEq, Debug)]
pub struct Dataset {
    
    pub self_type: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq)]
pub struct GmseParamData {
    pub name: SmolStr,
    pub refno: SmolStr,
    pub owner: SmolStr,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub type_name: SmolStr,
    pub radius: f64,
    pub angle: f64,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: ::prost::alloc::vec::Vec<f64>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: ::prost::alloc::vec::Vec<f64>,
    pub height: f64,
    pub offset: f64,
    /// 顺序 x y z
    pub box_lengths: ::prost::alloc::vec::Vec<f64>,
    pub xyz: ::prost::alloc::vec::Vec<f64>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: ::prost::alloc::vec::Vec<CateAxisParam>,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateAxisParam {
    pub pt: ::prost::alloc::vec::Vec<f64>,
    pub dir: ::prost::alloc::vec::Vec<f64>,
    pub pconnect: ::prost::alloc::string::String,
    pub pbore: f64,
}


pub mod geo_params_data {
    #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
    pub enum CateGeoParam {
        Boxi(super::CateBoxImpliedParam),
        Box(super::CateBoxParam),
        Cone(super::CateConeParam),
        LCylinder(super::CateLCylinderParam),
        SCylinder(super::CateSCylinderParam),

        Disc(super::CateDiscParam),
        Dish(super::CateDishParam),
        Extrusion(super::CateExtrusionParam),
        Line(super::CateLineParam),
        Pyramid(super::CatePyramidParam),
        RectTorus(super::CateRectTorusParam),
        Revolution(super::CateRevolutionParam),
        Sline(super::CateSlineParam),
        SlopeBottomCylinder(super::CateSlopeBottomCylinderParam),
        Snout(super::CateSnoutParam),
        Sphere(super::CateSphereParam),
        Torus(super::CateTorusParam),
        TubeImplied(super::CateTubeImpliedParam),
        SVER(super::CateSverParam),
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize,  Debug)]
pub struct CateBoxImpliedParam {
    pub axis: ::core::option::Option<CateAxisParam>,
    pub x_length: f64,
    pub z_length: f64,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateBoxParam {
    
    pub size: ::prost::alloc::vec::Vec<f64>,
    
    pub offset: ::prost::alloc::vec::Vec<f64>,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateConeParam {
    pub axis: ::core::option::Option<CateAxisParam>,
    pub dist_to_btm: f64,
    pub diameter: f64,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSCylinderParam {
    
    pub axis: ::core::option::Option<CateAxisParam>,
    
    pub dist_to_btm: f64,
    
    pub height: f64,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize,Debug)]
pub struct CateLCylinderParam {
    
    pub axis: ::core::option::Option<CateAxisParam>,
    
    pub dist_to_btm: f64,
    
    pub dist_to_top: f64,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateExtrusionParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub height: f64,
    
    pub x: f64,
    
    pub y: f64,
    
    pub z: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateDiscParam {
    
    pub axis: ::core::option::Option<CateAxisParam>,
    
    pub dist_to_btm: f64,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize,Debug)]
pub struct CateDishParam {
    
    pub axis: ::core::option::Option<CateAxisParam>,
    
    pub dist_to_btm: f64,
    
    pub height: f64,
    
    pub diameter: f64,
    
    pub radius: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize,Debug)]
pub struct CateLineParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CatePyramidParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub pc: ::core::option::Option<CateAxisParam>,
    
    pub x_bottom: f64,
    
    pub y_bottom: f64,
    
    pub x_top: f64,
    
    pub y_top: f64,
    
    pub dist_to_btm: f64,
    
    pub dist_to_top: f64,
    
    pub x_offset: f64,
    
    pub y_offset: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
/// 截面为矩形的弯管
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateRectTorusParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub height: f64,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateRevolutionParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub angel: f64,
    
    pub x: f64,
    
    pub y: f64,
    
    pub z: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSlineParam {
    
    pub start_pt: ::prost::alloc::vec::Vec<f64>,
    
    pub end_pt: ::prost::alloc::vec::Vec<f64>,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSlopeBottomCylinderParam {
    
    pub axis: ::core::option::Option<CateAxisParam>,
    
    pub height: f64,
    
    pub diameter: f64,
    
    pub distance: f64,
    
    pub x_shear: f64,
    
    pub y_shear: f64,
    
    pub alt_x_shear: f64,
    
    pub alt_y_shear: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
/// 圆台 或 管嘴
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSnoutParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub dist_to_btm: f64,
    
    pub dist_to_top: f64,
    
    pub btm_diameter: f64,
    
    pub top_diameter: f64,
    
    pub offset: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
/// 球
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSphereParam {
    
    pub axis: ::core::option::Option<CateAxisParam>,
    
    pub dist_to_center: f64,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}
///元件库里的torus参数
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateTorusParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateTubeImpliedParam {
    
    pub center_position: ::prost::alloc::vec::Vec<f64>,
    
    pub direction: ::prost::alloc::vec::Vec<f64>,
    
    pub diameter: f64,
    
    pub height: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSverParam{
    
    pub x: f64,
    
    pub y: f64,
    
    pub radius:f64,
}



