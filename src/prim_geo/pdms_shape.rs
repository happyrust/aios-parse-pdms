use std::fmt::Debug;
use bevy::prelude::{Mesh, Vec3};
use bevy::prelude::FromWorld;
use truck_modeling::{Curve, Shell};
// use bevy_inspector_egui::Inspectable;
use bevy::ecs::component::Component;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use truck_base::cgmath64::{Point3, Vector3};
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use bevy::reflect::{Reflect, ReflectRef};
use bevy::ecs::reflect::ReflectComponent;
use ncollide3d::bounding_volume::AABB;
use ncollide3d::math::{Point, Vector};
use ncollide3d::na;
use truck_base::bounding_box::BoundingBox;
use crate::prim_geo::cylinder::{LCylinder, SCylinder};
use crate::prim_geo::sbox::SBox;

pub const TRIANGLE_TOL: f64 = 0.01;

pub trait VerifiedShape{
    fn check_valid(&self) -> bool{
        true
    }
}

#[inline]
pub fn gen_bounding_box(shell: &Shell) -> BoundingBox<Point3>{
    let mut bdd_box = BoundingBox::new();
    shell
        .iter()
        .flat_map(truck_modeling::Face::boundaries)
        .flatten()
        .for_each(|edge| {
            let curve = edge.oriented_curve();
            bdd_box += match curve {
                Curve::BSplineCurve(curve) => {
                    let bdb = curve.roughly_bounding_box();
                    vec![*bdb.max(), *bdb.min()].into_iter().collect()
                }
                Curve::NURBSCurve(curve) => curve.roughly_bounding_box(),
                Curve::IntersectionCurve(_) => BoundingBox::new(),
            };
        });
    bdd_box
    // let (size, center) = (bdd_box.size(), bdd_box.center());
}

#[derive(Serialize, Deserialize, Component, Debug, Clone, Default)]
pub struct PdmsMesh{
    // pub mesh: Mesh,
    pub indices: Vec<u32>,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub aabb: (Vec3, Vec3),
}

pub trait BrepShape : VerifiedShape + Debug{

    fn gen_brep(&self) -> Option<Shell>;

    //todo 实现模型的hash，主要是看比列
    //通过比例缩放可以更大的共享几何信息
    fn hash_mesh_params(&self) -> u64{
        0
    }

    //生成对应的单位长度的模型，比如Dish，就是以R为1的情况生成模型
    fn gen_unit_shape(&self) -> PdmsMesh{
        PdmsMesh::default()
    }

    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::ONE
    }



    //直接使用基本体的快速生成
    fn quick_gen_mesh(&self) -> Option<Mesh>{
        None
    }

    fn gen_mesh(&self, tol: Option<f32>) -> PdmsMesh{
        // let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let mut aabb = AABB::new_invalid();
        if let Some(brep) = self.gen_brep() {
            let brep_bbox = gen_bounding_box(&brep);
            let (size, c) = (brep_bbox.size(), brep_bbox.center());
            let d = brep_bbox.diagonal() / 2.0;
            aabb = AABB::from_half_extents(
                Point::<f32>::new(c[0] as f32, c[1] as f32, c[2] as f32),
                Vector::<f32>::new(d[0] as f32, d[1] as f32, d[2] as f32)
            );
            if size <= f64::EPSILON{
                return PdmsMesh::default();
            }
            let tolerance = tol.unwrap_or((TRIANGLE_TOL * size) as f32) as f64;
            if let Some(s) = brep.triangulation(tolerance) {
                let polygon = s.to_polygon();
                let vertices = polygon.positions().iter().map(|&x| x.array()).collect::<Vec<_>>();
                let normals = polygon.normals().iter().map(|&x| x.array()).collect::<Vec<_>>();
                let uvs = polygon.uv_coords().iter().map(|x| [x[0] as f32, x[1] as f32]).collect::<Vec<_>>();
                let mut indices = vec![];
                for i in polygon.tri_faces(){
                    indices.push(i[0].pos as u32);
                    indices.push(i[1].pos as u32);
                    indices.push(i[2].pos as u32);
                }
                // mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                // mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                // mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                // mesh.set_indices(Some(Indices::U16(
                //     indices
                // )));
                let a = aabb.mins;
                let b = aabb.maxs;
                return  PdmsMesh{
                    indices,
                    vertices,
                    normals,
                    aabb: (Vec3::new(a.x, a.y, a.z), Vec3::new(b.x, b.y, b.z))
                };
            }
        }
        PdmsMesh::default()
    }
}

pub trait ScaledShape{
    fn get_scale_vec3(&self) -> Vec3;
}

pub trait BrepMathTrait{
    fn vector3(&self) -> Vector3;
    fn point3(&self) -> Point3;
}

impl BrepMathTrait for Vec3 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }
}

pub trait BevyMathTrait{
    fn vec3(&self) -> Vec3;
    fn array(&self) -> [f32; 3];
}

impl BevyMathTrait for Vector3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

impl BevyMathTrait for Point3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}


#[derive(Component, Debug, /*Inspectable,*/ Clone, Serialize, Deserialize,)]
// #[reflect(Component)]
pub enum PdmsPrimShape {
    SBoxShape(SBox),
    // SphereShape(SSphere),
    LCylinderShape(LCylinder),
    SCylinderShape(SCylinder),
    // CTorusShape(CTorus),
    // SCTorusShape(SCTorus),
    // DishShape(Dish),
    // FacetShape(Facet),
    // SRTorusShape(SRTorus),
    // LSnoutShape(LSnout),
    // TubiShape(Tubi),
    // PyramidShape(LPyramid),
    // ExtruShape(Extrusion),
}

impl Default for PdmsPrimShape {
    fn default() -> Self {
        PdmsPrimShape::SBoxShape(SBox::default())
    }
}

impl PdmsPrimShape {
    pub fn gen_mesh(& self) -> PdmsMesh {
        match self {
            PdmsPrimShape::SBoxShape(s) => s.gen_mesh(None),
            PdmsPrimShape::SCylinderShape(s) => s.gen_mesh(None),
            PdmsPrimShape::LCylinderShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::CTorusShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::SCTorusShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::DishShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::FacetShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::SRTorusShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::LSnoutShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::TubiShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::ExtruShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::PyramidShape(s) => s.gen_mesh(None),
            // PdmsPrimShape::SphereShape(s) => s.gen_mesh(None),
        }
    }
}