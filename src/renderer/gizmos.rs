use cgmath::{Matrix4, Vector3};

use crate::{symmetry::Symmetry, util};

pub struct Gizmos {
    gizmos: Vec<Gizmo>,
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Uniforms {
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

#[derive(Clone, Copy, Debug)]
struct Gizmo(Vector3<f32>, Vector3<f32>);

unsafe impl bytemuck::Pod for Gizmo {}
unsafe impl bytemuck::Zeroable for Gizmo {}

impl Gizmos {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 2048 * std::mem::size_of::<Gizmo>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: util::align(std::mem::size_of::<Uniforms>(), 16) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/gizmo.wgsl"),
                    include_str!("shaders/util.wgsl"),
                )
                .into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[&uniform_bind_group_layout],
                    ..Default::default()
                }),
            ),
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
                    format: color_format,
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
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multiview: None,
        });

        Self {
            gizmos: Vec::new(),
            vertex_buffer,
            pipeline,
            uniform_bind_group,
            uniform_buffer,
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

    pub fn clear(&mut self) {
        self.gizmos.clear();
    }

    pub fn prepare(&self, queue: &wgpu::Queue, camera: Symmetry, proj: Matrix4<f32>) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                view: camera.matrix(),
                proj,
            }]),
        );
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(self.gizmos.as_slice()),
        );
    }

    pub fn render<'p: 'r, 'r>(&'p self, render_pass: &mut wgpu::RenderPass<'r>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..2 * self.gizmos.len() as u32, 0..1);
    }
}
