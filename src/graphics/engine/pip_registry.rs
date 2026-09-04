use std::{cell::RefCell, collections::{HashMap, HashSet}};
use crate::graphics::*;

/// Keeps track of and validates requests for render/compute pipelines
pub struct PipelineRegistry {
    gpu: GpuHandle,
    handles: ResourceHandler<PipelineId, PipelineHandle>,
    blueprints: HashMap<PipelineId, Pipeline>,
    deferred: HashMap<PipelineId, Pipeline>,
    valid_pips: RefCell<HashSet<PipelineId>>
}

impl PipelineRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            handles: ResourceHandler::new(),
            blueprints: HashMap::new(),
            deferred: HashMap::new(),
            valid_pips: RefCell::new(HashSet::new())
        }
    }

    /// request a new pipeline
    pub fn request<'a>(
        &mut self,
        id: &PipelineId,
        pip_def: &Pipeline,
        bind_groups: &'a BindGroupRegistry,
    ) {
        if self.handles.contains(id) { return; }

        if !self.blueprints.contains_key(id) {
            self.blueprints.insert(*id, pip_def.clone());
        }

        let mut bg_layouts = Vec::new();
        for bgl_id in &pip_def.bg_layouts {
            if let Some(layout) = bind_groups.get_cloned_layout(bgl_id) {
                bg_layouts.push((*layout).clone())
            }
        }
        
        // println!("expected layouts: {}, ready layouts: {}", pip_def.bg_layouts.len(), bg_layouts.len());

        if pip_def.bg_layouts.len() != bg_layouts.len() {
            self.deferred.insert(*id, pip_def.clone());
            return; 
        }
        self.deferred.remove(id);
        
        for layout_id in &pip_def.bg_layouts {
            bind_groups.check_inc_bgl(layout_id);
        }

        let gpu = self.gpu.clone();
        let pip_def_copy = pip_def.clone();
        match &pip_def.pip_type {
            PipelineType::Render(ty) => {
                let pip_type = ty.clone();
                let r_pip_task = Task::non_blocking(async move {
                    gpu.create_render_pipeline(pip_def_copy, pip_type, bg_layouts)
                });

                self.handles.request_new(id, r_pip_task);
            },
            PipelineType::Compute(ty) => {
                let pip_type = ty.clone();
                let c_pip_task = Task::non_blocking(async move {
                    gpu.create_compute_pipeline(pip_def_copy, pip_type, bg_layouts)
                });

                self.handles.request_new(id, c_pip_task);
            }
        };
    }

    /// sync the registry and process defferred groups
    pub fn sync<'a>(&mut self, bind_groups: &'a BindGroupRegistry) {
        self.handles.sync();

        let pending_bgs = std::mem::take(&mut self.deferred);
        for (id, pip_def) in &pending_bgs {
            self.request(id, pip_def, bind_groups);
        }

        // println!("{:?}", self.handles.status_of_all());
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, pip_id: &PipelineId) {
        self.handles.remove(pip_id);
        self.blueprints.remove(pip_id);
        self.deferred.remove(pip_id);
        self.valid_pips.borrow_mut().remove(pip_id);
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

    /// Validates and returns a pipeline handle corresponding to the id if it passes
    pub fn validate(&self, pip_id: &PipelineId, context: &GpuContext) -> Option<&PipelineHandle> {
        if self.valid_pips.borrow().contains(pip_id) {
            return self.handles.get(pip_id);
        }

        // check if the referenced layouts are valid
        if let Some(pip_def) = self.blueprints.get(pip_id) {
            for layout in &pip_def.bg_layouts {
                if !context.bg_registry.contains_bg_layout(layout) {
                    return None
                }
            }
        }

        self.valid_pips.borrow_mut().insert(*pip_id);
        self.handles.get(pip_id)
    }

    /// Invalidate a pipeline. This prevents it from being used in subsequent gpu commands
    pub fn invalidate(&self, pip_id: &PipelineId) {
        self.valid_pips.borrow_mut().remove(pip_id);
    }
}