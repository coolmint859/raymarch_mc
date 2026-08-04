use std::collections::HashMap;

use crate::graphics::*;

/// Keeps track of and validates requests for bind groups
pub struct BindGroupRegistry {
    gpu: GpuHandle,
    /// The handles to bind groups
    bg_handles: ResourceHandler<BindGroupId, BindGroupHandle>,
    /// The handles to bind group layouts
    layout_handles: ResourceHandler<LayoutId, BindGroupLayoutHandle>,
    /// maps bind groups to their blueprints
    blueprints: HashMap<BindGroupId, BindGroup>,
    /// map of ids of bind group that have yet to pass request validation
    deffered: HashMap<BindGroupId, (LayoutId, BindGroup)>,
}

impl BindGroupRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            bg_handles: ResourceHandler::new(),
            layout_handles: ResourceHandler::new(),
            blueprints: HashMap::new(),
            deffered: HashMap::new(),
        }
    }

    /// Request a new bind group layout
    pub fn request_layout(
        &mut self,
        id: &LayoutId,
        builder: &BindGroup
    ) {
        if self.layout_handles.contains(id) { return; };

        let gpu = self.gpu.clone();
        let builder = builder.clone();

        let layout_task = Task::non_blocking(async move {
            gpu.create_bg_layout(builder)
        });

        self.layout_handles.request_new(id, layout_task);
    }

    /// Request a new bind group
    pub fn request_bg<'a>(
        &mut self,
        bg_id: &BindGroupId,
        layout_id: &LayoutId,
        builder: &BindGroup,
        buffers: &'a ResourceHandler<BufferId, BufferHandle>,
        textures: &'a ResourceHandler<TextureId, TextureHandle>,
        samplers: &'a ResourceHandler<SamplerId, SamplerHandle>,
    ) {
        if self.bg_handles.contains(bg_id) { return; }

        let layout = match self.layout_handles.get(layout_id) {
            Some(bg_layout) => bg_layout.clone(),
            None => {
                self.deffered.insert(*bg_id, (*layout_id, builder.clone()));
                self.request_layout(layout_id, builder);
                return; 
            }
        };

        if !self.blueprints.contains_key(bg_id) {
            self.blueprints.insert(*bg_id, builder.clone());
        }

        let mut buffer_handles = Vec::new();
        let mut texture_handles = Vec::new();
        let mut sampler_handles = Vec::new();
        
        let mut expected_buf_len = 0usize;
        let mut expected_tex_len = 0usize;
        let mut expected_samp_len = 0usize;

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
                BindingTarget::Sampler(samp_id) => {
                    if let Some(handle) = samplers.get(samp_id) {
                        sampler_handles.push((*samp_id, handle.clone(), binding.slot));
                    }
                    expected_samp_len += 1;
                }
            }
        }

        let ok_buffers = expected_buf_len == buffer_handles.len();
        let ok_textures = expected_tex_len == texture_handles.len();
        let ok_samplers = expected_samp_len == sampler_handles.len();

        if !(ok_buffers && ok_textures && ok_samplers) {
            self.deffered.insert(*bg_id, (*layout_id, builder.clone()));
            return; 
        };

        self.deffered.remove(bg_id);

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
            for (_id, samp, slot) in &sampler_handles {
                entries.push(wgpu::BindGroupEntry {
                    binding: *slot,
                    resource: wgpu::BindingResource::Sampler(samp)
                });
            }

            gpu.create_bind_group(builder, entries, layout)
        });

        self.bg_handles.request_new(bg_id, bind_group_task);
    }

    /// sync the registry and process defferred groups
    pub fn sync<'a>(
        &mut self,
        buffers: &'a ResourceHandler<BufferId, BufferHandle>,
        textures: &'a ResourceHandler<TextureId, TextureHandle>,
        samplers: &'a ResourceHandler<SamplerId, SamplerHandle>,
    ) {
        self.layout_handles.sync();
        self.bg_handles.sync();

        // println!("pending bind groups: {}", self.deffered.len());
        let pending_bgs = std::mem::take(&mut self.deffered);
        for (bg_id, (bgl_id, builder)) in &pending_bgs {
            self.request_bg(bg_id, &bgl_id, builder, buffers, textures, samplers);
        }
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, id: &BindGroupId) {
        self.bg_handles.remove(id);
        self.blueprints.remove(id);
        self.deffered.remove(id);
    }

    /// returns a clone of the handle to a stored bind group
    pub fn get_cloned_bg(&self, id: &BindGroupId) -> Option<BindGroupHandle> {
        return self.bg_handles.get(id).cloned()
    }

    /// returns a clone of the handle to a stored bind group layout
    pub fn get_cloned_layout(&self, id: &LayoutId) -> Option<BindGroupLayoutHandle> {
        return self.layout_handles.get(id).cloned()
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
            if let Some(mut layout) = bind_groups.get_cloned_layout(bg_id) {
                layout.ref_count += 1;
                bg_layouts.push((*layout).clone())
            }
        }
        
        // println!("expected layouts: {}, ready layouts: {}", builder.bg_layouts.len(), bg_layouts.len());

        if builder.bg_layouts.len() != bg_layouts.len() {
            self.deferred.insert(*id, builder.clone());
            return; 
        }

        self.deferred.remove(id);

        let gpu = self.gpu.clone();
        let pip_builder = builder.clone();
        match builder.pip_type {
            PipelineType::Render(ty) => {
                let r_pip_task = Task::non_blocking(async move {
                    gpu.create_render_pipeline(pip_builder, ty, bg_layouts)
                });

                self.handles.request_new(id, r_pip_task);
            },
            PipelineType::Compute(ty) => {
                let c_pip_task = Task::non_blocking(async move {
                    gpu.create_compute_pipeline(pip_builder, ty, bg_layouts)
                });

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

        // println!("{:?}", self.handles.status_of_all());
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