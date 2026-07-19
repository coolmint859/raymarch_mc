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
pub enum PassInfo {
    Render(RenderPassInfo),
    Compute(ComputePassInfo),
}

/// unique identifier to a buffer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct BufferId(pub &'static str);

/// unique identifier for a texture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct TextureId(pub &'static str);

/// unique identifier for a pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct PipelineId(pub &'static str);

/// unique identifier for a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct BindGroupId(pub &'static str);

/// Represents the state of the gpu, providing means to create and modify resources, and execute pipelines
pub struct GpuContext {
    gpu: GpuHandle,
    pass_queue: Vec<PassInfo>,
    executor: PassExecutor,

    pub(crate) buffers: ResourceHandler<BufferId, BufferHandle>,
    pub(crate) textures: ResourceHandler<TextureId, TextureHandle>,
    pub(crate) bg_registry: BindGroupRegistry,
    pub(crate) pip_registry: PipelineRegistry,
}

impl GpuContext {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            pass_queue: Vec::new(),
            executor: PassExecutor::new(gpu.clone()),

            buffers: ResourceHandler::new(),
            textures: ResourceHandler::new(),
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
            gpu.create_buffer(builder).await
        });
        self.buffers.request_new(id, buffer_task);
    }

    /// Request a texture to be created from the provided builder and mapped to the provided id.
    pub fn request_texture(&mut self, id: &TextureId, builder: Texture) {
        if self.textures.contains(id) { return; }

        let gpu = self.gpu.clone();
        let texture_task = Task::non_blocking(async move {
            gpu.create_texture(builder).await
        });
        self.textures.request_new(id, texture_task);
    }

    /// Request a bind group to be created from the provided builder and mapped to the provided id.
    pub fn request_bind_group(&mut self, id: &BindGroupId, builder: &BindGroup) {
        self.bg_registry.request(id, builder, &self.buffers, &self.textures);
    }

    /// Request a pipeline to be created from the provided builder and mapped to the provided id.
    pub fn request_pipeline(&mut self, id: &PipelineId, builder: &Pipeline) {
        self.pip_registry.request(id, &builder, &self.bg_registry);
    }

    /// Prepare the context for the next frame
    pub fn prepare_frame(&mut self) {
        self.buffers.sync();
        self.textures.sync();
        self.bg_registry.sync(&self.buffers, &self.textures);
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
        self.executor.invalidate_bind_group(id, self);
    }

    /// Remove a pipeline from the context, releasing the vram allocation
    pub fn remove_pipeline(&mut self, id: &PipelineId) {
        self.pip_registry.remove(id);
        self.executor.invalidate_pipeline(id);
    }

    /// Add a render/compute pass the the context's pass queue
    pub fn add_pass(&mut self, pass: PassInfo) {
        self.pass_queue.push(pass);
    }

    /// Execute the gpu passes added to the pass queue.
    pub fn finish(&mut self, canvas: &Canvas) -> Result<(), wgpu::SurfaceError> {
        let output = canvas.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let format = canvas.config.format;

        let passes = std::mem::take(&mut self.pass_queue);
        self.executor.execute(self, passes, view);
        output.present();

        Ok(())
    }
}
