use crate::graphics::{BindGroupId, GpuContext, PipelineId, Drawable};

/// Represents commands that can submitted and recorded by an Executor implementation.
pub trait GpuCommand {
    /// Record this command using the provided encoder in the provided context
    fn record(&mut self, encoder: &mut wgpu::CommandEncoder, context: &GpuContext);
}

/// Describes the state of a draw command (echoing wgpu render pass descriptors)
pub struct RenderingState {
    /// The output texture view for which draw calls will render to
    pub output_view: wgpu::TextureView,
    /// An optional clear color for the render pass created from a draw command. If None, the render pass will not clear the output view before rendering.
    pub clear_color: Option<wgpu::Color>,
}

/// Renders Drawable entites to an output view via a wgpu render pass
pub struct DrawCommand {
    /// The set of drawables to issue draw calls for
    draws: Vec<Box<dyn Drawable>>,
    /// The output texture the draw calls will be rendered to
    state: RenderingState
}

impl DrawCommand {
    pub fn new(state: RenderingState) -> Self {
        Self {
            draws: Vec::new(),
            state
        }
    }

    /// Create a new DrawCommand with a set of drawables
    pub fn from_draws(state: RenderingState, drawables: impl IntoIterator<Item = impl Drawable + 'static>) -> Self {
        let mut cmd = DrawCommand::new(state);

        for drawable in drawables {
            cmd.add_draw(drawable);
        }

        cmd
    }

    /// Add a Drawable item into the draw command
    pub fn with_draw(mut self, drawable: impl Drawable + 'static) -> Self {
        self.add_draw(drawable);
        self
    }

    /// Add a Drawable item into the draw command
    pub fn add_draw(&mut self, drawable: impl Drawable + 'static) {
        self.draws.push(Box::new(drawable));
    }

    /// Returns true if at least one drawable was added to the command, false otherwise
    pub fn has_draws(&self) -> bool {
        return !self.draws.is_empty()
    }
}

impl GpuCommand for DrawCommand {
    fn record(&mut self, encoder: &mut wgpu::CommandEncoder, context: &GpuContext) {
        let load_op = match self.state.clear_color {
            Some(color) => wgpu::LoadOp::Clear(color),
            None => wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                view: &self.state.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            ..Default::default()
        });

        for mut drawable in std::mem::take(&mut self.draws) {
            drawable.draw(&mut render_pass, context);
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
    fn record(&mut self, encoder: &mut wgpu::CommandEncoder, context: &GpuContext) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            ..Default::default()
        });

        let Some(pipeline) = context.validate_pipeline(&self.pip_id).and_then(|pip| pip.to_compute()) else {
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