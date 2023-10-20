use crate::{
    grid::{self, Grid},
    renderer::{self, RenderPass, DEPTH_FORMAT, SAMPLES},
};
use cgmath::{vec3, Vector3};
use derive_setters::Setters;
use wgpu::util::DeviceExt;

#[derive(Debug, Setters)]
pub struct VoxelPass {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_normal_buffer: wgpu::Buffer,
    voxel_position_buffer: wgpu::Buffer,
    voxel_color_buffer: wgpu::Buffer,
    voxel_count: usize,
}

impl VoxelPass {
    pub fn new(
        renderer: &renderer::Renderer,
        grid: &Grid<bool>,
        color: &Grid<Vector3<f32>>,
    ) -> VoxelPass {
        let mut vertices: Vec<Vector3<f32>> = Vec::new();
        let mut vertex_normals: Vec<Vector3<f32>> = Vec::new();
        for [i, j, k] in CUBE_VERTEX_INDICES {
            let vs = [
                CUBE_VERTICES[i as usize],
                CUBE_VERTICES[j as usize],
                CUBE_VERTICES[k as usize],
            ];
            vertices.extend(vs);
            vertex_normals.extend(std::iter::repeat((vs[2] - vs[0]).cross(vs[1] - vs[0])).take(3));
        }

        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut colors: Vec<Vector3<f32>> = Vec::new();

        for [x, y, z] in grid::coordinates() {
            if grid[[x, y, z]] {
                positions.push(vec3(x as f32, y as f32, z as f32));
                colors.push(color[[x, y, z]]);
            }
        }

        println!("{} voxels", positions.len());

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

        let vertex_normal_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: unsafe {
                        std::slice::from_raw_parts(
                            vertex_normals.as_ptr() as *const u8,
                            vertex_normals.len() * std::mem::size_of::<Vector3<f32>>(),
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
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            }],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vector3<f32>>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 3,
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

        Self {
            pipeline,
            vertex_buffer,
            vertex_normal_buffer,
            voxel_position_buffer,
            voxel_color_buffer,
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

impl RenderPass for VoxelPass {
    fn render<'p: 'r, 'r>(&'p self, _queue: &wgpu::Queue, render_pass: &mut wgpu::RenderPass<'r>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.vertex_normal_buffer.slice(..));
        render_pass.set_vertex_buffer(2, self.voxel_position_buffer.slice(..));
        render_pass.set_vertex_buffer(3, self.voxel_color_buffer.slice(..));
        render_pass.draw(0..3 * 12, 0..self.voxel_count as u32);
    }
}
