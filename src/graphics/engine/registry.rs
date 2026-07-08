use std::collections::HashMap;

use crate::graphics::*;

/// Keeps track of and validates requests for bind groups
pub struct BindGroupRegistry {
    gpu: GpuHandle,
    /// The handles to bind groups
    handles: ResourceHandler<BindGroupId, BindGroupHandle>,
    /// maps bind groups to their blueprints
    blueprints: HashMap<BindGroupId, BindGroup>,
    /// map of ids of bind group that have yet to pass request validation
    deffered: HashMap<BindGroupId, BindGroup>,
}

impl BindGroupRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            handles: ResourceHandler::new(),
            blueprints: HashMap::new(),
            deffered: HashMap::new(),
        }
    }

    /// Request a new bind group
    pub fn request<'a>(
        &mut self,
        id: &BindGroupId,
        builder: &BindGroup,
        buffers: &'a ResourceHandler<BufferId, BufferHandle>,
        textures: &'a ResourceHandler<TextureId, TextureHandle>,
    ) {
        if self.handles.contains(id) { return; }

        if !self.blueprints.contains_key(id) {
            self.blueprints.insert(*id, builder.clone());
        }

        let mut buffer_handles = Vec::new();
        let mut texture_handles = Vec::new();
        let mut expected_tex_len = 0usize;
        let mut expected_buf_len = 0usize;

        for binding in &builder.bindings {
            match &binding.target {
                BindingTarget::Buffer(buf_id) => {
                    if let Some(handle) = buffers.get(buf_id) {
                        buffer_handles.push((*buf_id, handle.clone(), binding.slot));
                    }
                    expected_buf_len += 1;
                },
                BindingTarget::Texture(tex_id) => {
                    if let Some(handle) = textures.get(tex_id) {
                        texture_handles.push((*tex_id, handle.clone(), binding.slot));
                    }
                    expected_tex_len += 1;
                }
            }
        }

        let ok_buffers = expected_buf_len == buffer_handles.len();
        let ok_textures = expected_tex_len == texture_handles.len();

        if !(ok_buffers && ok_textures) { 
            self.deffered.insert(*id, builder.clone());
            return; 
        };

        self.deffered.remove(id);

        let gpu = self.gpu.clone();
        let builder = builder.clone();

        let bind_group_task = Task::non_blocking(async move {
            let mut entries = Vec::new();

            for (_id, buf, slot) in &buffer_handles {
                entries.push(wgpu::BindGroupEntry {
                    binding: *slot,
                    resource: buf.as_entire_binding()
                });
            }
            for (_id, tex, slot) in &texture_handles {
                entries.push(wgpu::BindGroupEntry {
                    binding: *slot,
                    resource: wgpu::BindingResource::TextureView(tex)
                });
            }

            create_bind_group(gpu, builder, entries).await
        });

        self.handles.request_new(id, bind_group_task);
    }

    /// sync the registry and process defferred groups
    pub fn sync<'a>(
        &mut self,
        buffers: &'a ResourceHandler<BufferId, BufferHandle>,
        textures: &'a ResourceHandler<TextureId, TextureHandle>,
    ) {
        self.handles.sync();

        let pending_bgs = std::mem::take(&mut self.deffered);
        for (id, builder) in &pending_bgs {
            self.request(id, builder, buffers, textures);
        }
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, id: &BindGroupId) {
        self.handles.remove(id);
        self.blueprints.remove(id);
        self.deffered.remove(id);
    }

    /// returns a clone of the handle to a stored bind group
    pub fn get_cloned(&self, id: &BindGroupId) -> Option<BindGroupHandle> {
        return self.handles.get(id).cloned()
    }

    pub fn get_blueprints(&self) -> &HashMap<BindGroupId, BindGroup> {
        &self.blueprints
    }

    pub fn get_blueprint(&self, id: &BindGroupId) -> Option<&BindGroup> {
        self.blueprints.get(id)
    }
}

/// Keeps track of and validates requests for render/compute pipelines
pub struct PipelineRegistry {
    gpu: GpuHandle,
    handles: ResourceHandler<PipelineId, PipelineHandle>,
    blueprints: HashMap<PipelineId, Pipeline>,
    deferred: HashMap<PipelineId, Pipeline>
}

impl PipelineRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            handles: ResourceHandler::new(),
            blueprints: HashMap::new(),
            deferred: HashMap::new()
        }
    }

    /// request a new pipeline
    pub fn request<'a>(
        &mut self,
        id: &PipelineId,
        builder: &Pipeline,
        bind_groups: &'a BindGroupRegistry,
    ) {
        if self.handles.contains(id) { return; }

        if !self.blueprints.contains_key(id) {
            self.blueprints.insert(*id, builder.clone());
        }

        let mut bg_layouts = Vec::new();
        for bg_id in &builder.bg_layouts {
            if let Some(bind_group) = bind_groups.get_cloned(bg_id) {
                bg_layouts.push(bind_group.layout.clone())
            }
        }

        if builder.bg_layouts.len() != bg_layouts.len() {
            self.deferred.insert(*id, builder.clone());
            return; 
        }

        self.deferred.remove(id);
        match builder.pip_type {
            PipelineType::Render(ty) => {
                let r_pip_task = Task::non_blocking(
                    create_render_pipeline(self.gpu.clone(), builder.clone(), ty, bg_layouts)
                );

                self.handles.request_new(id, r_pip_task);
            },
            PipelineType::Compute(ty) => {
                let c_pip_task = Task::non_blocking(
                    create_compute_pipeline(self.gpu.clone(), builder.clone(), ty, bg_layouts)
                );
                self.handles.request_new(id, c_pip_task);
            }
        };
    }

    /// sync the registry and process defferred groups
    pub fn sync<'a>(&mut self, bind_groups: &'a BindGroupRegistry) {
        self.handles.sync();

        let pending_bgs = std::mem::take(&mut self.deferred);
        for (id, builder) in &pending_bgs {
            self.request(id, builder, bind_groups);
        }
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, id: &PipelineId) {
        self.handles.remove(id);
        self.blueprints.remove(id);
        self.deferred.remove(id);
    }

    /// returns a clone of the handle to a stored render pipeline
    pub fn get_render_handle(&self, id: &PipelineId) -> Option<wgpu::RenderPipeline> {
        return self.handles.get(id)?.as_render()
    }

    /// returns a clone of the handle to a stored compute pipeline
    pub fn get_compute_handle(&self, id: &PipelineId) -> Option<wgpu::ComputePipeline> {
        return self.handles.get(id)?.as_compute()
    }

    pub fn get_blueprints(&self) -> &HashMap<PipelineId, Pipeline> {
        &self.blueprints
    }

    pub fn get_blueprint(&self, id: &PipelineId) -> Option<&Pipeline> {
        self.blueprints.get(id)
    }
}