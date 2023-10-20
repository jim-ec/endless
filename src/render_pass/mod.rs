#![warn(unused)]

pub mod gizmo_pass;
pub mod voxel_pass;

pub trait RenderPass {
    fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        color_attachment: wgpu::RenderPassColorAttachment,
        depth_attachment: wgpu::RenderPassDepthStencilAttachment,
        bind_group: &wgpu::BindGroup,
    );
}
