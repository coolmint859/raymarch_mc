use std::{ops::Deref, sync::Arc};

use crate::graphics::BindGroupId;

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

pub struct RenderPipelineBuilder<'a> {
    pub label: String,
    pub bg_layouts: Vec<BindGroupId>,
    pub shader_desc: Option<wgpu::ShaderModuleDescriptor<'a>>,
    pub vs_main: String,
    pub fs_main: String,
    pub target_format: Option<wgpu::TextureFormat>
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: "render_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_desc: None,
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
    pub fn with_shader(mut self, desc: wgpu::ShaderModuleDescriptor<'a>) -> Self {
        self.shader_desc = Some(desc);
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