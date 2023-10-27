pub mod gizmos;
pub mod voxels;

use std::{collections::HashMap, sync::Arc};

use cgmath::{vec4, InnerSpace, Matrix, Vector3};
use itertools::Itertools;
use winit::window::Window;

use crate::{
    camera,
    symmetry::Symmetry,
    util,
    world::{Chunk, N},
};

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
    pub(super) voxel_pipeline: VoxelPipeline,
    chunk_meshes: HashMap<Vector3<isize>, VoxelMesh>,
    chunk_uniform_buffer: wgpu::Buffer,
}

const MAX_CHUNK_UNIFORMS: usize = 4096; // TODO: Smaller size, but resize on demand

#[derive(Default)]
pub struct RenderStats {
    pub chunk_count: usize,
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

        let chunk_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: util::align(
                MAX_CHUNK_UNIFORMS * std::mem::size_of::<voxels::Uniforms>(),
                16,
            ) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            chunk_uniform_buffer,
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
        enable_gizmos: bool,
    ) -> Result<RenderStats, wgpu::SurfaceError> {
        puffin::profile_function!();

        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture_view = self.depth_texture.create_view(&Default::default());

        let mut stats = RenderStats::default();

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

        let mut command_encoder = self.device.create_command_encoder(&Default::default());

        let ui_triangles = self.ui_ctx.tessellate(ui_output.shapes);
        self.ui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut command_encoder,
            &ui_triangles,
            &egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: scale_factor,
            },
        );
        for (id, delta) in &ui_output.textures_delta.set {
            self.ui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }
        for id in &ui_output.textures_delta.free {
            self.ui_renderer.free_texture(id);
        }

        let mut visible_chunks = vec![];
        {
            puffin::profile_scope!("Cull Chunks");

            for chunk in chunks.values() {
                // Since this is the full model-view-projection matrix, the extracted
                // planes are in model space.
                let m = (proj * self.camera_symmetry.matrix() * chunk.voxel_mesh.symmetry.matrix())
                    .transpose();

                // We do not test against the far plane because that is handled by the generation radius.
                let planes = [
                    m[3] + m[0], // left
                    m[3] - m[0], // right
                    m[3] + m[1], // bottom
                    m[3] - m[1], // top
                    m[3] + m[2], // near
                ];

                // The extent of the unit cube we are testing against
                let max = N as f32 / chunk.voxel_mesh.symmetry.scale;

                // A chunk is partially visible if:
                // ∀ plane ∈ planes: ∃ vertex ∈ chunk: dist(plane, vertex) > 0
                // Applying De Morgan's law:
                // ¬(∀ plane ∈ planes: ∃ vertex ∈ chunk: dist(plane, vertex) > 0)
                // = ∃ plane ∈ planes: ¬(∃ vertex ∈ chunk: dist(plane, vertex) > 0)
                // = ∃ plane ∈ planes: ∀ vertex ∈ chunk: dist(plane, vertex) ≤ 0
                if !planes.into_iter().any(|plane| {
                    [0.0, max]
                        .into_iter()
                        .cartesian_product([0.0, max])
                        .cartesian_product([0.0, max])
                        .all(|((x, y), z)| plane.dot(vec4(x, y, z, 1.0)) <= 0.0)
                }) {
                    visible_chunks.push(chunk);
                    stats.chunk_count += 1;
                }
            }
        }

        let bind_groups: Vec<_> = {
            puffin::profile_scope!("Create Bind Groups");
            (0..visible_chunks.len())
                .map(|i| {
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: &self.voxel_pipeline.bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &self.chunk_uniform_buffer,
                                offset: (i * std::mem::size_of::<voxels::Uniforms>())
                                    as wgpu::BufferAddress,
                                size: Some(unsafe {
                                    wgpu::BufferSize::new_unchecked(std::mem::size_of::<
                                        voxels::Uniforms,
                                    >(
                                    )
                                        as wgpu::BufferAddress)
                                }),
                            }),
                        }],
                    })
                })
                .collect()
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        {
            puffin::profile_scope!("Update Uniform Buffer");

            let uniforms: Vec<_> = visible_chunks
                .iter()
                .map(|chunk| voxels::Uniforms {
                    model: chunk.voxel_mesh.symmetry.matrix(),
                    view: self.camera_symmetry.matrix(),
                    proj,
                    light: camera.translation,
                })
                .collect();
            assert!(
                uniforms.len() <= MAX_CHUNK_UNIFORMS,
                "Chunk uniform buffer out of memory"
            );

            self.queue.write_buffer(
                &self.chunk_uniform_buffer,
                0,
                bytemuck::cast_slice(uniforms.as_slice()),
            );
        }

        {
            puffin::profile_scope!("Render Chunks");

            render_pass.set_pipeline(&self.voxel_pipeline.pipeline);

            for (chunk, bind_group) in visible_chunks.iter().zip(bind_groups.iter()) {
                {
                    puffin::profile_scope!("Record Render Pass");
                    render_pass.set_bind_group(0, bind_group, &[]);
                    render_pass.set_vertex_buffer(0, chunk.voxel_mesh.buffer.slice(..));
                    render_pass.draw(0..chunk.voxel_mesh.count as u32, 0..1);
                }
            }
        }

        // Gizmos
        if enable_gizmos {
            self.gizmos.prepare(&self.queue, self.camera_symmetry, proj);
            self.gizmos.render(&mut render_pass);
        }

        // UI
        self.ui_renderer.render(
            &mut render_pass,
            &ui_triangles,
            &egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: scale_factor,
            },
        );

        drop(render_pass);

        self.queue.submit([command_encoder.finish()]);
        surface_texture.present();

        self.gizmos.clear();

        Ok(stats)
    }
}
