#![warn(unused)]

pub mod line_pass;
pub mod voxel_pass;
pub mod water_pass;

pub trait RenderPass {
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_attachment: wgpu::RenderPassColorAttachment,
        depth_attachment: wgpu::RenderPassDepthStencilAttachment,
        bind_group: &wgpu::BindGroup,
    );
}
