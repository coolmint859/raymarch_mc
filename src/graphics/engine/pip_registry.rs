use std::{cell::RefCell, collections::{HashMap, HashSet}};
use crate::graphics::*;

/// The resolved dependencies of a bind group
pub(crate) struct PipelineDependencies {
    pub bg_layouts: Vec<wgpu::BindGroupLayout>,
}

/// Keeps track of and validates requests for render/compute pipelines
pub(crate) struct PipelineRegistry {
    gpu: GpuHandle,
    handles: ResourceHandler<PipelineId, PipelineHandle>,
    pip_defs: HashMap<PipelineId, Pipeline>,
    deferred: HashMap<PipelineId, Pipeline>,
    valid_pips: RefCell<HashSet<PipelineId>>
}

impl PipelineRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            handles: ResourceHandler::new(),
            pip_defs: HashMap::new(),
            deferred: HashMap::new(),
            valid_pips: RefCell::new(HashSet::new())
        }
    }

    /// request a new pipeline
    pub fn request<'a>(
        &mut self,
        id: &PipelineId,
        pip_def: &Pipeline,
        bgs: &'a BindGroupRegistry,
    ) {
        if self.handles.contains(id) { return; }

        if let Some(deps) = self.resolve_dependencies(pip_def, bgs) {
            self.deferred.remove(id);

            if !self.pip_defs.contains_key(id) {
                self.pip_defs.insert(*id, pip_def.clone());
            }

            for layout_id in &pip_def.bg_layouts {
                bgs.check_inc_bgl(layout_id);
            }

            let gpu = self.gpu.clone();
            let pip_def_copy = pip_def.clone();
            match &pip_def.pip_type {
                PipelineType::Render(ty) => {
                    let pip_type = ty.clone();
                    let r_pip_task = Task::non_blocking(async move {
                        gpu.create_render_pipeline(pip_def_copy, pip_type, deps.bg_layouts)
                    });

                    self.handles.request_new(id, r_pip_task);
                },
                PipelineType::Compute(ty) => {
                    let pip_type = ty.clone();
                    let c_pip_task = Task::non_blocking(async move {
                        gpu.create_compute_pipeline(pip_def_copy, pip_type, deps.bg_layouts)
                    });

                    self.handles.request_new(id, c_pip_task);
                }
            };
        } else {
            self.deferred.insert(*id, pip_def.clone());
        }
    }

    /// sync the registry and process defferred groups
    pub fn sync<'a>(&mut self, bgs: &'a BindGroupRegistry) {
        self.handles.sync();

        let pending_bgs = std::mem::take(&mut self.deferred);
        for (id, pip_def) in &pending_bgs {
            self.request(id, pip_def, bgs);
        }
    }

    /// remove a bind group from the registry
    pub fn remove(&mut self, pip_id: &PipelineId) {
        self.handles.remove(pip_id);
        self.pip_defs.remove(pip_id);
        self.deferred.remove(pip_id);
        self.invalidate(pip_id);
    }

    pub fn get_blueprint(&self, id: &PipelineId) -> Option<&Pipeline> {
        self.pip_defs.get(id)
    }

    /// Validates and returns a pipeline handle corresponding to the id if it passes
    pub fn validate<'a>(&self, pip_id: &PipelineId, bgs: &'a BindGroupRegistry) -> Option<&PipelineHandle> {
        if self.valid_pips.borrow().contains(pip_id) {
            return self.handles.get(pip_id);
        }

        let pip_def = self.pip_defs.get(pip_id)?;
        self.resolve_dependencies(pip_def, bgs)?;

        self.valid_pips.borrow_mut().insert(*pip_id);
        self.handles.get(pip_id)
    }

    /// Invalidate a pipeline. This prevents it from being used in subsequent gpu commands
    pub fn invalidate(&self, pip_id: &PipelineId) {
        self.valid_pips.borrow_mut().remove(pip_id);
    }

    fn resolve_dependencies<'a>(
        &self,
        pip_def: &Pipeline, 
        bgs: &'a BindGroupRegistry
    ) -> Option<PipelineDependencies> {
        let mut bg_layouts = Vec::new();
        for bgl_id in &pip_def.bg_layouts {
            if let Some(layout) =  bgs.layout_handles.get(bgl_id).cloned() {
                bg_layouts.push((*layout).clone())
            } else {
                return None
            }
        }

        return Some(PipelineDependencies { bg_layouts })
    }
}