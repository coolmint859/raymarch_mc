use std::{cell::{Cell, RefCell}, collections::{HashMap, HashSet}, ops::Deref, println};
use crate::graphics::*;

/// A lightweight handle to a bind group and associated layout
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindGroupHandle {
    pub bind_group: wgpu::BindGroup,
    pub layout_id: LayoutId
}

impl Deref for BindGroupHandle {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

/// A lightweight handle to a bind group layout
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindGroupLayoutHandle {
    pub ref_count: Cell<usize>,
    pub layout: wgpu::BindGroupLayout,
}

impl Deref for BindGroupLayoutHandle {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

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
    deferred: HashMap<BindGroupId, (LayoutId, BindGroup)>,
    /// the set of bind groups that are known to be valid. 
    valid_bgs: RefCell<HashSet<BindGroupId>>
}

impl BindGroupRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            bg_handles: ResourceHandler::new(),
            layout_handles: ResourceHandler::new(),
            blueprints: HashMap::new(),
            deferred: HashMap::new(),
            valid_bgs: RefCell::new(HashSet::new())
        }
    }

    /// Request a new bind group layout
    pub fn request_layout(&mut self, id: &LayoutId, bg_def: &BindGroup) {
        if self.layout_handles.contains(id) { return; };

        let gpu = self.gpu.clone();
        let builder = bg_def.clone();

        let layout_task = Task::non_blocking(async move {
            gpu.create_bg_layout(builder)
                .and_then(|layout| Ok(BindGroupLayoutHandle { layout, ref_count: Cell::new(1)}))
        });

        self.layout_handles.request_new(id, layout_task);
    }

    /// Request a new bind group
    pub fn request_bg<'a>(
        &mut self,
        bg_id: &BindGroupId,
        layout_id: &LayoutId,
        bg_def: &BindGroup,
        buffers: &'a ResourceHandler<BufferId, wgpu::Buffer>,
        textures: &'a ResourceHandler<TextureId, TextureHandle>,
        samplers: &'a ResourceHandler<SamplerId, wgpu::Sampler>,
    ) {
        if self.bg_handles.contains(bg_id) { return; }

        let layout = match self.layout_handles.get(layout_id) {
            Some(bg_layout) => bg_layout.clone(),
            None => {
                self.deferred.insert(*bg_id, (*layout_id, bg_def.clone()));
                self.request_layout(layout_id, bg_def);
                return; 
            }
        };

        if !self.blueprints.contains_key(bg_id) {
            self.blueprints.insert(*bg_id, bg_def.clone());
        }

        let mut buffer_handles = Vec::new();
        let mut texture_handles = Vec::new();
        let mut sampler_handles = Vec::new();
        
        let mut expected_buf_len = 0usize;
        let mut expected_tex_len = 0usize;
        let mut expected_samp_len = 0usize;

        for binding in &bg_def.bindings {
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
            self.deferred.insert(*bg_id, (*layout_id, bg_def.clone()));
            return; 
        };

        self.deferred.remove(bg_id);
        self.check_inc_bgl(layout_id);

        let gpu = self.gpu.clone();
        let builder = bg_def.clone();
        let layout_id_copy = *layout_id;

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
                .and_then(|bg| Ok(BindGroupHandle { bind_group: bg, layout_id: layout_id_copy}))
        });

        self.bg_handles.request_new(bg_id, bind_group_task);
    }

    /// sync the registry and process deferred groups
    pub fn sync<'a>(
        &mut self,
        buffers: &'a ResourceHandler<BufferId, wgpu::Buffer>,
        textures: &'a ResourceHandler<TextureId, TextureHandle>,
        samplers: &'a ResourceHandler<SamplerId, wgpu::Sampler>,
    ) {
        self.layout_handles.sync();
        self.bg_handles.sync();

        // println!("pending bind groups: {}", self.deffered.len());
        let pending_bgs = std::mem::take(&mut self.deferred);
        for (bg_id, (bgl_id, bg_def)) in &pending_bgs {
            self.request_bg(bg_id, &bgl_id, bg_def, buffers, textures, samplers);
        }
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, bg_id: &BindGroupId) {
        self.bg_handles.remove(bg_id);
        self.blueprints.remove(bg_id);
        self.deferred.remove(bg_id);
        self.valid_bgs.borrow_mut().remove(bg_id);
    }

    /// Validates and returns a bind group corresponding to the id if it passes
    pub fn validate(&self, bg_id: &BindGroupId, context: &GpuContext) -> Option<BindGroupHandle> {
        if self.valid_bgs.borrow().contains(bg_id) {
            return self.bg_handles.get(bg_id).cloned();
        }

        let bg_blueprint = self.blueprints.get(bg_id)?;

        for entry in &bg_blueprint.bindings {
            match &entry.target {
                BindingTarget::Buffer(buf_id) => {
                    if !context.buffers.contains(buf_id) {
                        // println!("[BindGroupRegistry] Validation failed for bind group @{:?}: Missing Buffer @{:?}", bg_id, buf_id);
                        return None; 
                    }
                },
                BindingTarget::Texture(tex_id) => {
                    if !context.textures.contains(tex_id) { 
                        // println!("[BindGroupRegistry] Validation failed for bind group @{:?}: Missing Texture @{:?}", bg_id, tex_id);
                        return None; 
                    }
                }
                BindingTarget::Sampler(samp_id) => {
                    if !context.samplers.contains(samp_id) { 
                        // println!("[BindGroupRegistry] Validation failed for bind group @{:?}: Missing Sampler @{:?}", bg_id, samp_id);
                        return None; 
                    }
                }
            }
        }

        self.valid_bgs.borrow_mut().insert(*bg_id);
        self.bg_handles.get(bg_id).cloned()
    }

    /// Invalidate a bind group. This prevents it from being used in subsequent gpu commands
    pub fn invalidate(&mut self, bg_id: &BindGroupId) {
        self.valid_bgs.borrow_mut().remove(bg_id);
    }
    
    /// returns a clone of the handle to a stored bind group layout
    pub fn get_cloned_layout(&self, id: &LayoutId) -> Option<BindGroupLayoutHandle> {
        return self.layout_handles.get(id).cloned()
    }

    pub fn get_bg_defs(&self) -> &HashMap<BindGroupId, BindGroup> {
        &self.blueprints
    }

    /// Check if the registry contains a bind group layout with the provided id
    pub fn contains_bg_layout(&self, bgl_id: &LayoutId) -> bool {
        return self.layout_handles.contains(bgl_id);
    }

    /// Get the handle to a bind group
    pub fn get_bg(&self, bg_id: &BindGroupId) -> Option<&BindGroupHandle> {
        return self.bg_handles.get(bg_id)
    }

    /// Decrement the reference count of a bind group layout, if exists. 
    /// If the count hits 0, this will remove it from the registry
    pub fn check_dec_bgl(&mut self, bgl_id: &LayoutId) {
        let mut should_remove = false;
        if let Some(layout) = self.layout_handles.get_mut(bgl_id) {
            layout.ref_count.set(layout.ref_count.get() - 1);

            should_remove = layout.ref_count.get() <= 0;

            println!("Subtracted ref count of bind group layout @{:?}; curr count: {}", bgl_id, layout.ref_count.get());
        }

        if should_remove {
            self.layout_handles.remove(bgl_id);
        }
    }

    /// Increment the reference count of a bind group layout, if exists.
    pub fn check_inc_bgl(&self, bgl_id: &LayoutId) {
        if let Some(layout) = self.layout_handles.get(bgl_id) {
            layout.ref_count.set(layout.ref_count.get() + 1);

            println!("Added ref count of bind group layout @{:?}; curr count: {}", bgl_id, layout.ref_count.get());
        }
    }
}