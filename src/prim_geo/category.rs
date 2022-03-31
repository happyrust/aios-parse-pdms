
use crate::shape::pdms_shape::BrepShapeTrait;
use bevy::math::TransformSRT;
use id_tree::NodeId;


#[derive(Debug)]
pub struct CateBrepShape{
    pub brep_shape: Box<dyn BrepShapeTrait>,
    pub transform: TransformSRT,
    pub visible: bool,
    pub is_tubing: bool,
}