pub mod gizmos;
pub mod voxels;

use std::{collections::HashMap, sync::Arc};

use cgmath::Vector3;
use winit::window::Window;

use crate::{camera, symmetry::Symmetry, world::Chunk};

use self::{
    gizmos::Gizmos,
    voxels::{VoxelMesh, VoxelPipeline},
};

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

pub struct Renderer {
    surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    ui_renderer: egui_wgpu::Renderer,
    ui_ctx: egui::Context,
    depth_texture: wgpu::Texture,
    camera_symmetry: Symmetry,
    camera_fovy: f32,
    pub gizmos: Gizmos,
    voxel_pipeline: VoxelPipeline,
    chunk_meshes: HashMap<Vector3<isize>, VoxelMesh>,
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

        Self {
            ui_renderer: egui_wgpu::Renderer::new(&device, config.format, Some(DEPTH_FORMAT), 1),
            ui_ctx: egui::Context::default(),
            gizmos: Gizmos::new(&device, config.format, DEPTH_FORMAT),
            voxel_pipeline: VoxelPipeline::new(&device, config.format, DEPTH_FORMAT),
            surface,
            adapter,
            device: Arc::new(device),
            queue,
            config,
            depth_texture,
            camera_symmetry: camera::Camera::initial().symmetry(),
            camera_fovy: camera::Camera::initial().fovy,
            chunk_meshes: HashMap::new(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
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

    pub fn ctx(&self) -> &egui::Context {
        &self.ui_ctx
    }

    pub fn render(
        &mut self,
        camera: camera::Camera,
        ui_output: egui::FullOutput,
        chunks: &HashMap<Vector3<isize>, Chunk>,
        scale_factor: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture_view = self.depth_texture.create_view(&Default::default());

        self.camera_symmetry = self
            .camera_symmetry
            .inverse()
            .interpolate(&camera.symmetry().inverse(), 0.4)
            .inverse();
        self.camera_fovy += 0.4 * (camera.fovy - self.camera_fovy);
        let proj = camera::perspective_matrix(
            self.camera_fovy.to_radians(),
            self.config.width as f32 / self.config.height as f32,
            0.1,
            None,
        );

        // Clear
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
            self.queue.submit([command_encoder.finish()]);
        }

        // Chunks
        for chunk in chunks.values() {
            self.voxel_pipeline.prepare(
                &self.queue,
                &chunk.voxel_mesh,
                self.camera_symmetry,
                proj,
                camera.translation,
            );

            let mut command_encoder = self.device.create_command_encoder(&Default::default());

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

            self.voxel_pipeline
                .render(&mut render_pass, &chunk.voxel_mesh);

            drop(render_pass);
            self.queue.submit([command_encoder.finish()]);
        }

        // Gizmos
        {
            self.gizmos.prepare(&self.queue, self.camera_symmetry, proj);

            let mut command_encoder = self.device.create_command_encoder(&Default::default());

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

            self.gizmos.render(&mut render_pass);

            drop(render_pass);
            self.queue.submit([command_encoder.finish()]);
        }

        // UI
        {
            for (id, delta) in &ui_output.textures_delta.set {
                self.ui_renderer
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
            for id in &ui_output.textures_delta.free {
                self.ui_renderer.free_texture(id);
            }

            let triangles = self.ui_ctx.tessellate(ui_output.shapes);

            let mut command_encoder = self.device.create_command_encoder(&Default::default());

            self.ui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut command_encoder,
                &triangles,
                &egui_wgpu::renderer::ScreenDescriptor {
                    size_in_pixels: [self.config.width, self.config.height],
                    pixels_per_point: scale_factor,
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

            self.ui_renderer.render(
                &mut render_pass,
                &triangles,
                &egui_wgpu::renderer::ScreenDescriptor {
                    size_in_pixels: [self.config.width, self.config.height],
                    pixels_per_point: scale_factor,
                },
            );
            drop(render_pass);
            self.queue.submit([command_encoder.finish()]);
        }

        surface_texture.present();

        self.gizmos.clear();

        Ok(())
    }
}
