use crate::pdms_types::RefU64;
use async_trait::async_trait;
use crate::AttrMap;
use crate::pdms_data::ScomInfo;

#[async_trait]
pub trait PdmsDataInterface{

    async fn get_ele_attr(&self, refno: &RefU64) -> Option<AttrMap>;

    async fn get_children_attrs(&self, refno: &RefU64) -> Vec<AttrMap>;
}