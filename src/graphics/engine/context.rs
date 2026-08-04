use crate::graphics::*;

/// Represents a render pass
#[derive(Clone, Debug)]
pub struct RenderPassInfo {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub vertex_count: u32,
    pub instance_count: u32
}

/// Represents a compute pass
#[derive(Clone, Debug)]
pub struct ComputePassInfo {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub work_groups: (u32, u32, u32) // x, y, z
}

/// Represents a render or compute pass.
#[derive(Clone, Debug)]
pub enum GpuCommand {
    RenderPass(RenderPassInfo),
    ComputePass(ComputePassInfo),
    CopyTexture{ src: TextureId, dst: TextureId },
}

/// unique identifier to a buffer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct BufferId(pub &'static str);

/// unique identifier for a texture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct TextureId(pub &'static str);

/// unique identifier for a sampler
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct SamplerId(pub &'static str);

/// unique identifier for a pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct PipelineId(pub &'static str);

/// unique identifier for a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct BindGroupId(pub &'static str);

/// unique identifier for a bind group layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct LayoutId(pub &'static str);

/// Represents the state of the gpu, providing means to create and modify resources, and execute pipelines
pub struct GpuContext {
    pub(crate) gpu: GpuHandle,
    cmds: Vec<GpuCommand>,
    executor: GpuExecutor,

    pub(crate) buffers: ResourceHandler<BufferId, BufferHandle>,
    pub(crate) textures: ResourceHandler<TextureId, TextureHandle>,
    pub(crate) samplers: ResourceHandler<SamplerId, SamplerHandle>,
    pub(crate) bg_registry: BindGroupRegistry,
    pub(crate) pip_registry: PipelineRegistry,
}

impl GpuContext {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            cmds: Vec::new(),
            executor: GpuExecutor::new(),

            buffers: ResourceHandler::new(),
            textures: ResourceHandler::new(),
            samplers: ResourceHandler::new(),
            bg_registry: BindGroupRegistry::new(gpu.clone()),
            pip_registry: PipelineRegistry::new(gpu.clone()),
            gpu
        }
    }

    /// Reconfigure the surface texture configuration to match the canvas
    pub fn configure_surface(&self, canvas: &mut Canvas) {
        canvas.surface.configure(&self.gpu.device, &canvas.config);
    }

    /// Request a buffer to be created from the provided builder and mapped to the provided id.
    pub fn request_buffer(&mut self, id: &BufferId, builder: Buffer) {
        if self.buffers.contains(id) { return; }

        let gpu = self.gpu.clone();
        let buffer_task = Task::non_blocking( async move {
            gpu.create_buffer(builder)
        });
        self.buffers.request_new(id, buffer_task);
    }

    /// Request a texture to be created from the provided builder and mapped to the provided id.
    pub fn request_texture(&mut self, id: &TextureId, builder: Texture) {
        if self.textures.contains(id) { return; }

        let gpu = self.gpu.clone();
        let texture_task = Task::non_blocking(async move {
            gpu.create_texture(builder)
        });
        self.textures.request_new(id, texture_task);
    }

    /// Request a sampler to be created from the provided builder and mapped to the provided id.
    pub fn request_sampler(&mut self, id: &SamplerId, builder: Sampler) {
        if self.samplers.contains(id) { return; }

        let gpu = self.gpu.clone();
        let sampler_task = Task::non_blocking(async move {
            gpu.create_sampler(builder)
        });
        self.samplers.request_new(id, sampler_task);
    }

    /// Request a bind group to be created from the provided builder and mapped to the provided id.
    pub fn request_bind_group(&mut self, bg_id: &BindGroupId, bgl_id: &LayoutId, builder: &BindGroup) {
        self.bg_registry.request_bg(bg_id, bgl_id, builder, &self.buffers, &self.textures, &self.samplers);
    }

    /// Request a pipeline to be created from the provided builder and mapped to the provided id.
    pub fn request_pipeline(&mut self, id: &PipelineId, builder: &Pipeline) {
        self.pip_registry.request(id, &builder, &self.bg_registry);
    }

    /// Copy a texture into to another one, overwriting it's data 
    pub fn copy_textures(&self, src_id: &TextureId, dst_id: &TextureId) {
        GpuExecutor::copy_textures(self, src_id, dst_id);
    }

    /// Prepare the context for the next frame
    pub fn prepare_frame(&mut self) {
        self.buffers.sync();
        self.textures.sync();
        self.samplers.sync();
        self.bg_registry.sync(&self.buffers, &self.textures, &self.samplers);
        self.pip_registry.sync(&self.bg_registry);
    }

    /// Update a buffer with the provided id, if found. The data payload must not exceed the buffer size
    pub fn update_buffer(&mut self, id: &BufferId, update: impl BufferUpdate) {
        if let Some(buffer) = self.buffers.get(id) {
            let data = update.bytes();
            let offset = update.offset();

            let update_size = offset + data.len() as u64;
            assert!(update_size <= buffer.size());

            self.gpu.queue.write_buffer(buffer, offset, data);
        }
    }

    /// Remove a texture from the context, releasing the vram allocation
    pub fn remove_texture(&mut self, id: &TextureId) {
        self.textures.remove(id);
        self.executor.invalidate_texture(id, self);
    }

    /// Remove a buffer from the context, releasing the vram allocation
    pub fn remove_buffer(&mut self, id: &BufferId) {
        self.buffers.remove(id);
        self.executor.invalidate_buffer(id, self);
    }

    /// Remove a bind group from the context, releasing the vram allocation
    pub fn remove_bind_group(&mut self, id: &BindGroupId) {
        self.bg_registry.remove(id);
        self.executor.invalidate_bind_group(id);
    }

    /// Remove a pipeline from the context, releasing the vram allocation
    pub fn remove_pipeline(&mut self, id: &PipelineId) {
        self.pip_registry.remove(id);
        self.executor.invalidate_pipeline(id);
    }

    /// Add a render/compute pass the the context's pass queue
    pub fn add_command(&mut self, pass: GpuCommand) {
        self.cmds.push(pass);
    }

    /// Execute the gpu commands added to the command queue.
    pub fn finish(&mut self, canvas: &Canvas) -> Result<(), wgpu::SurfaceError> {
        let output = canvas.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let format = canvas.config.format;

        let cmds = std::mem::take(&mut self.cmds);
        self.executor.execute(self, cmds, view);
        output.present();

        Ok(())
    }
}
