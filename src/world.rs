use std::collections::HashMap;

use cgmath::{vec3, Vector3};
use noise::NoiseFn;

use crate::{
    field::Field,
    gizmo_pass::GizmoPass,
    renderer::Renderer,
    util::{profile, rescale, rgb},
    voxel_pass::{VoxelMesh, VoxelPipeline},
};

pub const N: usize = 64;

pub struct World {
    pub voxel_pipeline: VoxelPipeline,
    pub gizmo_pass: GizmoPass,
    pub chunks: HashMap<Vector3<isize>, Chunk>,
}

pub struct Chunk {
    pub mask: Field<bool, 3>,
    pub color: Field<Vector3<f32>, 3>,
    pub mesh: VoxelMesh,
}

impl World {
    pub fn new(renderer: &mut Renderer) -> World {
        println!("Voxels: {}x{}x{} = {}", N, N, N, N.pow(3));

        let mut chunks = HashMap::new();

        let n: isize = 0;
        for x in -n..=n {
            for y in -n..=n {
                for z in 0..=0 {
                    let c = vec3(x, y, z);
                    let lod = x.unsigned_abs() + y.unsigned_abs();
                    // let lod = lod >> 2;

                    if (N >> lod) > 0 {
                        profile(&format!("Chunk ({x:+},{y:+},{:+}) @{lod}", 0), || {
                            chunks.insert(c, Chunk::new(renderer, c, lod));
                        });
                    }
                }
            }
        }

        World {
            voxel_pipeline: VoxelPipeline::new(renderer),
            gizmo_pass: GizmoPass::new(renderer),
            chunks,
        }
    }
}

impl Chunk {
    pub fn new(renderer: &mut Renderer, translation: Vector3<isize>, lod: usize) -> Self {
        use noise::{Fbm, Perlin, Turbulence};
        let mut noise = Fbm::<Perlin>::new(0);
        noise.frequency = 0.01;
        let noise = Turbulence::<_, Perlin>::new(noise);

        let extent = N >> lod;
        let scale = 1 << lod;

        let t = N as f64
            * vec3(
                translation.x as f64,
                translation.y as f64,
                translation.z as f64,
            );

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Sediment {
            Rock,
            Grass,
            Air,
        }

        let rock_height_map: Field<f32, 2> = Field::new(extent, |[x, y]| {
            let mut n =
                noise.get([scale as f64 * x as f64 + t.x, scale as f64 * y as f64 + t.y]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..1.0);
            n = n.powf(1.5);
            n -= translation.z as f32;
            n * extent as f32
        });

        let rock_normal_map: Field<Vector3<f32>, 2> = rock_height_map.normal();

        let blur_xy = rock_normal_map.map(|v| v.z).blur(3.0);

        let sediments: Field<Sediment, 3> = Field::new(extent, |[x, y, z]| {
            let flatteness = blur_xy[[x, y]];

            let rock = rock_height_map[[x, y]];
            let grass = 3.0 * flatteness;
            let n =
                noise.get([scale as f64 * x as f64 + t.x, scale as f64 * y as f64 + t.y]) as f32;
            let m = rescale(n, -1.0..1.0, 0.5..1.0);
            let grass = grass * m;

            let z = z as f32;
            if z <= rock.ceil() {
                Sediment::Rock
            } else if z <= rock.ceil() + grass {
                Sediment::Grass
            } else {
                Sediment::Air
            }
        });

        let field: Field<bool, 3> =
            Field::new(extent, |[x, y, z]| sediments[[x, y, z]] != Sediment::Air);

        let color = sediments.map(|s| match s {
            Sediment::Rock => rgb(50, 40, 50),
            Sediment::Grass => rgb(120, 135, 5),
            Sediment::Air => rgb(0, 0, 0),
        });

        let env = field.environment();

        Self {
            mesh: VoxelMesh::new(
                renderer,
                &field.shell(&env),
                &env.visibility(),
                &color,
                N as f32 * translation.cast().unwrap(),
                scale as f32,
            ),
            mask: field,
            color,
        }
    }
}
