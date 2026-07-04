use std::{ops::Deref, sync::Arc};

use crate::graphics::BindGroupId;

#[derive(Clone, Debug)]
/// builders for gpu pipelines
pub enum PipelineBuilder {
    Render(RenderPipelineBuilder),
    Compute(ComputePipelineBuilder)
}

impl PipelineBuilder {
    /// Attempts to convert the Pipeline Builder variant into a render builder type.
    pub fn as_render(&self) -> Option<&RenderPipelineBuilder> {
        match self {
            PipelineBuilder::Render(builder) => Some(builder),
            _ => None
        }
    }

    /// Attempts to convert the Pipeline Builder variant into a compute builder type.
    pub fn as_compute(&self) -> Option<&ComputePipelineBuilder> {
        match self {
            PipelineBuilder::Compute(builder) => Some(builder),
            _ => None
        }
    }

    /// Get a reference to the bind group ids this pipeline builder references
    pub fn bind_groups(&self) -> &Vec<BindGroupId> {
        match self {
            PipelineBuilder::Compute(builder) => &builder.bg_layouts,
            PipelineBuilder::Render(builder) => &builder.bg_layouts
        }
    }
}

/// A lightweight handle to a rendering pipeline
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderPipelineHandle {
    pub pipeline: Arc<wgpu::RenderPipeline>
}

impl Deref for RenderPipelineHandle {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

#[derive(Clone, Debug)]
pub struct RenderPipelineBuilder {
    pub label: String,
    pub bg_layouts: Vec<BindGroupId>,
    pub shader_source: Option<&'static str>,
    pub vs_main: String,
    pub fs_main: String,
    pub target_format: Option<wgpu::TextureFormat>
}

impl RenderPipelineBuilder {
    pub fn new() -> Self {
        Self {
            label: "render_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_source: None,
            vs_main: "vs_main".to_string(),
            fs_main: "fs_main".to_string(),
            target_format: None
        }
    }

    /// Set the label for gpu profiling of the resultant render pipeline
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add bind group layouts to the pipeline
    pub fn with_bg_layouts(mut self, layouts: &[BindGroupId]) -> Self {
        self.bg_layouts.extend_from_slice(layouts);
        self
    }

    /// Add a shader descriptor to the pipeline
    pub fn with_shader(mut self, source: &'static str) -> Self {
        self.shader_source = Some(source);
        self
    }

    /// Set the names to the entry points as defined in the shader
    pub fn with_entry_points(mut self, vs: &str, fs: &str) -> Self {
        self.vs_main = vs.to_string();
        self.fs_main = fs.to_string();
        self
    }

    /// Set the target format of the pipeline
    pub fn with_target_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.target_format = Some(format);
        self
    }
}

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

#[derive(Clone, Debug)]
pub struct ComputePipelineBuilder {
    pub label: String,
    pub bg_layouts: Vec<BindGroupId>,
    pub shader_source: Option<&'static str>,
    pub main: String,
}

impl ComputePipelineBuilder {
    pub fn new() -> Self {
        Self {
            label: "compute_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_source: None,
            main: "cs_main".to_string()
        }
    }

    /// Set the label for gpu profiling of the resultant buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Set the shader program this pipeline will execute with
    pub fn with_shader(mut self, source: &'static str) -> Self {
        self.shader_source = Some(source);
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