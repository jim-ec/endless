use crate::{
    field::Field,
    renderer::{self, RenderPass, DEPTH_FORMAT, SAMPLES},
};
use cgmath::{vec3, InnerSpace, Vector3};
use wgpu::util::DeviceExt;

pub struct VoxelPipeline {
    pipeline: wgpu::RenderPipeline,
}

#[derive(Debug)]
pub struct VoxelMesh {
    position_buffer: wgpu::Buffer,
    normal_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    voxel_count: usize,
}

pub struct VoxelPass<'a>(pub &'a VoxelPipeline, pub &'a VoxelMesh);

impl VoxelPipeline {
    pub fn new(renderer: &renderer::Renderer) -> Self {
        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&renderer.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        bind_group_layouts: &[&renderer.bind_group_layout],
                        ..Default::default()
                    },
                )),
                vertex: wgpu::VertexState {
                    module: &renderer.shader,
                    entry_point: "voxel_vertex",
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vector3<f32>>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x4,
                            }],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vector3<f32>>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x4,
                            }],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vector3<f32>>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            }],
                        },
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &renderer.shader,
                    entry_point: "voxel_fragment",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.config.format,
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
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multiview: None,
            });

        Self { pipeline }
    }
}

impl VoxelMesh {
    pub fn new(
        renderer: &renderer::Renderer,
        field: &Field<bool, 3>,
        color: &Field<Vector3<f32>, 3>,
        translation: Vector3<isize>,
        scale: isize,
    ) -> Self {
        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut colors: Vec<Vector3<f32>> = Vec::new();
        let mut normals: Vec<Vector3<f32>> = Vec::new();

        let mut instance_vertices = vec![];
        let mut instance_normals = vec![];
        for [i, j, k] in CUBE_VERTEX_INDICES {
            let vs = [
                scale as f32 * CUBE_VERTICES[i as usize],
                scale as f32 * CUBE_VERTICES[j as usize],
                scale as f32 * CUBE_VERTICES[k as usize],
            ];
            instance_vertices.extend(vs);
            instance_normals.extend(
                std::iter::repeat((vs[2] - vs[0]).cross(vs[1] - vs[0]).normalize()).take(3),
            );
        }

        for [x, y, z] in field.coordinates() {
            if field[[x, y, z]] {
                let position = scale as f32 * vec3(x as f32, y as f32, z as f32)
                    + vec3(
                        translation.x as f32,
                        translation.y as f32,
                        translation.z as f32,
                    );
                for i in 0..instance_vertices.len() {
                    positions.push(position + instance_vertices[i]);
                    normals.push(instance_normals[i]);
                    colors.push(color[[x, y, z]]);
                }
            }
        }

        let position_buffer =
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

        let normal_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: unsafe {
                    std::slice::from_raw_parts(
                        normals.as_ptr() as *const u8,
                        normals.len() * std::mem::size_of::<Vector3<f32>>(),
                    )
                },
                usage: wgpu::BufferUsages::VERTEX,
            });

        let color_buffer = renderer
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

        Self {
            position_buffer,
            normal_buffer,
            color_buffer,
            voxel_count: colors.len(),
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

impl<'a> RenderPass for VoxelPass<'a> {
    fn render<'p: 'r, 'r>(&'p self, _queue: &wgpu::Queue, render_pass: &mut wgpu::RenderPass<'r>) {
        let VoxelPass(pipeline, mesh) = self;
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_vertex_buffer(0, mesh.position_buffer.slice(..));
        render_pass.set_vertex_buffer(1, mesh.normal_buffer.slice(..));
        render_pass.set_vertex_buffer(2, mesh.color_buffer.slice(..));
        render_pass.draw(0..mesh.voxel_count as u32, 0..1);
    }
}
