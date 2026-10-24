use std::{collections::HashSet};
use crate::{graphics::{pip_registry::PipelineRegistry, *}, utils::{HoldPolicy, ResourceHandler, Task}};

/// The hold policy for all gpu resources globally. All resources have an indefinite hold policy by default.
/// 
/// These are followed internally by the context unless a resource is configured otherwise.
#[derive(Debug, Clone, Copy)]
pub struct GlobalHoldPolicy {
    pub buffers: HoldPolicy,
    pub textures: HoldPolicy,
    pub samplers: HoldPolicy,
    pub bind_groups: HoldPolicy,
    pub pipelines: HoldPolicy,
}

impl Default for GlobalHoldPolicy {
    fn default() -> Self {
        Self {
            buffers: HoldPolicy::Indefinite,
            textures: HoldPolicy::Indefinite,
            samplers: HoldPolicy::Indefinite,
            bind_groups: HoldPolicy::Indefinite,
            pipelines: HoldPolicy::Indefinite,
        }
    }
}

/// Represents types that can be identified as a gpu resource
pub trait ResourceId {
    /// The id type mapped to the `ResourceId` type
    type Id;

    /// Create a new id as the associated type `Self::Id`
    fn create_id(&self) -> Self::Id;
}

/// The low level gpu resources (Buffers, Textures and Samplers)
pub(crate) struct GpuResources {
    pub(crate) buffers: ResourceHandler<BufferId, wgpu::Buffer, String>,
    pub(crate) textures: ResourceHandler<TextureId, TextureHandle, String>,
    pub(crate) samplers: ResourceHandler<SamplerId, wgpu::Sampler, String>
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

/// A container and lifecycle manager for gpu resources, providing means to create, modify, and remove them.
/// 
/// This handles lifecycles of gpu resources through internal, timer-based async handlers, where resources are removed 
/// automatically if they are not used, based on a hold policy. All resources are therefore created asynchronously, and kept alive for as long as they are used, unless specified otherwise.
/// Hold policies can be globally configured or specified per resource through their blueprints. All resources have an indefinite hold time by default (meaning they stay alive indefinitely unless manually removed)
/// 
/// Blueprints are builder-like descriptor structs that describe the purpose, usage, dependencies, and lifetimes of a wpgu resource.
/// They are meant to be lightweight and therefore can be cloned, copied, and used as keys in `HashMap`s.
/// Buffer and texture blueprints have associated source types that can be used to prepopulate them with initial data.
/// 
/// Resources that have dependencies (like bind groups and pipelines) are created as soon as the dependencies themselves are created at the next call to `sync()`.
/// This allows for runtime validation, eliminating many potential execution bugs that could arise due to failed resource creations.
/// 
/// Executors that use this context should (and only can) access raw wgpu handles through the validation system. This ensures that executing draw and compute commands
/// only happen if their depenenencies are ready. 
/// 
/// All resources are mapped to ids that are either universally unique (as is the case for buffers and textures), or content-addressable (as is the case for bind group layouts and pipelines).
/// These ids are lightweight and clonable, allowing any system using this `GpuContext` to easily send ids across threads or own directly.
/// 
/// This struct makes no assumptions about the data that lives within the resources, meaning that memory management is still up to the user.
pub struct GpuContext {
    pub(crate) gpu: GpuHandle,
    pub(crate) resources: GpuResources,
    pub(crate) bg_registry: BindGroupRegistry,
    pub(crate) pip_registry: PipelineRegistry,
    pub(crate) global_hp: GlobalHoldPolicy,
}

impl GpuContext {
    /// Create a new `GpuContext` from a `GpuHandle`.
    pub fn new(gpu: GpuHandle, global_hp: GlobalHoldPolicy) -> Self {
        Self {
            resources: GpuResources::new(),
            bg_registry: BindGroupRegistry::new(gpu.clone()),
            pip_registry: PipelineRegistry::new(gpu.clone()),
            global_hp,
            gpu
        }
    }

    /// Get a reference to the `GpuHandle` used by this `GpuContext`
    pub fn gpu_handle(&self) -> &GpuHandle {
        &self.gpu
    }

    /// Request a buffer to be created from the provided blueprint.
    /// 
    /// Returns a `BufferId` that is uniquely associated with the resultant wgpu `Buffer`.
    pub fn request_buffer<S>(&mut self, blueprint: Buffer, source: BufferSource<S>) -> BufferId {
        let id = blueprint.create_id();

        let policy = blueprint.hold_policy.unwrap_or(self.global_hp.buffers);
        
        let gpu = self.gpu.clone();
        let buffer_task = Task::io_bound( async move {
            gpu.create_buffer(blueprint, source.data, source.capacity)
        }).with_hold_policy(policy);

        self.resources.buffers.request(id, buffer_task);

        id
    }

