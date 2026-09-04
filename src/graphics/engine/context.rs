use std::{collections::HashSet, println};

use crate::graphics::*;

/// Represents a render pass
#[derive(Clone, Debug)]
pub struct RenderPassInfo {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub vertex_buffers: Vec<BufferId>,
    pub index_buffer: Option<BufferId>,
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

/// Helper struct encapsulating the id of a bind group and it's associated layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NamedBindGroup {
    pub layout_id: LayoutId,
    pub id: BindGroupId
}

impl NamedBindGroup {
    pub fn new(name: &'static str) -> Self {
        Self {
            id: BindGroupId(name),
            layout_id: LayoutId(
                Box::leak(Box::new(format!("{name}_layout")))
            )
        }
    }
}

/// The low level gpu resources as used in bind groups.
pub(crate) struct GpuResources {
    pub(crate) buffers: ResourceHandler<BufferId, wgpu::Buffer>,
    pub(crate) textures: ResourceHandler<TextureId, TextureHandle>,
    pub(crate) samplers: ResourceHandler<SamplerId, wgpu::Sampler>
}

impl GpuResources {
    pub fn new() -> Self {
        Self {
            buffers: ResourceHandler::new(),
            textures: ResourceHandler::new(),
            samplers: ResourceHandler::new()
        }
    }

    /// sync the internal handlers with the main thread
    pub fn sync(&mut self) {
        self.buffers.sync();
        self.textures.sync();
        self.samplers.sync();
    }
}

/// Represents the state of the gpu, providing means to create and modify resources, and execute pipelines
pub struct GpuContext {
    pub(crate) gpu: GpuHandle,
    pub(crate) resources: GpuResources,
    pub(crate) bg_registry: BindGroupRegistry,
    pub(crate) pip_registry: PipelineRegistry,
}

