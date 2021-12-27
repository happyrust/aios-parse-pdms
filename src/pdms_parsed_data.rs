use dashmap::DashMap;
use crate::pdms_origin_data::GmseParam;
use crate::pdms_types::{AttrVal, ElementData};

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignPipeRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignComponentRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignBranRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RefnosRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Refnos {
    #[prost(string, repeated, tag = "1")]
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
#[derive(Clone, Debug, Default)]
pub struct GeomsInfo {
    pub geometries: ::prost::alloc::vec::Vec<GeoParamsData>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Dataset {
    #[prost(string, tag = "1")]
    pub self_type: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GmseParamData {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub owner: ::prost::alloc::string::String,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    #[prost(string, tag = "4")]
    pub type_name: ::prost::alloc::string::String,
    #[prost(double, tag = "5")]
    pub radius: f64,
    #[prost(double, tag = "6")]
    pub angle: f64,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    #[prost(double, repeated, tag = "7")]
    pub diameters: ::prost::alloc::vec::Vec<f64>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    #[prost(double, repeated, tag = "8")]
    pub distances: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, tag = "9")]
    pub height: f64,
    #[prost(double, tag = "10")]
    pub offset: f64,
    /// 顺序 x y z
    #[prost(double, repeated, tag = "11")]
    pub box_lengths: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "12")]
    pub xyz: ::prost::alloc::vec::Vec<f64>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    #[prost(message, repeated, tag = "13")]
    pub paxises: ::prost::alloc::vec::Vec<CateAxisParam>,
    #[prost(bool, tag = "14")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "15")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateAxisParam {
    #[prost(double, repeated, tag = "1")]
    pub pt: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub dir: ::prost::alloc::vec::Vec<f64>,
    #[prost(string, tag = "3")]
    pub pconnect: ::prost::alloc::string::String,
    #[prost(double, tag = "4")]
    pub pbore: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GeoParamsData {
    #[prost(
        oneof = "geo_params_data::CateGeoParams",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17"
    )]
    pub cate_geo_params: ::core::option::Option<geo_params_data::CateGeoParams>,
}
/// Nested message and enum types in `GeoParamsData`.
pub mod geo_params_data {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum CateGeoParams {
        #[prost(message, tag = "1")]
        Boxi(super::CateBoxImpliedParam),
        #[prost(message, tag = "2")]
        Box(super::CateBoxParam),
        #[prost(message, tag = "3")]
        Cone(super::CateConeParam),
        #[prost(message, tag = "4")]
        LCylinder(super::CateLCylinderParam),
        #[prost(message, tag = "19")]
        SCylinder(super::CateSCylinderParam),

        #[prost(message, tag = "5")]
        Disc(super::CateDiscParam),
        #[prost(message, tag = "6")]
        Dish(super::CateDishParam),
        #[prost(message, tag = "7")]
        Extrusion(super::CateExtrusionParam),
        #[prost(message, tag = "8")]
        Line(super::CateLineParam),
        #[prost(message, tag = "9")]
        Pyramid(super::CatePyramidParam),
        #[prost(message, tag = "10")]
        RectTorus(super::CateRectTorusParam),
        #[prost(message, tag = "11")]
        Revolution(super::CateRevolutionParam),
        #[prost(message, tag = "12")]
        Sline(super::CateSlineParam),
        #[prost(message, tag = "13")]
        SlopeBottomCylinder(super::CateSlopeBottomCylinderParam),
        #[prost(message, tag = "14")]
        Snout(super::CateSnoutParam),
        #[prost(message, tag = "15")]
        Sphere(super::CateSphereParam),
        #[prost(message, tag = "16")]
        Torus(super::CateTorusParam),
        #[prost(message, tag = "17")]
        TubeImplied(super::CateTubeImpliedParam),
        #[prost(message, tag = "18")]
        SVER(super::CateSverParam),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateBoxImpliedParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub x_length: f64,
    #[prost(double, tag = "3")]
    pub z_length: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateBoxParam {
    #[prost(double, repeated, tag = "1")]
    pub size: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub offset: ::prost::alloc::vec::Vec<f64>,
    #[prost(bool, tag = "3")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "4")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateConeParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSCylinderParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateLCylinderParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub dist_to_top: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateExtrusionParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub x: f64,
    #[prost(double, tag = "5")]
    pub y: f64,
    #[prost(double, tag = "6")]
    pub z: f64,
    #[prost(bool, tag = "7")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "8")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateDiscParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateDishParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(double, tag = "5")]
    pub radius: f64,
    #[prost(bool, tag = "6")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "7")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateLineParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CatePyramidParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "3")]
    pub pc: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "4")]
    pub x_bottom: f64,
    #[prost(double, tag = "5")]
    pub y_bottom: f64,
    #[prost(double, tag = "6")]
    pub x_top: f64,
    #[prost(double, tag = "7")]
    pub y_top: f64,
    #[prost(double, tag = "8")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "9")]
    pub dist_to_top: f64,
    #[prost(double, tag = "10")]
    pub x_offset: f64,
    #[prost(double, tag = "11")]
    pub y_offset: f64,
    #[prost(bool, tag = "12")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "13")]
    pub tube_flag: bool,
}
/// 截面为矩形的弯管
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateRectTorusParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateRevolutionParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub angel: f64,
    #[prost(double, tag = "4")]
    pub x: f64,
    #[prost(double, tag = "5")]
    pub y: f64,
    #[prost(double, tag = "6")]
    pub z: f64,
    #[prost(bool, tag = "7")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "8")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSlineParam {
    #[prost(double, repeated, tag = "1")]
    pub start_pt: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub end_pt: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSlopeBottomCylinderParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub height: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(double, tag = "4")]
    pub distance: f64,
    #[prost(double, tag = "5")]
    pub x_shear: f64,
    #[prost(double, tag = "6")]
    pub y_shear: f64,
    #[prost(double, tag = "7")]
    pub alt_x_shear: f64,
    #[prost(double, tag = "8")]
    pub alt_y_shear: f64,
    #[prost(bool, tag = "9")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "10")]
    pub tube_flag: bool,
}
/// 圆台 或 管嘴
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSnoutParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "4")]
    pub dist_to_top: f64,
    #[prost(double, tag = "5")]
    pub btm_diameter: f64,
    #[prost(double, tag = "6")]
    pub top_diameter: f64,
    #[prost(double, tag = "7")]
    pub offset: f64,
    #[prost(bool, tag = "8")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "9")]
    pub tube_flag: bool,
}
/// 球
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSphereParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_center: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
///元件库里的torus参数
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateTorusParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateTubeImpliedParam {
    #[prost(double, repeated, tag = "1")]
    pub center_position: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub direction: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(double, tag = "4")]
    pub height: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSverParam{
    #[prost(double, tag = "1")]
    pub x: f64,
    #[prost(double, tag = "2")]
    pub y: f64,
    #[prost(double, tag = "3")]
    pub radius:f64,
}

#[derive(Debug,Default,Clone)]
pub struct SLoo{
    pub name:String,
    pub refno:String,
    pub self_type:String,
    pub owner:String,
    pub purp:String,
    pub svers:Vec<GmseParam>,
}

// impl SLoo {
//     pub fn new(e:ElementData) -> Self{
//         Self{
//             name: e.name,
//             refno: e.ref_no,
//             self_type: e.noun_name,
//             owner: e.owner,
//             purp: get_map_string_type_value(&e.attr_data_map,"PURP"),
//             svers: vec![]
//         }
//     }
//
// }