    /// Ensures a buffer has been requested.
    /// 
    /// If `test_id` maps to an already requested buffer, it's returned back to the caller. Otherwise a new buffer is requested from `blueprint` and `source`, and a new `BufferId` is returned. 
    pub fn ensure_buffer<S>(&mut self, test_id: BufferId, blueprint: Buffer, source: BufferSource<S>) -> BufferId {
        if self.is_buffer_requested(test_id) { return test_id };
        self.request_buffer(blueprint, source)
    }

    /// Request a texture to be created from the provided blueprint and source.
    /// 
    /// Returns a `TextureId` that is uniquely associated with the resultant wgpu `Texture`.
    pub fn request_texture(&mut self, blueprint: Texture, source: impl TextureSource) -> TextureId {
        let id = blueprint.create_id();

        let policy = blueprint.hold_policy.unwrap_or(self.global_hp.buffers);

        let gpu = self.gpu.clone();
        let texture_task = Task::io_bound(async move {
            gpu.create_texture(blueprint, source)
        }).with_hold_policy(policy);

        self.resources.textures.request(id, texture_task);

        id
    }

    /// Ensures a texture has been requested.
    /// 
    /// If `test_id` maps to an already requested texture, it's returned back to the caller. Otherwise a new texture is requested from `blueprint` and `source`, and a new `TextureId` is returned. 
    pub fn ensure_texture(&mut self, test_id: TextureId, blueprint: Texture, source: impl TextureSource) -> TextureId {
        if self.is_texture_requested(test_id) { return test_id };
        self.request_texture(blueprint, source)
    }

    /// Request a sampler to be created from the provided blueprint.
    /// 
    /// Returns a `SamplerId` that is content-addressable to the blueprint. (This ensures that identical sampler blueprints map to the same id)
    pub fn request_sampler<B>(&mut self, blueprint: B) -> SamplerId 
    where 
        B: SamplerType + ResourceId<Id = SamplerId>
    {
        let id = blueprint.create_id();
        if self.resources.samplers.contains(&id) { return id; }

        let policy = blueprint.hold_policy().unwrap_or(self.global_hp.buffers);

        let gpu = self.gpu.clone();
        let sampler_task = Task::io_bound(async move {
            gpu.create_sampler(blueprint)
        }).with_hold_policy(policy);

        self.resources.samplers.request(id, sampler_task);

        id
    }

    /// Request a bind group to be created from the provided blueprint
    /// 
    /// Returns a `BindGroupIdPair`, which contains a unique `BindGroupId` and content-addressable `LayoutId`.
    pub fn request_bind_group(&mut self, blueprint: BindGroup) -> BindGroupIdPair {
        return self.bg_registry.request_bg(blueprint, &self.resources, self.global_hp.bind_groups);
    }

    /// Ensures a bind group has been requested.
    /// 
    /// If `test_pair` maps to an already requested bind group, it's returned back to the caller. Otherwise a new bind group is requested from `blueprint`, and a new `BindGroupIdPair` is returned. 
    /// 
    /// Note: If the layout the bind group uses was already requested before, it's `LayoutId` will be the same regardless of whether the bind group is requested new.
    pub fn ensure_bind_group(&mut self, test_pair: BindGroupIdPair, blueprint: BindGroup) -> BindGroupIdPair {
        if self.is_bindgroup_requested(&test_pair.id) { return test_pair};
        self.request_bind_group(blueprint)
    }

    /// Request a pipeline to be created from the provided blueprint.
    /// 
    /// Returns a `PipelineId` that is content-addressable to the blueprint. (This ensures that identical pipeline blueprints map to the same id)
    pub fn request_pipeline(&mut self, blueprint: impl Into<PipelineType>) -> PipelineId {
        return self.pip_registry.request(blueprint.into(), &self.bg_registry, self.global_hp.pipelines);
    }

    /// Sync pending resources with the main thread. This should be called regularly.
    pub fn sync(&mut self) {
        self.resources.sync();
        self.bg_registry.sync(&self.resources, self.global_hp.bind_groups);
        self.pip_registry.sync(&self.bg_registry, self.global_hp.pipelines);
    }

