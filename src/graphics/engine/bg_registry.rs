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

/// Contains the dependencies of a bind group. Used in bind group creation and to determine validity.
pub(crate) struct BindGroupDependencies {
    layout: BindGroupLayoutHandle,
    /// the buffers attached to a bind group, including the slot number
    buffers: Vec<(BufferId, wgpu::Buffer, u32)>,
    /// the textures attached to a bind group, including the slot number
    textures: Vec<(TextureId, TextureHandle, u32)>,
    /// the samplers attached to a bind group, including the slot number
    samplers: Vec<(SamplerId, wgpu::Sampler, u32)>,
}

/// Keeps track of and validates requests for bind groups
pub(crate) struct BindGroupRegistry {
    gpu: GpuHandle,
    /// map of ids of bind group that have yet to pass request validation
    deferred: HashMap<BindGroupId, (LayoutId, BindGroup)>,
    /// the set of bind groups that are known to be valid. 
    valid_bgs: RefCell<HashSet<BindGroupId>>,
    /// The handles to bind groups
    pub(crate) bg_handles: ResourceHandler<BindGroupId, BindGroupHandle>,
    /// The handles to bind group layouts
    pub(crate) layout_handles: ResourceHandler<LayoutId, BindGroupLayoutHandle>,
    /// maps bind groups to their blueprints
    pub(crate) bg_defs: HashMap<BindGroupId, BindGroup>,
}

impl BindGroupRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            bg_handles: ResourceHandler::new(),
            layout_handles: ResourceHandler::new(),
            bg_defs: HashMap::new(),
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
        bgl_id: &LayoutId,
        bg_def: &BindGroup,
        resources: &'a GpuResources,
    ) {
        if self.bg_handles.contains(bg_id) { return; }
        
        if let Some(deps) = self.resolve_dependencies(bgl_id, bg_def, resources) {
            self.deferred.remove(bg_id);

            if !self.bg_defs.contains_key(bg_id) {
                self.bg_defs.insert(*bg_id, bg_def.clone());
            }

            self.check_inc_bgl(bgl_id);

            let gpu = self.gpu.clone();
            let builder = bg_def.clone();
            let layout_id_copy = *bgl_id;

            let bind_group_task = Task::non_blocking(async move {
                let mut entries = Vec::new();

                for (_id, buf, slot) in &deps.buffers {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot,
                        resource: buf.as_entire_binding()
                    });
                }
                for (_id, tex, slot) in &deps.textures {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot,
                        resource: wgpu::BindingResource::TextureView(tex)
                    });
                }
                for (_id, samp, slot) in &deps.samplers {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot,
                        resource: wgpu::BindingResource::Sampler(samp)
                    });
                }

                gpu.create_bind_group(builder, entries, deps.layout)
                    .and_then(|bg| Ok(BindGroupHandle { bind_group: bg, layout_id: layout_id_copy}))
            });

            self.bg_handles.request_new(bg_id, bind_group_task);
        } else {
            self.request_layout(bgl_id, bg_def);
            self.deferred.insert(*bg_id, (*bgl_id, bg_def.clone()));
        }
    }

    /// sync the registry and process deferred groups
    pub fn sync<'a>(&mut self, resources: &'a GpuResources) {
        self.layout_handles.sync();
        self.bg_handles.sync();

        // println!("pending bind groups: {}", self.deffered.len());
        let pending_bgs = std::mem::take(&mut self.deferred);
        for (bg_id, (bgl_id, bg_def)) in &pending_bgs {
            self.request_bg(bg_id, &bgl_id, bg_def, resources);
        }
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, bg_id: &BindGroupId) {
        self.bg_handles.remove(bg_id);
        self.bg_defs.remove(bg_id);
        self.deferred.remove(bg_id);
        self.valid_bgs.borrow_mut().remove(bg_id);
    }

    /// Validates and returns a bind group corresponding to the id if it passes
    pub fn validate<'a>(&self, bg_id: &BindGroupId, resources: &'a GpuResources) -> Option<BindGroupHandle> {
        if self.valid_bgs.borrow().contains(bg_id) {
            return self.bg_handles.get(bg_id).cloned();
        }

        if let Some(bgl_id) = self.bg_handles.get(bg_id)
            .and_then(|bg| Some(bg.layout_id)) 
        {
            let bg_def = self.bg_defs.get(bg_id)?.clone();
            self.resolve_dependencies(&bgl_id, &bg_def, &resources)?;
        }

        self.valid_bgs.borrow_mut().insert(*bg_id);
        self.bg_handles.get(bg_id).cloned()
    }

    /// Invalidate a bind group. This prevents it from being used in subsequent gpu commands
    pub fn invalidate(&mut self, bg_id: &BindGroupId) {
        self.valid_bgs.borrow_mut().remove(bg_id);
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

    /// Fetches bind group dependencies if they are ready. If at least one dependency is not yet ready, None is returned
    fn resolve_dependencies<'a>(
        &self, 
        bgl_id: &LayoutId,
        bg_def: &BindGroup,
        resources: &'a GpuResources,
    ) -> Option<BindGroupDependencies> {
        let layout = self.layout_handles.get(bgl_id)?.clone();

        let mut buffer_handles = Vec::new();
        let mut texture_handles = Vec::new();
        let mut sampler_handles = Vec::new();

        for binding in &bg_def.bindings {
            match &binding.target {
                BindingTarget::Buffer(buf_id) => {
                    if let Some(handle) = resources.buffers.get(buf_id) {
                        buffer_handles.push((*buf_id, handle.clone(), binding.slot));
                    } else {
                        return None; // buffer missing, dependency unresolved
                    }
                },
                BindingTarget::Texture(tex_id) => {
                    if let Some(handle) = resources.textures.get(tex_id) {
                        texture_handles.push((*tex_id, handle.clone(), binding.slot));
                    } else {
                        return None; // texture missing, dependency unresolved
                    }
                }
                BindingTarget::Sampler(samp_id) => {
                    if let Some(handle) = resources.samplers.get(samp_id) {
                        sampler_handles.push((*samp_id, handle.clone(), binding.slot));
                    } else {
                        return None; // sampler missing, dependency unresolved
                    }
                }
            }
        }

        return Some(BindGroupDependencies {
            layout,
            buffers: buffer_handles,
            textures: texture_handles,
            samplers: sampler_handles
        })
    }
}