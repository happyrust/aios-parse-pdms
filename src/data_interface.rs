use crate::pdms_types::{PdmsTree, RefU64, RefU64Vec};
use async_trait::async_trait;
use glam::{TransformRT, TransformSRT};
use id_tree::NodeId;
use smol_str::SmolStr;
use crate::{AttrMap, EleNode};
use crate::pdms_data::ScomInfo;

// #[async_trait]
pub trait PdmsDataInterface {
    fn sync_total_project(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn sync_incremental_project(&mut self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn get_ele_attr(&self, refno: RefU64) -> anyhow::Result<AttrMap>;

    fn get_ele_children_attrs(&self, refno: RefU64) -> Vec<AttrMap>;

    fn get_ele_children_refs(&self, refno: RefU64) -> RefU64Vec;

    fn get_ele_world_transform(&self, refno: RefU64) -> TransformRT;

    fn get_pdms_tree(&self, project: &str, db_no: u32) -> Option<PdmsTree>;

    fn get_node_id(&self, refno: RefU64) -> Option<NodeId>;

    fn get_name(&self, refno: RefU64) -> SmolStr;

    fn get_name_by_hash(&self, refno: RefU64, name_hash: u32) -> Option<SmolStr>;

    fn get_refnos_by_type(&self,project_name:SmolStr,att_type:&str) -> Option<RefU64Vec>;

    fn get_pdms_project_tree(&self, project:&str,main_db:u32) -> anyhow::Result<PdmsTree> ;
}