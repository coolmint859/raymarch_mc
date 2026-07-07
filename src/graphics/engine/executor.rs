use std::ops::Deref;

use crate::graphics::{BindGroupId, BufferId, ComputePassInfo, GpuContext, GpuHandle, PassInfo, PassValidator, PipelineId, RenderPassInfo, TextureId};

pub(crate) struct PassExecutor {
    gpu: GpuHandle,
    validator: PassValidator,
}

impl PassExecutor {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            validator: PassValidator::new()
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

    /// invalidate a bind group, indicating that it was lost/destroyed.
    pub fn invalidate_bind_group<'a>(&self, bg_id: &BindGroupId, context: &'a GpuContext) {
        self.validator.invalidate_bind_group(bg_id, context);
    }

    /// invalidate a pipeline, indicating that it was lost/destroyed.
    pub fn invalidate_pipeline(&self, pip_id: &PipelineId) {
        self.validator.invalidate_pipeline(pip_id);
    }

    /// Execute the render/compute passes on the provided output view
    pub fn execute<'a>(&self, context: &'a GpuContext, passes: Vec<PassInfo>, output_view: wgpu::TextureView) {
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        
        for pass in passes {
            match pass {
                PassInfo::Render(pass) => self.execute_render_pass(context, &mut encoder, pass, &output_view),
                PassInfo::Compute(pass) => self.execute_compute_pass(context, &mut encoder, pass),
            }
        }
        
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
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
            // println!("[GpuContext] Failed to validate render pipeline @{:?}", pass.pipeline_id);
            return; 
        };
        render_pass.set_pipeline(&pipeline);

        for (idx, bg_id) in info.bind_groups.iter().enumerate() {
            let Some(bg) = self.validator.verify_bind_group(bg_id, context) else { 
                // println!("[GpuContext] Failed to validate bind group @{:?} for render pipeline @{:?}", bg_id, pass.pipeline_id);
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
            // println!("[GpuContext] Failed to validate compute pipeline @{:?}", pass.pipeline_id);
            return; 
        };
        compute_pass.set_pipeline(&pipeline);

        for (idx, bg_id) in info.bind_groups.iter().enumerate() {
            let Some(bg) = self.validator.verify_bind_group(bg_id, context) else { 
                // println!("[GpuContext] Failed to validate bind group @{:?} for compute pipeline @{:?}", bg_id, pass.pipeline_id);
                return; 
            };
            compute_pass.set_bind_group(idx as u32, bg.deref(), &[]);
        }

        let (wx, wy, wz) = info.work_groups;
        compute_pass.dispatch_workgroups(wx, wy, wz);
    }
}