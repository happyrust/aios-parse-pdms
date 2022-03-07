use crate::pdms_types::{RefU64, RefU64Vec};
use async_trait::async_trait;
use glam::{TransformRT, TransformSRT};
use crate::AttrMap;
use crate::pdms_data::ScomInfo;

#[async_trait]
pub trait PdmsDataInterface{

    async fn get_ele_attr(&self, refno: &RefU64) -> Option<AttrMap>;

    async fn get_ele_children_attrs(&self, refno: &RefU64) -> Vec<AttrMap>;

    async fn get_ele_children_refs(&self, refno: &RefU64) -> RefU64Vec;

    async fn get_ele_world_transform(&self, refno: &RefU64) -> TransformRT;
}