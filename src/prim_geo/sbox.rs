use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use truck_polymesh::stl::IntoSTLIterator;
use bevy_inspector_egui::Inspectable;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use log::kv::Source;
use lyon::math::size;
use crate::prim_geo::helper::quad_indices;
use crate::AttrMap;
use crate::prim_geo::pdms_shape::{BrepMathTrait, BrepShape, ScaledShape, VerifiedShape};

#[derive(Component, Debug, Inspectable, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Component)]
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

impl ScaledShape for SBox {
    #[inline]
    fn get_scale_vec3(&self) -> Vec3 {
        self.size
    }
}

impl VerifiedShape for SBox {
    fn check_valid(&self) -> bool {
        true
    }
}

impl BrepShape for SBox {
    fn gen_brep(& self) -> Option<Shell> {
        if !self.check_valid() { return None; }
        let v = builder::vertex((self.center - self.size / 2.0).point3());
        let e = builder::tsweep(&v, Vector3::unit_x() * self.size.x as f64);
        let f = builder::tsweep(&e, Vector3::unit_y() * self.size.y as f64);
        let mut s = builder::tsweep(&f, Vector3::unit_z() * self.size.z as f64).into_boundaries();
        s.pop()
    }

    // fn gen_mesh(&self) -> Mesh{
    //     self.quick_gen_mesh().unwrap()
    // }

    fn quick_gen_mesh(&self) -> Option<Mesh>{

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        dbg!("quick gen box");

        let xp = (0.5 * self.size[0]);
        let xm = -xp;
        let yp = (0.5 * self.size[1]) ;
        let ym = -yp;
        let zp = (0.5 * self.size[2]);
        let zm = -zp;
        let verts = [
            [ [xm, ym, zp ], [xm, yp, zp ], [xm, yp, zm ], [xm, ym, zm  ]],
            [ [xp, ym, zm ], [xp, yp, zm ], [xp, yp, zp ], [xp, ym, zp  ]],
            [ [xp, ym, zm ], [xp, ym, zp ], [xm, ym, zp ], [xm, ym, zm  ]],
            [ [xm, yp, zm ], [xm, yp, zp ], [xp, yp, zp ], [xp, yp, zm  ]],
            [ [xm, yp, zm ], [xp, yp, zm ], [xp, ym, zm ], [xm, ym, zm  ]],
            [ [xm, ym, zp ], [xp, ym, zp ], [xp, yp, zp ], [xm, yp, zp  ]]
        ];
        let ns = [
            [-1.0, 0.0,  0.0 ],
            [1.0,  0.0,  0.0 ],
            [0.0, -1.0,  0.0 ],
            [0.0,  1.0,  0.0 ],
            [0.0,  0.0, -1.0 ],
            [0.0,  0.0,  1.0 ]
        ];

        let faces_n = 6;
        let vertices_n = 6 * 4;
        let mut positions = Vec::with_capacity(vertices_n);
        let mut normals = Vec::with_capacity(vertices_n);
        let triangles_n = 2 * faces_n;
        let mut indices = Vec::with_capacity(3*triangles_n);
        let mut uvs = Vec::new();

        let mut o = 0usize;
        let mut i_v = 0usize;
        let mut i_p = 0usize;
        for f in 0..6{
            for i in 0..4{
                normals.push(ns[f]);
                positions.push(verts[f][i]);
                uvs.push([0.0, 0.0]);
            }
            quad_indices(&mut indices, &mut i_p, o, 0, 1, 2, 3);
            o += 4;
        }
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U16(
            indices.into_iter().map(|x| x as u16).collect()
        )));
        Some(mesh)
    }
}


impl From<&AttrMap> for SBox {
    fn from(m: &AttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(m.get("XLEN").unwrap().double_value().unwrap() as f32 ,
                            m.get("YLEN").unwrap().double_value().unwrap() as f32 ,
                            m.get("ZLEN").unwrap().double_value().unwrap() as f32 ),
        }
    }
}