    /// Update a buffer with the provided id, if found.
    /// 
    /// Returns an error if the `update` data exceeds the buffer's capacity.
    pub fn update_buffer(&mut self, id: &BufferId, update: impl BufferUpdate) -> Result<(), String>{
        if let Some(buffer) = self.resources.buffers.get(id) {
            let data = update.bytes();
            let offset = update.offset();

            let update_size = offset + data.len() as u64;
            if update_size > buffer.size() {
                return Err(format!("[GpuContext] Data payload exceeds the buffer size."));
            }

            self.gpu.queue.write_buffer(buffer, offset, data);
        }

        Ok(())
    }

    /// Remove a texture from the context, releasing the allocation from gpu memory. This also causes any bind group that used it to become invalid.
    pub fn remove_texture(&mut self, id: &TextureId) {
        self.resources.textures.remove(id);

        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in &self.bg_registry.bg_blueprints {
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

        println!("[GpuContext] Removed Texture with label '{:?}'", id);
    }

    /// Remove a buffer from the context, releasing the allocation from gpu memory. This also causes any bind group that used it to become invalid.
    pub fn remove_buffer(&mut self, id: &BufferId) {
        self.resources.buffers.remove(id);
        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in &self.bg_registry.bg_blueprints {
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

        println!("[GpuContext] Removed Buffer with label '{:?}'", id);
    }

    /// Remove a sampler from the context, releasing the allocation from gpu memory. This also causes any bind group that used it to become invalid.
    pub fn remove_sampler(&mut self, id: &SamplerId) {
        self.resources.samplers.remove(id);
        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in &self.bg_registry.bg_blueprints {
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

        println!("[GpuContext] Removed Sampler with label '{:?}'", id);
    }

    /// Remove a bind group from the context, releasing the vram allocation.
    /// 
    /// This decrements the associated bind group layout's reference count. If this was the last resource that used the layout,
    /// (i.e. no other pipeline or bind group references it), then the layout is also removed.
    pub fn remove_bind_group(&mut self, bg_id: &BindGroupId) {
        self.bg_registry.remove_bg(bg_id);

        println!("[GpuContext] Removed Bind Group with label '{:?}'", bg_id);
    }

    /// Remove a pipeline from the context, releasing the vram allocation.
    ///
    /// This decrements the associated bind group layout's reference count. If this was the last resource that used the layout,
    /// (i.e. no other pipeline or bind group references it), then the layout is also removed.
    pub fn remove_pipeline(&mut self, id: &PipelineId) {
        self.pip_registry.remove(id, &mut self.bg_registry);

        println!("[GpuContext] Removed Pipeline with label '{:?}'", id);
    }

    /// Validates a bind group. This ensures it can be used in a gpu command. Returns a reference to the bind group if validation passed.
    /// 
    /// You can call this prior to rendering to speed up initial gameplay frames, but note that it will only work if
    /// the dependent resources are already ready.
    ///
    /// Note that any changes to underlying resources will invalidate the bind group, triggering revalidation on the next usage.
    pub fn validate_bind_group(&self, bg_id: &BindGroupId) -> Option<BindGroupHandle> {
        self.bg_registry.validate(bg_id, &self.resources)
    }

    /// Validates a pipeline. This ensures it can be used in future gpu commands. Returns a reference to the pipeline handle if validation passed.
    /// 
    /// You can call this prior to rendering to speed up initial gameplay frames, but note that it will only work if
    /// the dependent resources are already ready.
    /// 
    /// Note that any changes to underlying resources will invalidate the pipeline, triggering revalidation on the next usage.
    pub fn validate_pipeline(&self, pip_id: &PipelineId) -> Option<&PipelineHandle> {
        self.pip_registry.validate(pip_id, &self.bg_registry)
    }

    /// Returns `true` if a buffer mapped to `buf_id` exists or is pending completion, `false` otherwise
    pub fn is_buffer_requested(&self, buf_id: BufferId) -> bool {
        self.resources.buffers.mark_accessed(&buf_id);
        self.resources.buffers.is_requested(&buf_id)
    }

    /// Returns `true` if a texture mapped to `tex_id` exists or is pending completion, `false` otherwise
    pub fn is_texture_requested(&self, tex_id: TextureId) -> bool {
        self.resources.textures.mark_accessed(&tex_id);
        self.resources.textures.is_requested(&tex_id)
    }

    /// Returns `true` if a bind group mapped to `bg_id` exists or is pending completion, `false` otherwise
    pub fn is_bindgroup_requested(&self, bg_id: &BindGroupId) -> bool {
        self.bg_registry.mark_bg_accessed(bg_id, &self.resources);
        self.bg_registry.is_bg_requested(bg_id)
    }
}
