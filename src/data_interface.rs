use crate::pdms_types::{PdmsTree, RefU64, RefU64Vec};
use async_trait::async_trait;
use glam::{TransformRT, TransformSRT};
use smol_str::SmolStr;
use crate::{AttrMap, EleNode};
use crate::pdms_data::ScomInfo;

#[async_trait]
pub trait PdmsDataInterface{

    async fn get_ele_attr_async(&self, refno: &RefU64) -> Option<AttrMap>;

    async fn get_ele_children_attrs_async(&self, refno: &RefU64) -> Vec<AttrMap>;

    async fn get_ele_children_refs_async(&self, refno: &RefU64) -> RefU64Vec;

    async fn get_ele_world_transform_async(&self, refno: &RefU64) -> TransformRT;

    fn get_tree(&self, project: &str, db_no: u32) -> Option<PdmsTree>;

    fn get_name(&self, refno: &RefU64) -> SmolStr;

    fn get_name_by_hash(&self, refno: &RefU64, name_hash: u32) -> Option<SmolStr>;

    //todo get_foreign_atr
    //
}