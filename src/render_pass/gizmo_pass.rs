use cgmath::Vector3;

use crate::renderer::{self, Renderer, DEPTH_FORMAT};

use super::RenderPass;

pub struct GizmoPass {
    gizmos: Vec<Gizmo>,
    buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

#[derive(Clone, Copy, Debug)]
struct Gizmo(Vector3<f32>, Vector3<f32>);

unsafe impl bytemuck::Pod for Gizmo {}
unsafe impl bytemuck::Zeroable for Gizmo {}

impl GizmoPass {
    pub fn new(renderer: &Renderer) -> Self {
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 2048 * std::mem::size_of::<Gizmo>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
                    entry_point: "gizmo_vertex",
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
                    module: &renderer.shader,
                    entry_point: "gizmo_fragment",
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
                multisample: wgpu::MultisampleState {
                    count: renderer::SAMPLES,
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

impl RenderPass for GizmoPass {
    fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        color_attachment: wgpu::RenderPassColorAttachment,
        depth_attachment: wgpu::RenderPassDepthStencilAttachment,
        bind_group: &wgpu::BindGroup,
    ) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(self.gizmos.as_slice()),
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(depth_attachment),
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..2 * self.gizmos.len() as u32, 0..1);
    }
}
