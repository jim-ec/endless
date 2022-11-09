use std::time::Instant;

use crate::{
    raster::{Raster, HEIGHT, WIDTH},
    renderer,
    transform::Transform,
    util::{perf, rgb},
};
use cgmath::{vec3, Matrix4, Vector3};
use derive_setters::Setters;
use wgpu::util::DeviceExt;

#[derive(Debug, Setters)]
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub voxel_position_buffer: wgpu::Buffer,
    pub voxel_color_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub voxel_count: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MeshUniforms {
    transform: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for MeshUniforms {}
unsafe impl bytemuck::Zeroable for MeshUniforms {}

impl Mesh {
    pub fn new(renderer: &renderer::Renderer, raster: &Raster<bool>) -> Mesh {
        let mut vertices: Vec<Vector3<f32>> = Vec::new();
        for [i, j, k] in CUBE_VERTEX_INDICES {
            vertices.push(CUBE_VERTICES[i as usize]);
            vertices.push(CUBE_VERTICES[j as usize]);
            vertices.push(CUBE_VERTICES[k as usize]);
        }

        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut colors: Vec<Vector3<f32>> = Vec::new();

        perf("Voxel mesh generation", || {
            for x in 0..WIDTH {
                for y in 0..WIDTH {
                    for z in 0..HEIGHT {
                        if raster[(x, y, z)] {
                            positions.push(vec3(x as f32, y as f32, z as f32));
                            colors.push(match z {
                                0..=1 => vec3(0.2, 0.2, 1.0),
                                2..=4 => rgb(194, 178, 128),
                                _ => rgb(91, 135, 49),
                            });
                        }
                    }
                }
            }
        });

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: unsafe {
                    std::slice::from_raw_parts(
                        vertices.as_ptr() as *const u8,
                        vertices.len() * std::mem::size_of::<Vector3<f32>>(),
                    )
                },
                usage: wgpu::BufferUsages::VERTEX,
            });

        let voxel_position_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: unsafe {
                        std::slice::from_raw_parts(
                            positions.as_ptr() as *const u8,
                            positions.len() * std::mem::size_of::<Vector3<f32>>(),
                        )
                    },
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let voxel_color_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: unsafe {
                        std::slice::from_raw_parts(
                            colors.as_ptr() as *const u8,
                            colors.len() * std::mem::size_of::<Vector3<f32>>(),
                        )
                    },
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let uniform_buffer;
        {
            let unpadded_size = std::mem::size_of::<MeshUniforms>();
            let align_mask = 0xf - 1;
            let padded_size = (unpadded_size + align_mask) & !align_mask;

            uniform_buffer = renderer.device.create_buffer(
                &(wgpu::BufferDescriptor {
                    label: None,
                    size: padded_size as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            );
        }

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &renderer.mesh_uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

        Self {
            vertex_buffer,
            voxel_position_buffer,
            voxel_color_buffer,
            bind_group,
            uniform_buffer,
            voxel_count: colors.len(),
        }
    }

    pub fn upload_uniforms(&self, queue: &wgpu::Queue, frame: Transform) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[MeshUniforms {
                transform: frame.matrix(),
            }]),
        );
    }
}

const CUBE_VERTICES: [Vector3<f32>; 8] = [
    vec3(0.0, 0.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(1.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0),
    vec3(1.0, 0.0, 1.0),
    vec3(0.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
];

const CUBE_VERTEX_INDICES: [[u16; 3]; 12] = [
    [0, 2, 3],
    [0, 3, 1],
    [4, 5, 7],
    [4, 7, 6],
    [4, 0, 1],
    [4, 1, 5],
    [5, 1, 3],
    [5, 3, 7],
    [7, 3, 2],
    [7, 2, 6],
    [6, 2, 0],
    [6, 0, 4],
];
