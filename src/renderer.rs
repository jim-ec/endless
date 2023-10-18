use winit::window::Window;

use crate::{camera, render_pass::RenderPass};

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
pub const SAMPLES: u32 = 4;

pub struct Renderer {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    pub color_texture: Option<wgpu::Texture>,
    pub color_texture_view: Option<wgpu::TextureView>,
    pub depth_texture: wgpu::Texture,
    pub depth_texture_view: wgpu::TextureView,
    pub shader: wgpu::ShaderModule,
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
            size: std::mem::size_of::<camera::CameraUniforms>() as wgpu::BufferAddress,
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
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                            camera::CameraUniforms,
                        >()
                            as wgpu::BufferAddress),
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
                sample_count: SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            }),
        );

        let color_texture = if SAMPLES > 1 {
            Some(device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: window.inner_size().width,
                    height: window.inner_size().height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            }))
        } else {
            None
        };

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
            bind_group: uniform_bind_group,
            bind_group_layout: uniform_bind_group_layout,
            color_texture_view: color_texture
                .as_ref()
                .map(|color_texture| color_texture.create_view(&Default::default())),
            color_texture,
            depth_texture_view: depth_texture.create_view(&Default::default()),
            depth_texture,
            buffer: uniform_buffer,
            shader,
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
                sample_count: SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            }),
        );
        self.depth_texture_view = self.depth_texture.create_view(&Default::default());

        if SAMPLES > 1 {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            });
            self.color_texture_view = Some(texture.create_view(&Default::default()));
            self.color_texture = Some(texture);
        }
    }

    pub fn render<'a>(
        &self,
        camera: &camera::Camera,
        passes: &[&'a dyn RenderPass],
    ) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[
                camera.uniforms(self.size.width as f32 / self.size.height as f32)
            ]),
        );

        let mut command_encoder = self.device.create_command_encoder(&Default::default());

        if let Some(first_pass) = passes.first() {
            first_pass.render(
                &mut command_encoder,
                wgpu::RenderPassColorAttachment {
                    view: self.texture_view(&view),
                    resolve_target: self.resolve_target(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: true,
                    },
                },
                wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                },
                &self.bind_group,
            );
        }

        for pass in &passes[1..] {
            pass.render(
                &mut command_encoder,
                wgpu::RenderPassColorAttachment {
                    view: self.texture_view(&view),
                    resolve_target: self.resolve_target(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                },
                wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                },
                &self.bind_group,
            );
        }

        self.queue.submit(Some(command_encoder.finish()));

        surface_texture.present();

        Ok(())
    }

    pub fn texture_view<'a>(&'a self, view: &'a wgpu::TextureView) -> &'a wgpu::TextureView {
        self.color_texture_view.as_ref().unwrap_or(view)
    }

    pub fn resolve_target<'a>(
        &'a self,
        view: &'a wgpu::TextureView,
    ) -> Option<&'a wgpu::TextureView> {
        self.color_texture_view.as_ref().and(Some(view))
    }
}