impl GpuContext {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            resources: GpuResources::new(),
            bg_registry: BindGroupRegistry::new(gpu.clone()),
            pip_registry: PipelineRegistry::new(gpu.clone()),
            gpu
        }
    }

    /// Reconfigure the surface texture configuration to match the canvas
    pub fn configure_surface(&self, canvas: &mut Canvas) {
        canvas.surface.configure(&self.gpu.device, &canvas.config);
    }

    /// Request a buffer to be created from the provided definition and mapped to the provided id.
    pub fn request_buffer(&mut self, id: &BufferId, buffer_def: Buffer) {
        if self.resources.buffers.contains(id) { return; }

        let gpu = self.gpu.clone();
        let buffer_task = Task::non_blocking( async move {
            gpu.create_buffer(buffer_def)
        });
        self.resources.buffers.request_new(id, buffer_task);
    }

    /// Request a texture to be created from the provided definition and mapped to the provided id.
    pub fn request_texture(&mut self, id: &TextureId, texture_def: Texture) {
        if self.resources.textures.contains(id) { return; }

        let gpu = self.gpu.clone();
        let texture_task = Task::non_blocking(async move {
            gpu.create_texture(texture_def)
        });
        self.resources.textures.request_new(id, texture_task);
    }

    /// Request a sampler to be created from the provided definition and mapped to the provided id.
    pub fn request_sampler(&mut self, id: &SamplerId, sampler_def: Sampler) {
        if self.resources.samplers.contains(id) { return; }

        let gpu = self.gpu.clone();
        let sampler_task = Task::non_blocking(async move {
            gpu.create_sampler(sampler_def)
        });
        self.resources.samplers.request_new(id, sampler_task);
    }

    /// Request a bind group to be created from the provided definition and mapped to the provided id.
    pub fn request_bind_group(&mut self, bg_id: &BindGroupId, bgl_id: &LayoutId, bg_def: &BindGroup) {
        self.bg_registry.request_bg(bg_id, bgl_id, bg_def, &self.resources);
    }

    /// Request a pipeline to be created from the provided definition and mapped to the provided id.
    pub fn request_pipeline(&mut self, id: &PipelineId, pip_def: &Pipeline) {
        self.pip_registry.request(id, &pip_def, &self.bg_registry);
    }

    /// Copy a texture into to another one, overwriting it's data 
    pub fn copy_textures(&self, src_id: &TextureId, dst_id: &TextureId) {
        GpuExecutor::copy_textures(self, src_id, dst_id);
    }

    /// Sync pending resources with the main thread. This should be called regularly in frame-based applications
    pub fn sync(&mut self) {
        self.resources.sync();
        self.bg_registry.sync(&self.resources);
        self.pip_registry.sync(&self.bg_registry);
    }

    /// Update a buffer with the provided id, if found. The data payload must not exceed the buffer size
    pub fn update_buffer(&mut self, id: &BufferId, update: impl BufferUpdate) {
        if let Some(buffer) = self.resources.buffers.get(id) {
            let data = update.bytes();
            let offset = update.offset();

            let update_size = offset + data.len() as u64;
            assert!(update_size <= buffer.size());

            self.gpu.queue.write_buffer(buffer, offset, data);
        }
    }

    /// Remove a texture from the context, releasing the allocation from gpu memory. This also causes any bind group that used it to become invalid.
    pub fn remove_texture(&mut self, id: &TextureId) {
        self.resources.textures.remove(id);

        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in &self.bg_registry.bg_defs {
            for entry in &bg_blueprint.bindings {
                if let BindingTarget::Texture(tex_id) = &entry.target {
                    if tex_id == id {
                        invalid_bgs.insert(bd_id.clone());
                        continue;
                    }
                }
            }
        }

        for bg_id in &invalid_bgs {
            self.bg_registry.invalidate(bg_id);
        }

        println!("Removed Texture with label '{:?}'", id);
    }

    /// Remove a buffer from the context, releasing the allocation from gpu memory. This also causes any bind group that used it to become invalid.
    pub fn remove_buffer(&mut self, id: &BufferId) {
        self.resources.buffers.remove(id);
        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in &self.bg_registry.bg_defs {
            for entry in &bg_blueprint.bindings {
                if let BindingTarget::Buffer(buf_id) = &entry.target {
                    if buf_id == id {
                        invalid_bgs.insert(*bd_id);
                        continue;
                    }
                }
            }
        }

        for bg_id in &invalid_bgs {
            self.bg_registry.invalidate(bg_id);
        }

        println!("Removed Buffer with label '{:?}'", id);
    }

    /// Remove a sampler from the context, releasing the allocation from gpu memory. This also causes any bind group that used it to become invalid.
    pub fn remove_sampler(&mut self, id: &SamplerId) {
        self.resources.samplers.remove(id);
        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in &self.bg_registry.bg_defs {
            for entry in &bg_blueprint.bindings {
                if let BindingTarget::Sampler(samp_id) = &entry.target {
                    if samp_id == id {
                        invalid_bgs.insert(*bd_id);
                        continue;
                    }
                }
            }
        }

        for bg_id in &invalid_bgs {
            self.bg_registry.invalidate(bg_id);
        }

        println!("Removed Buffer with label '{:?}'", id);
    }

    /// Remove a bind group from the context, releasing the vram allocation
    pub fn remove_bind_group(&mut self, bg_id: &BindGroupId) {
        if let Some(bg_handle) = self.bg_registry.bg_handles.get(bg_id).cloned() {
            self.bg_registry.check_dec_bgl(&bg_handle.layout_id);
        }
        self.bg_registry.remove(bg_id);

        println!("Removed Bind Group with label '{:?}'", bg_id);
    }

    /// Remove a pipeline from the context, releasing the vram allocation
    pub fn remove_pipeline(&mut self, id: &PipelineId) {
        if let Some(pip) = self.pip_registry.get_blueprint(id) {
            for layout_id in &pip.bg_layouts {
                self.bg_registry.check_dec_bgl(layout_id);
            }
        }

        self.pip_registry.remove(id);

        println!("Removed Pipeline with label '{:?}'", id);
    }

    /// Validate a bind group, ensuring it can be used in a gpu command
    pub(crate) fn validate_bind_group(&self, bg_id: &BindGroupId) -> Option<BindGroupHandle> {
        self.bg_registry.validate(bg_id, &self.resources)
    }

    /// Validate a pipeline, ensuring it can be used in a gpu command
    pub(crate) fn validate_pipeline(&self, pip_id: &PipelineId) -> Option<&PipelineHandle> {
        self.pip_registry.validate(pip_id, &self.bg_registry)
    }
}
