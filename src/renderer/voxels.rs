use crate::{
    field::{Field, Vis},
    symmetry::Symmetry,
};
use cgmath::{vec3, InnerSpace, Matrix4, Quaternion, Vector3};
use wgpu::util::DeviceExt;

pub struct VoxelPipeline {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
}

/// Uniforms for one chunk.
/// Since we allocate a single buffer for chunk uniforms, the alignment
/// needs to conform to [`wgpu::Limits::min_uniform_buffer_offset_alignment`].
#[repr(C)]
#[repr(align(256))]
#[derive(Copy, Clone, Debug)]
pub(super) struct Uniforms {
    pub model: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub light: Vector3<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

#[derive(Debug)]
pub struct VoxelMesh {
    pub symmetry: Symmetry,
    pub(super) buffer: wgpu::Buffer,
    pub(super) count: usize,
}

struct Vertex {
    position: Vector3<f32>,
    normal: Vector3<f32>,
    color: u32,
}

impl VoxelPipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/voxel.wgsl"),
                    include_str!("shaders/util.wgsl"),
                )
                .into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[&bind_group_layout],
                    ..Default::default()
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: memoffset::offset_of!(Vertex, position) as wgpu::BufferAddress,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: memoffset::offset_of!(Vertex, normal) as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: memoffset::offset_of!(Vertex, color) as wgpu::BufferAddress,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Uint32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multiview: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

impl VoxelMesh {
    pub fn new(
        device: &wgpu::Device,
        mask: &Field<bool, 3>,
        vis: &Field<Vis, 3>,
        color: &Field<Vector3<f32>, 3>,
        translation: Vector3<f32>,
        scale: f32,
    ) -> Self {
        let mut vertices: Vec<Vertex> = Vec::new();

        for [x, y, z] in mask.coordinates() {
            if mask[[x, y, z]] {
                let position = vec3(x as f32, y as f32, z as f32);
                let mut faces = Vec::with_capacity(6);
                if x == 0 || vis[[x, y, z]].contains(Vis::XN) {
                    faces.push(CUBE_FACE_X_0);
                }
                if x == vis.extent() - 1 || vis[[x, y, z]].contains(Vis::XP) {
                    faces.push(CUBE_FACE_X_1);
                }
                if y == 0 || vis[[x, y, z]].contains(Vis::YN) {
                    faces.push(CUBE_FACE_Y_0);
                }
                if y == vis.extent() - 1 || vis[[x, y, z]].contains(Vis::YP) {
                    faces.push(CUBE_FACE_Y_1);
                }
                if z == 0 || vis[[x, y, z]].contains(Vis::ZN) {
                    faces.push(CUBE_FACE_Z_0);
                }
                if z == vis.extent() - 1 || vis[[x, y, z]].contains(Vis::ZP) {
                    faces.push(CUBE_FACE_Z_1);
                }
                for face in faces {
                    for [i, j, k] in face {
                        let vs = [
                            CUBE_VERTICES[i as usize],
                            CUBE_VERTICES[j as usize],
                            CUBE_VERTICES[k as usize],
                        ];
                        let normal = (vs[2] - vs[0]).cross(vs[1] - vs[0]).normalize();
                        let color = color[[x, y, z]];
                        let color = ((color.x * 255.0) as u32) << 16
                            | ((color.y * 255.0) as u32) << 8
                            | ((color.z * 255.0) as u32);
                        vertices.extend(vs.into_iter().map(|v| Vertex {
                            position: position + v,
                            normal,
                            color,
                        }));
                    }
                }
            }
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertices.len() * std::mem::size_of::<Vertex>(),
                )
            },
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            symmetry: Symmetry {
                rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                translation,
                scale,
            },
            buffer,
            count: vertices.len(),
        }
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

const CUBE_FACE_X_0: [[u16; 3]; 2] = [[6, 2, 0], [6, 0, 4]];
const CUBE_FACE_X_1: [[u16; 3]; 2] = [[5, 1, 3], [5, 3, 7]];
const CUBE_FACE_Y_0: [[u16; 3]; 2] = [[4, 0, 1], [4, 1, 5]];
const CUBE_FACE_Y_1: [[u16; 3]; 2] = [[7, 3, 2], [7, 2, 6]];
const CUBE_FACE_Z_0: [[u16; 3]; 2] = [[0, 2, 3], [0, 3, 1]];
const CUBE_FACE_Z_1: [[u16; 3]; 2] = [[4, 5, 7], [4, 7, 6]];
