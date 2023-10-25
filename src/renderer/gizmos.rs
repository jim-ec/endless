use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::renderer::{RenderPass, Renderer, DEPTH_FORMAT};

pub struct Gizmos {
    gizmos: Vec<Gizmo>,
    buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

#[derive(Clone, Copy, Debug)]
struct Gizmo(Vector3<f32>, Vector3<f32>);

unsafe impl bytemuck::Pod for Gizmo {}
unsafe impl bytemuck::Zeroable for Gizmo {}

impl Gizmos {
    pub fn new(renderer: &Renderer) -> Self {
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 2048 * std::mem::size_of::<Gizmo>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(
                    concat!(
                        include_str!("shaders/gizmo.wgsl"),
                        include_str!("shaders/util.wgsl"),
                    )
                    .into(),
                ),
            });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&renderer.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        bind_group_layouts: &[&renderer.uniform_bind_group_layout],
                        ..Default::default()
                    },
                )),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vertex",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vector3<f32>>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x4,
                        }],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fragment",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                multisample: wgpu::MultisampleState::default(),
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
            gizmos: Vec::new(),
            buffer,
            pipeline,
        }
    }

    pub fn aabb(&mut self, min: Vector3<f32>, max: Vector3<f32>) {
        self.gizmos.extend([
            Gizmo(min, Vector3::new(min.x, min.y, max.z)),
            Gizmo(
                Vector3::new(min.x, min.y, max.z),
                Vector3::new(max.x, min.y, max.z),
            ),
            Gizmo(
                Vector3::new(max.x, min.y, max.z),
                Vector3::new(max.x, min.y, min.z),
            ),
            Gizmo(Vector3::new(max.x, min.y, min.z), min),
            Gizmo(
                Vector3::new(min.x, max.y, min.z),
                Vector3::new(min.x, max.y, max.z),
            ),
            Gizmo(Vector3::new(min.x, max.y, max.z), max),
            Gizmo(max, Vector3::new(max.x, max.y, min.z)),
            Gizmo(
                Vector3::new(max.x, max.y, min.z),
                Vector3::new(min.x, max.y, min.z),
            ),
            Gizmo(
                Vector3::new(min.x, min.y, min.z),
                Vector3::new(min.x, max.y, min.z),
            ),
            Gizmo(
                Vector3::new(min.x, min.y, max.z),
                Vector3::new(min.x, max.y, max.z),
            ),
            Gizmo(
                Vector3::new(max.x, min.y, max.z),
                Vector3::new(max.x, max.y, max.z),
            ),
            Gizmo(
                Vector3::new(max.x, min.y, min.z),
                Vector3::new(max.x, max.y, min.z),
            ),
        ]);
    }
}

impl RenderPass for Gizmos {
    fn render<'p: 'r, 'r>(&'p self, queue: &wgpu::Queue, render_pass: &mut wgpu::RenderPass<'r>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..2 * self.gizmos.len() as u32, 0..1);

        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(self.gizmos.as_slice()),
        );
    }

    fn model_matrix(&self) -> Matrix4<f32> {
        Matrix4::identity()
    }
}
