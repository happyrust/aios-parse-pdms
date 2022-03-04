use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use truck_polymesh::stl::IntoSTLIterator;
// use bevy_inspector_egui::Inspectable;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use log::kv::Source;
use lyon::math::size;
use crate::prim_geo::helper::quad_indices;
use crate::AttrMap;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShape, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, /*Inspectable, Reflect,*/ Clone, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct SBox {
    pub center: Vec3,
    pub size: Vec3,
}

impl Default for SBox {
    fn default() -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}



impl VerifiedShape for SBox {
    fn check_valid(&self) -> bool {
        true
    }
}

impl BrepShape for SBox {

    fn hash_mesh_params(&self) -> u64{
        1u64            //代表BOX
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        SBox::default().gen_mesh(None)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        self.size
    }

    fn gen_brep(& self) -> Option<Shell> {
        if !self.check_valid() { return None; }
        let v = builder::vertex((self.center - self.size / 2.0).point3());
        let e = builder::tsweep(&v, Vector3::unit_x() * self.size.x as f64);
        let f = builder::tsweep(&e, Vector3::unit_y() * self.size.y as f64);
        let mut s = builder::tsweep(&f, Vector3::unit_z() * self.size.z as f64).into_boundaries();
        s.pop()
    }
}


impl From<&AttrMap> for SBox {
    fn from(m: &AttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(m.get_val("XLEN").unwrap().double_value().unwrap() as f32,
                            m.get_val("YLEN").unwrap().double_value().unwrap() as f32,
                            m.get_val("ZLEN").unwrap().double_value().unwrap() as f32 ),
        }
    }
}

impl From<AttrMap> for SBox {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}



