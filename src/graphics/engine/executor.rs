use std::ops::Deref;

use wgpu::{CommandEncoder, Origin3d, TexelCopyTextureInfo};

use crate::graphics::{BindGroupId, BufferId, ComputePassInfo, GpuCommand, GpuContext, LayoutId, PassValidator, PipelineId, RenderPassInfo, TextureId};

/// Executes render and compute pipelines
pub(crate) struct GpuExecutor {
    validator: PassValidator,
}

impl GpuExecutor {
    pub fn new() -> Self {
        Self {
            validator: PassValidator::new()
        }
    }

    /// Copy a texture into another
    pub fn copy_textures<'a>(context: &'a GpuContext, src_id: &TextureId, dst_id: &TextureId) {
        let mut encoder = context.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        GpuExecutor::copy_textures_from_encoder(context, &mut encoder, src_id, dst_id);
        context.gpu.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Copy a texture into another with the provided encoder
    pub fn copy_textures_from_encoder<'a>(context: &'a GpuContext, encoder: &mut CommandEncoder, src_id: &TextureId, dst_id: &TextureId) {
        let src_tex_opt = context.textures.get(src_id);
        let dst_tex_opt = context.textures.get(dst_id);

        if let (Some(src_tex), Some(dst_tex)) = (src_tex_opt, dst_tex_opt) {
            println!("src format: {:?}, dst format: {:?}", src_tex.texture.format(), dst_tex.texture.format());
            println!("src extent: {:?}, dst extent: {:?}", src_tex.extent, dst_tex.extent);

            encoder.copy_texture_to_texture(
                TexelCopyTextureInfo {
                    texture: &src_tex.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                }, 
                TexelCopyTextureInfo {
                    texture: &dst_tex.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                }, 
                src_tex.extent
            );
        }
    }

    /// invalidate a buffer, indicating that it was lost/destroyed.
    pub fn invalidate_buffer<'a>(&self, buf_id: &BufferId, context: &'a GpuContext) {
        self.validator.invalidate_buffer(buf_id, context);
    }

    /// invalidate a texture, indicating that it was lost/destroyed.
    pub fn invalidate_texture<'a>(&self, tex_id: &TextureId, context: &'a GpuContext) {
        self.validator.invalidate_texture(tex_id, context);
    }
    
    /// invalidate a bind group layout, indicating that it was lost/destroyed.
    pub fn invalidate_layout<'a>(&self, bgl_id: &LayoutId, context: &'a GpuContext) {
        self.validator.invalidate_layout(bgl_id, context);
    }

    /// invalidate a bind group, indicating that it was lost/destroyed.
    pub fn invalidate_bind_group(&self, bg_id: &BindGroupId) {
        self.validator.invalidate_bind_group(bg_id);
    }

    /// invalidate a pipeline, indicating that it was lost/destroyed.
    pub fn invalidate_pipeline(&self, pip_id: &PipelineId) {
        self.validator.invalidate_pipeline(pip_id);
    }

    /// Execute the render/compute passes on the provided output view
    pub fn execute<'a>(&self, context: &'a GpuContext, commands: Vec<GpuCommand>, output_view: wgpu::TextureView) {
        let mut encoder = context.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        
        for cmd in commands {
            match cmd {
                GpuCommand::RenderPass(pass) => self.execute_render_pass(context, &mut encoder, pass, &output_view),
                GpuCommand::ComputePass(pass) => self.execute_compute_pass(context, &mut encoder, pass),
                GpuCommand::CopyTexture { src, dst } => GpuExecutor::copy_textures_from_encoder(context, &mut encoder, &src, &dst),
            }
        }

        context.gpu.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Execute a render pass on the provided encoder
    fn execute_render_pass<'a>(&self, context: &'a GpuContext, encoder: &mut wgpu::CommandEncoder, info: RenderPassInfo, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            ..Default::default()
        });

        let Some(pipeline) = self.validator.validate_render_pipeline(&info.pipeline_id, context) else { 
            // println!("[PassExecutor] Failed to validate render pipeline @{:?}", info.pipeline_id);
            return; 
        };
        render_pass.set_pipeline(&pipeline);

        for (idx, bg_id) in info.bind_groups.iter().enumerate() {
            let Some(bg) = self.validator.validate_bind_group(bg_id, context) else { 
                // println!("[PassExecutor] Failed to validate bind group @{:?} for render pipeline @{:?}", bg_id, info.pipeline_id);
                return; 
            };
            render_pass.set_bind_group(idx as u32, bg.deref(), &[]);
        }
        render_pass.draw(0..info.vertex_count, 0..info.instance_count);
    }

    /// Execute a compute pass on the provided encoder
    fn execute_compute_pass<'a>(&self, context: &'a GpuContext, encoder: &mut wgpu::CommandEncoder, info: ComputePassInfo) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            ..Default::default()
        });

        let Some(pipeline) = self.validator.validate_compute_pipeline(&info.pipeline_id, context) else {
            // println!("[PassExecutor] Failed to validate compute pipeline @{:?}", info.pipeline_id);
            return; 
        };
        compute_pass.set_pipeline(&pipeline);

        for (idx, bg_id) in info.bind_groups.iter().enumerate() {
            let Some(bg) = self.validator.validate_bind_group(bg_id, context) else { 
                // println!("[PassExecutor] Failed to validate bind group @{:?} for compute pipeline @{:?}", bg_id, info.pipeline_id);
                return; 
            };
            compute_pass.set_bind_group(idx as u32, bg.deref(), &[]);
        }

        let (wx, wy, wz) = info.work_groups;
        compute_pass.dispatch_workgroups(wx, wy, wz);
    }
}