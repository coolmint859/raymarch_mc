use std::{cell::RefCell, collections::HashSet};

use crate::graphics::{BindGroupHandle, BindGroupId, BindingTarget, BufferId, GpuContext, PipelineId, TextureId};

/// Validates the readiness of gpu resources. Stores resources it knows to be ready for fast retrieval
pub struct PassValidator {
    known_bind_groups: RefCell<HashSet<BindGroupId>>,
    known_pipelines: RefCell<HashSet<PipelineId>>
}

impl PassValidator {
    pub fn new() -> Self {
        Self {
            known_bind_groups: RefCell::new(HashSet::new()),
            known_pipelines: RefCell::new(HashSet::new()),
        }
    }

    /// Invalidate a buffer, indicating it was destroyed/removed. This also invalidates any bind groups that reference it.
    pub fn invalidate_buffer(&self, id: &BufferId, context: &GpuContext) {
        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in context.bg_registry.get_blueprints() {
            for entry in &bg_blueprint.bindings {
                if let BindingTarget::Buffer(buf_id) = &entry.target {
                    if buf_id == id {
                        invalid_bgs.insert(*bd_id);
                        continue;
                    }
                }
            }
        }

        for bg_id in invalid_bgs {
            self.invalidate_bind_group(&bg_id, context);
        }
    }

    /// Invalidate a texture, indicating it was destroyed/removed. This also invalidates any bind groups that reference it.
    pub fn invalidate_texture(&self, id: &TextureId, context: &GpuContext) {
        let mut invalid_bgs = HashSet::new();

        for (bd_id, bg_blueprint) in context.bg_registry.get_blueprints() {
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
            self.invalidate_bind_group(bg_id, context);
        }
    }

    /// Invalidate a bind group, indicating it was destroyed/removed. This also invalidates any pipelines that reference it.
    pub fn invalidate_bind_group(&self, bg_id: &BindGroupId, context: &GpuContext) {
        self.known_bind_groups.borrow_mut().remove(bg_id);

        let mut invalid_pipelines = HashSet::new();
        
        for (pip_id, pip_blueprint) in context.pip_registry.get_blueprints() {
            for other_bg_id in &pip_blueprint.bg_layouts {
                if other_bg_id == bg_id {
                    invalid_pipelines.insert(*pip_id);
                    continue;
                }
            }
        }

        for pip_id in invalid_pipelines {
            self.known_pipelines.borrow_mut().remove(&pip_id);
        }
    }

    /// Invalidate a pipeline, indicating it was destroyed/removed.
    pub fn invalidate_pipeline(&self, pip_id: &PipelineId) {
        self.known_pipelines.borrow_mut().remove(pip_id);
    }

    /// Validates a bind group by ensuring it's entries are ready.
    /// 
    /// Returns an option containing the handle to the bind group if ready, else None
    pub fn validate_bind_group<'a>(
        &self, 
        bg_id: &BindGroupId, 
        context: &'a GpuContext
    ) -> Option<BindGroupHandle> {
        if self.known_bind_groups.borrow_mut().contains(bg_id) {
            return context.bg_registry.get_cloned(bg_id);
        }

        let bg_blueprint = context.bg_registry.get_blueprint(bg_id)?;

        for entry in &bg_blueprint.bindings {
            match &entry.target {
                BindingTarget::Buffer(buf_id) => {
                    if !context.buffers.contains(buf_id) {
                        println!("[GpuValidator] Validation failed for bind group @{:?}: Missing Buffer @{:?}", bg_id, buf_id);
                        return None; 
                    }
                },
                BindingTarget::Texture(tex_id) => {
                    if !context.textures.contains(tex_id) { 
                        println!("[GpuValidator] Validation failed for bind group @{:?}: Missing Texture @{:?}", bg_id, tex_id);
                        return None; 
                    }
                }
            }
        }

        self.known_bind_groups.borrow_mut().insert(*bg_id);
        context.bg_registry.get_cloned(bg_id)
    }

    /// Validates a render pipeline given it's id by validating it's bind groups are ready.
    /// 
    /// Returns an option containing the handle to the render pipeline if ready, else None
    pub fn validate_render_pipeline<'a>(
        &self,
        pip_id: &PipelineId,
        context: &'a GpuContext
    ) -> Option<wgpu::RenderPipeline> {
        if self.known_pipelines.borrow_mut().contains(pip_id) {
            return context.pip_registry.get_render_handle(pip_id);
        }

        let pip_blueprint = context.pip_registry.get_blueprint(&pip_id)?;

        for bg_id in &pip_blueprint.bg_layouts {
            if self.validate_bind_group(bg_id, context).is_none() {
                // println!("[GpuValidator] Validation failed for render pipeline @{:?}: Missing Bind Group @{:?}", pip_id, bg_id);
                return None;
            }
        }

        self.known_pipelines.borrow_mut().insert(*pip_id);
        context.pip_registry.get_render_handle(pip_id)
    }

    /// Validates a compute pipeline given it's id by validating it's bind groups are ready.
    /// 
    /// Returns an option containing the handle to the compute pipeline if ready, else None
    pub fn validate_compute_pipeline<'a>(
        &self,
        pip_id: &PipelineId,
        context: &'a GpuContext
    ) -> Option<wgpu::ComputePipeline> {
        if self.known_pipelines.borrow_mut().contains(pip_id) {
            return context.pip_registry.get_compute_handle(pip_id);
        }

        let pip_blueprint = context.pip_registry.get_blueprint(&pip_id)?;

        for bg_id in &pip_blueprint.bg_layouts {
            if self.validate_bind_group(bg_id, context).is_none() {
                // println!("[GpuValidator] Validation failed for compute pipeline @{:?}: Missing Bind Group @{:?}", pip_id, bg_id);
                return None; 
            }
        }

        self.known_pipelines.borrow_mut().insert(*pip_id);
        context.pip_registry.get_compute_handle(pip_id)
    }
}