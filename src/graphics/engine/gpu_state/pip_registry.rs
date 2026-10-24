use std::{cell::RefCell, collections::{HashMap, HashSet}};
use crate::{graphics::*, utils::{HoldPolicy, ResourceHandler, Task}};

/// The resolved dependencies of a bind group
pub(crate) struct PipelineDependencies {
    pub bg_layouts: Vec<wgpu::BindGroupLayout>,
}

/// Keeps track of and validates requests for render/compute pipelines
pub(crate) struct PipelineRegistry {
    gpu: GpuHandle,
    handles: ResourceHandler<PipelineId, PipelineHandle, String>,
    pip_bps: HashMap<PipelineId, PipelineType>,
    deferred: HashMap<PipelineId, PipelineType>,
    valid_pips: RefCell<HashSet<PipelineId>>
}

impl PipelineRegistry {
    pub fn new(gpu: GpuHandle) -> Self {
        Self {
            gpu,
            handles: ResourceHandler::new(),
            pip_bps: HashMap::new(),
            deferred: HashMap::new(),
            valid_pips: RefCell::new(HashSet::new())
        }
    }

    /// Request a new pipeline
    pub fn request(&mut self, blueprint: PipelineType, bgs: &BindGroupRegistry, global_hp: HoldPolicy) -> PipelineId {
        let id = blueprint.create_id();

        // pipeline ids are content-addressable, so its possible the same blueprint was used
        if self.handles.contains(&id) { return id };

        self.try_request(&id, blueprint, bgs, global_hp);

        id
    }

    /// Sync the registry and process deferred groups
    pub fn sync(&mut self, bgs: &BindGroupRegistry, global_hp: HoldPolicy) {
        self.handles.sync();

        let pending_bgs = std::mem::take(&mut self.deferred);
        for (id, pip_def) in &pending_bgs {
            self.try_request(id, pip_def.clone(), bgs, global_hp);
        }
    }

    /// Remove a bind group from the registry
    pub fn remove(&mut self, pip_id: &PipelineId, bgs: &mut BindGroupRegistry) {
        if let Some(blueprint) = self.pip_bps.remove(pip_id) {
            for layout_id in blueprint.bg_layouts().iter() {
                bgs.check_dec_bgl(layout_id);
            }
        }

        self.handles.remove(pip_id);
        self.deferred.remove(pip_id);
        self.invalidate(pip_id);
    }

    /// Validates and returns a pipeline handle corresponding to the id if it passes
    pub fn validate(&self, pip_id: &PipelineId, bgs: &BindGroupRegistry) -> Option<&PipelineHandle> {
        if self.valid_pips.borrow().contains(pip_id) {
            return self.handles.get(pip_id);
        }

        let blueprint = self.pip_bps.get(pip_id)?;
        self.resolve_dependencies(blueprint, bgs)?;

        self.valid_pips.borrow_mut().insert(*pip_id);
        self.handles.get(pip_id)
    }

    /// Invalidate a pipeline. This prevents it from being used in subsequent gpu commands
    pub fn invalidate(&self, pip_id: &PipelineId) {
        self.valid_pips.borrow_mut().remove(pip_id);
    }

    /// Try requesting a new pipeline tied to the provided id.
    fn try_request(
        &mut self,
        pip_id: &PipelineId,
        blueprint: PipelineType, 
        bgs: &BindGroupRegistry,
        global_hp: HoldPolicy
    ) {
        if let Some(deps) = self.resolve_dependencies(&blueprint, bgs) {
            self.deferred.remove(pip_id);

            if !self.pip_bps.contains_key(pip_id) {
                self.pip_bps.insert(*pip_id, blueprint.clone());
            }

            for layout_id in blueprint.bg_layouts().iter() {
                bgs.check_inc_bgl(layout_id);
            }

            let gpu = self.gpu.clone();
            match blueprint {
                PipelineType::Render(pip_bp) => {
                    let policy = pip_bp.hold_policy.unwrap_or(global_hp);

                    let r_pip_task = Task::io_bound(async move {
                        gpu.create_render_pipeline(pip_bp, deps.bg_layouts)
                    }).with_hold_policy(policy);

                    self.handles.request(*pip_id, r_pip_task);
                },
                PipelineType::Compute(pip_bp) => {
                    let policy = pip_bp.hold_policy.unwrap_or(global_hp);

                    let c_pip_task = Task::io_bound(async move {
                        gpu.create_compute_pipeline(pip_bp, deps.bg_layouts)
                    }).with_hold_policy(policy);

                    self.handles.request(*pip_id, c_pip_task);
                }
            };
        } else {
            self.deferred.insert(*pip_id, blueprint);
        }
    }

    /// Resolves the dependencies of a pipeline given it's `blueprint` and a `BindGroupRegistry`
    fn resolve_dependencies(&self, blueprint: &PipelineType, bgs: &BindGroupRegistry) -> Option<PipelineDependencies> {
        let mut bg_layouts = Vec::new();
        for bgl_id in blueprint.bg_layouts().iter() {
            if let Some(layout) =  bgs.layout_handles.get(bgl_id).cloned() {
                bg_layouts.push((*layout).clone())
            } else {
                return None
            }
        }

        return Some(PipelineDependencies { bg_layouts })
    }
}