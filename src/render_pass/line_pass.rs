use cgmath::Vector3;
use wgpu::util::BufferInitDescriptor;
use wgpu::util::DeviceExt;

use itertools::Itertools;

use crate::renderer::{self, Renderer, DEPTH_FORMAT};

use super::RenderPass;

pub struct LinePass {
    count: usize,
    buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl LinePass {
    pub fn new(
        renderer: &Renderer,
        line_strips: impl IntoIterator<Item = impl IntoIterator<Item = Vector3<f32>>>,
    ) -> Self {
        let mut lines = Vec::new();
        for line_strip in line_strips {
            for (a, b) in line_strip.into_iter().tuple_windows() {
                lines.extend([a, b]);
            }
        }

        let buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    lines.as_ptr() as *const u8,
                    lines.len() * std::mem::size_of::<Vector3<f32>>(),
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
                    entry_point: "line_vertex",
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
                    entry_point: "line_fragment",
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
            count: lines.len(),
            buffer,
            pipeline,
        }
    }
}

impl RenderPass for LinePass {
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_attachment: wgpu::RenderPassColorAttachment,
        depth_attachment: wgpu::RenderPassDepthStencilAttachment,
        bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(depth_attachment),
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.count as u32, 0..1);
    }
}
