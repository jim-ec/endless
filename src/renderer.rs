use cgmath::{Matrix4, SquareMatrix, Vector3};
use winit::window::Window;

use crate::{camera, util};

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

pub struct Renderer {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub uniform_bind_group: wgpu::BindGroup,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    pub depth_texture: wgpu::Texture,
    pub shader: wgpu::ShaderModule,
    pub triangle_count: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Uniforms {
    pub model: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub camera_translation: Vector3<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

pub trait RenderPass {
    fn model_matrix(&self) -> Matrix4<f32>;
    fn render<'p: 'r, 'r>(&'p self, queue: &wgpu::Queue, render_pass: &mut wgpu::RenderPass<'r>);
}

impl Renderer {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No GPU available");

        println!("GPU: {}", adapter.get_info().name);
        println!("Render Backend: {:?}", adapter.get_info().backend);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        println!("Swapchain format: {:?}", config.format);
        println!("Swapchain present mode: {:?}", config.present_mode);

        surface.configure(&device, &config);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: util::align(std::mem::size_of::<Uniforms>(), 16) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let depth_texture = device.create_texture(
            &(wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            }),
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string("shaders/shader.wgsl")
                    .expect("Cannot read shader file")
                    .into(),
            ),
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            uniform_bind_group,
            uniform_bind_group_layout,
            depth_texture,
            uniform_buffer,
            shader,
            triangle_count: 0,
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);

        self.depth_texture = self.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            }),
        );
    }

    pub fn render(
        &self,
        camera: &camera::Camera,
        passes: &[&dyn RenderPass],
        egui_renderer: &mut egui_wgpu::Renderer,
        egui_mesh: &[egui::ClippedPrimitive],
        dpi: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut uniforms = Uniforms {
            model: Matrix4::identity(),
            view: camera.view_matrix(),
            proj: camera.proj_matrix(self.size.width as f32 / self.size.height as f32),
            camera_translation: camera.translation,
        };

        let depth_texture_view = self.depth_texture.create_view(&Default::default());

        {
            let mut command_encoder = self.device.create_command_encoder(&Default::default());
            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            self.queue.submit(Some(command_encoder.finish()));
        }

        for pass in passes {
            let mut command_encoder = self.device.create_command_encoder(&Default::default());

            uniforms.model = pass.model_matrix();
            self.queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            pass.render(&self.queue, &mut render_pass);

            drop(render_pass);
            self.queue.submit(Some(command_encoder.finish()));
        }

        {
            let mut command_encoder = self.device.create_command_encoder(&Default::default());

            egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut command_encoder,
                egui_mesh,
                &egui_wgpu::renderer::ScreenDescriptor {
                    size_in_pixels: [self.size.width, self.size.height],
                    pixels_per_point: dpi,
                },
            );

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            egui_renderer.render(
                &mut render_pass,
                egui_mesh,
                &egui_wgpu::renderer::ScreenDescriptor {
                    size_in_pixels: [self.size.width, self.size.height],
                    pixels_per_point: dpi,
                },
            );
            drop(render_pass);
            self.queue.submit(Some(command_encoder.finish()));
        }

        surface_texture.present();

        Ok(())
    }
}
