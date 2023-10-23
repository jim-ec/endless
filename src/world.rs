use std::collections::HashMap;

use cgmath::{vec3, InnerSpace, Vector3};
use noise::NoiseFn;

use crate::{
    field::Field,
    gizmo_pass::GizmoPass,
    renderer::Renderer,
    util::{rescale, rgb},
    voxel_pass::{VoxelMesh, VoxelPipeline},
};

pub const N: usize = 64;

pub struct World {
    pub voxel_pipeline: VoxelPipeline,
    pub gizmo_pass: GizmoPass,
    pub chunks: HashMap<Vector3<isize>, Chunk>,
}

pub struct Chunk {
    pub field: Field<bool, 3>,
    pub color: Field<Vector3<f32>, 3>,
    pub voxel_mesh: VoxelMesh,
}

impl World {
    pub fn new(renderer: &mut Renderer) -> World {
        println!("Voxels: {}x{}x{} = {}", N, N, N, N.pow(3));

        let mut chunks = HashMap::new();

        let n: isize = 4;
        for x in -n..=n {
            for y in -n..=n {
                let c = vec3(x, y, 0);
                let fidelity = x.unsigned_abs() + y.unsigned_abs();
                let fidelity = fidelity >> 2;

                if N >> fidelity > 0 {
                    chunks.insert(c, Chunk::new(renderer, c, fidelity));
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
    pub fn new(renderer: &mut Renderer, translation: Vector3<isize>, fidelity: usize) -> Self {
        use noise::{Fbm, Perlin, Turbulence};
        let mut noise = Fbm::<Perlin>::new(0);
        noise.frequency = 0.01;
        let noise = Turbulence::<_, Perlin>::new(noise);

        let extent = N >> fidelity;
        let scale = 1 << fidelity;

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

        let rock_height_map: Field<f32, 2> = Field::new("Rock Height Map", extent, |[x, y]| {
            let mut n =
                noise.get([scale as f64 * x as f64 + t.x, scale as f64 * y as f64 + t.y]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.9);
            n = n.powf(1.5);
            n * extent as f32
        });

        // TODO: Implement gradient as a kernel on fields
        let rock_gradient_map: Field<Vector3<f32>, 2> =
            Field::new("Rock Normal Map", extent, |[x, y]| {
                let dx = rock_height_map[[(x + 1).min(extent - 1), y]]
                    - rock_height_map[[x.saturating_sub(1), y]];
                let dy = rock_height_map[[x, (y + 1).min(extent - 1)]]
                    - rock_height_map[[x, y.saturating_sub(1)]];
                vec3(dx, dy, 1.0).normalize()
            });

        let sigma = 3.0;
        let gaussian = |x: f32| {
            let a = 1.0 / (std::f32::consts::TAU * sigma * sigma).sqrt();
            let b = -x * x / (2.0 * sigma * sigma);
            a * b.exp()
        };
        let mut kernel = vec![0.0; 6 * sigma.ceil() as usize + 1];
        for (x, kernel_entry) in kernel.iter_mut().enumerate() {
            *kernel_entry = gaussian(x as f32 - 3.0 * sigma);
        }

        let blur_x: Field<f32, 2> = Field::new("Blur x", extent, |[x, y]| {
            let mut acc = 0.0;
            for i in 0..kernel.len() {
                let x = x as isize + i as isize - kernel.len() as isize / 2;
                let x = x.clamp(0, extent as isize - 1);
                acc += kernel[i] * rock_gradient_map[[x as usize, y]].z;
            }
            acc
        });
        let blur_xy: Field<f32, 2> = Field::new("Blur y", extent, |[x, y]| {
            let mut acc = 0.0;
            for i in 0..kernel.len() {
                let y = y as isize + i as isize - kernel.len() as isize / 2;
                let y = y.clamp(0, extent as isize - 1);
                acc += kernel[i] * blur_x[[x, y as usize]];
            }
            acc
        });

        let sediments: Field<Sediment, 3> = Field::new("Sediments", extent, |[x, y, z]| {
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

        let field: Field<bool, 3> = Field::new("Field", extent, |[x, y, z]| {
            sediments[[x, y, z]] != Sediment::Air
        });

        let color = sediments.map("Color", |s| match s {
            Sediment::Rock => rgb(50, 40, 50),
            Sediment::Grass => rgb(120, 135, 5),
            Sediment::Air => rgb(0, 0, 0),
        });

        let env = field.environment();

        Self {
            voxel_mesh: VoxelMesh::new(
                renderer,
                &field.shell(&env),
                &env.visibility(),
                &color,
                N as isize * translation,
                scale,
            ),
            field,
            color,
        }
    }
}
