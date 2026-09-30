use std::ops::Range;

use crate::graphics::{BindGroupId, BufferId, GpuContext, PipelineId};

/// Represents entities that can used in a DrawCommand
pub trait Drawable {
    /// Draw this entity given the provided render pass in the provided context
    fn draw(&mut self, pass: &mut wgpu::RenderPass, context: &GpuContext);
}

/// Draws an entity. Must be used with a DrawCommand. Supports indexed or non-indexed draw calls
pub struct IndexedDraw {
    /// the id to the pipeline the draw command will run on
    pip_id: PipelineId,
    /// the set of bind groups used in the pipeline
    bind_groups: Vec<BindGroupId>,
    /// the set of vertex buffers used in the draw command
    vertex_buffers: Vec<BufferId>,
    /// optional index buffer. When set, invocations must be proportional to the size of this buffer.
    index_buffer: Option<BufferId>,
    /// the format of the index buffer, if provided
    index_format: Option<wgpu::IndexFormat>,
    /// The range of elements (vertices/indices) to draw with the pipeline
    element_range: Range<u32>,
    /// The range of instances to draw with the pipeline.
    instance_range: Range<u32>,
}

impl IndexedDraw {
    pub fn new(pip_id: PipelineId, element_range: Range<u32>) -> Self {
        Self {
            pip_id,
            element_range,
            bind_groups: Vec::new(),
            vertex_buffers: Vec::new(),
            index_buffer: None,
            index_format: None,
            instance_range: 0..1,
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

    /// Set the range of instances to draw. Must have a corresponding instance buffer that is at least as large as 'max'.
    pub fn with_instances(mut self, range: Range<u32>) -> Self {
        self.instance_range = range;
        self
    }
}

impl Drawable for IndexedDraw {
    fn draw(&mut self, pass: &mut wgpu::RenderPass, context: &GpuContext) {
        let Some(pipeline) = context.validate_pipeline(&self.pip_id).and_then(|pip| pip.to_render()) else { 
            // println!("[DrawCommand] Failed to validate render pipeline @{:?}", info.pipeline_id);
            return; 
        };
        pass.set_pipeline(&pipeline);

        for (idx, bg_id) in self.bind_groups.iter().enumerate() {
            let Some(bg) = context.validate_bind_group(bg_id) else { 
                // println!("[DrawCommand] Failed to validate bind group @{:?} for render pipeline @{:?}", bg_id, info.pipeline_id);
                return; 
            };
            pass.set_bind_group(idx as u32, &bg.bind_group, &[]);
        }

        for (idx, vtx_id) in self.vertex_buffers.iter().enumerate() {
            let Some(buffer) = context.resources.buffers.get(vtx_id) else {
                return;
            };
            pass.set_vertex_buffer(idx as u32, buffer.slice(..));
        }

        if let Some(idx_id) = self.index_buffer {
            let Some(buffer) = context.resources.buffers.get(&idx_id) else {
                return;
            };
            pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(self.element_range.clone(), 0, self.instance_range.clone());
        } else {
            pass.draw(self.element_range.clone(), self.instance_range.clone());
        }
    }
}