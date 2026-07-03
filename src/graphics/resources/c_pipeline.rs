use std::{ops::Deref, sync::Arc};

use crate::graphics::BindGroupId;

/// A lightweight handle to a compute pipeline
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputePipelineHandle {
    pub pipeline: Arc<wgpu::ComputePipeline>
}

impl Deref for ComputePipelineHandle {
    type Target = wgpu::ComputePipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

pub struct ComputePipelineBuilder {
    pub label: String,
    pub bg_layouts: Vec<BindGroupId>,
    pub shader_module: Option<wgpu::ShaderModule>,
    pub main: String,
}

impl ComputePipelineBuilder {
    pub fn new() -> Self {
        Self {
            label: "compute_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_module: None,
            main: "cs_main".to_string()
        }
    }

    /// Set the label for gpu profiling of the resultant buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Set the shader program this pipeline will execute with
    pub fn with_shader(mut self, module: wgpu::ShaderModule) -> Self {
        self.shader_module = Some(module);
        self
    }

    /// Add bind group layouts to the pipeline
    pub fn with_bg_layouts(mut self, layouts: &[BindGroupId]) -> Self {
        self.bg_layouts.extend_from_slice(layouts);
        self
    }

    /// Set the name to the entry point as defined in the shader
    pub fn with_entry_point(mut self, cs: &str) -> Self {
        self.main = cs.to_string();
        self
    }
}