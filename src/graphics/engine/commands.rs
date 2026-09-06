use crate::graphics::{BindGroupId, BufferId, GpuContext, PipelineId};

/// Represents commands that can submitted and recorded by an Executor implementation.
pub trait GpuCommand {
    /// Record this command using the provided encoder in the provided context
    fn record<'a>(&mut self, encoder: &mut wgpu::CommandEncoder, context: &'a GpuContext);
}

/// Draws triangles to output textures using a wgpu::RenderPass
pub struct DrawCommand {
    /// the id to the pipeline the draw command will run on
    pip_id: PipelineId,
    /// the output view for which the draw will render to
    output_view: wgpu::TextureView,
    /// the set of bind groups used in the pipeline
    bind_groups: Vec<BindGroupId>,
    /// the set of vertex buffers used in the draw command
    vertex_buffers: Vec<BufferId>,
    /// optional index buffer. When set, invocations must be proportional to the size of this buffer.
    index_buffer: Option<BufferId>,
    /// the format of the index buffer, if provided
    index_format: Option<wgpu::IndexFormat>,
    /// The number of invocations to run with the pipeline
    invocations: u32,
    /// The number of instances to draw.
    instance_count: u32,
}

impl DrawCommand {
    pub fn new(pip_id: PipelineId, output_view: wgpu::TextureView, invocations: u32) -> Self {
        Self {
            pip_id,
            output_view,
            invocations,
            bind_groups: Vec::new(),
            vertex_buffers: Vec::new(),
            index_buffer: None,
            index_format: None,
            instance_count: 1,
        }
    }

    /// Add a set of vertex buffers to the draw command
    pub fn with_vertex_buffers(mut self, buffers: &[BufferId]) -> Self {
        self.vertex_buffers.extend_from_slice(buffers);
        self
    }

    /// Add an index buffer to the draw command.
    pub fn with_index_buffer(mut self, buffer: BufferId, format: wgpu::IndexFormat) -> Self {
        self.index_buffer = Some(buffer);
        self.index_format = Some(format);
        self
    }

    /// Add a set of bind groups to the draw command
    pub fn with_bind_groups(mut self, groups: &[BindGroupId]) -> Self {
        self.bind_groups.extend_from_slice(groups);
        self
    }
}

impl GpuCommand for DrawCommand {
    fn record<'a>(&mut self, encoder: &mut wgpu::CommandEncoder, context: &'a GpuContext) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                view: &self.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            ..Default::default()
        });

        let Some(pipeline) = context.validate_pipeline(&self.pip_id).and_then(|pip| pip.as_render()) else { 
            // println!("[DrawCommand] Failed to validate render pipeline @{:?}", info.pipeline_id);
            return; 
        };
        render_pass.set_pipeline(&pipeline);

        for (idx, bg_id) in self.bind_groups.iter().enumerate() {
            let Some(bg) = context.validate_bind_group(bg_id) else { 
                // println!("[DrawCommand] Failed to validate bind group @{:?} for render pipeline @{:?}", bg_id, info.pipeline_id);
                return; 
            };
            render_pass.set_bind_group(idx as u32, &bg.bind_group, &[]);
        }

        for (idx, vtx_id) in self.vertex_buffers.iter().enumerate() {
            let Some(buffer) = context.resources.buffers.get(vtx_id) else {
                return;
            };
            render_pass.set_vertex_buffer(idx as u32, buffer.slice(..));
        }

        if let Some(idx_id) = self.index_buffer {
            let Some(buffer) = context.resources.buffers.get(&idx_id) else {
                return;
            };
            render_pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.invocations, 0, 0..self.instance_count);
        } else {
            render_pass.draw(0..self.invocations, 0..self.instance_count);
        }
    }
}

/// Dispatches a compute shader to the gpu
pub struct ComputeCommand {
    pip_id: PipelineId,
    bind_groups: Vec<BindGroupId>,
    workgroups: (u32, u32, u32),
}

impl ComputeCommand {
    pub fn new(pip_id: PipelineId, workgroups: (u32, u32, u32)) -> Self {
        Self {
            pip_id,
            workgroups,
            bind_groups: Vec::new()
        }
    }

    /// Add a set of bind groups to the draw command
    pub fn with_bind_groups(mut self, bind_groups: &[BindGroupId]) -> Self {
        self.bind_groups.extend_from_slice(bind_groups);
        self
    }
}

impl GpuCommand for ComputeCommand {
    fn record<'a>(&mut self, encoder: &mut wgpu::CommandEncoder, context: &'a GpuContext) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            ..Default::default()
        });

        let Some(pipeline) = context.validate_pipeline(&self.pip_id).and_then(|pip| pip.as_compute()) else {
            // println!("[ComputeCommand] Failed to validate compute pipeline @{:?}", info.pipeline_id);
            return; 
        };
        compute_pass.set_pipeline(&pipeline);

        for (idx, bg_id) in self.bind_groups.iter().enumerate() {
            let Some(bg) = context.validate_bind_group(bg_id) else { 
                // println!("[ComputeCommand] Failed to validate bind group @{:?} for compute pipeline @{:?}", bg_id, info.pipeline_id);
                return; 
            };
            compute_pass.set_bind_group(idx as u32, &bg.bind_group, &[]);
        }

        let (wx, wy, wz) = self.workgroups;
        compute_pass.dispatch_workgroups(wx, wy, wz);
    }
}