use crate::graphics::BindGroupId;

/// Represents a handle to a render/compute pipeline
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipelineHandle {
    Render(wgpu::RenderPipeline),
    Compute(wgpu::ComputePipeline)
} 

impl PipelineHandle {
    /// Get the render pipeline handle if this handle is the Render variant
    pub fn as_render(&self) -> Option<wgpu::RenderPipeline> {
        match self {
            PipelineHandle::Render(handle) => Some(handle.clone()),
            PipelineHandle::Compute(_) => None
        }
    }

    /// Get the compute pipeline handle if this handle is the Compute variant
    pub fn as_compute(&self) -> Option<wgpu::ComputePipeline> {
        match self {
            PipelineHandle::Compute(handle) => Some(handle.clone()),
            PipelineHandle::Render(_) => None
        }
    }
}

/// A render pipeline
#[derive(Clone, Copy, Debug)]
pub struct RenderPipelineType {
    pub vs_main: &'static str, 
    pub fs_main: &'static str, 
    pub format: wgpu::TextureFormat
}

impl Default for RenderPipelineType {
    fn default() -> Self {
        Self {
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: wgpu::TextureFormat::Bgra8UnormSrgb
        }
    }
}

/// A compute pipeline
#[derive(Clone, Copy, Debug)]
pub struct ComputePipelineType {
    pub main: &'static str
}

impl Default for ComputePipelineType {
    fn default() -> Self {
        Self { main: "cs_main"}
    }
}

/// The type of gpu pipeline
#[derive(Clone, Copy, Debug)]
pub enum PipelineType {
    Render(RenderPipelineType),
    Compute(ComputePipelineType)
}

/// Blueprint for render/compute pipelines
#[derive(Clone, Debug)]
pub struct Pipeline {
    pub label: String,
    pub pip_type: PipelineType,
    pub bg_layouts: Vec<BindGroupId>,
    pub shader_path: Option<&'static str>,
}

impl Pipeline {
    pub fn new(ty: PipelineType) -> Self {
        Self {
            label: "pipeline".to_string(),
            pip_type: ty,
            bg_layouts: Vec::new(),
            shader_path: None,
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
    pub fn with_shader(mut self, path: &'static str) -> Self {
        self.shader_path = Some(path);
        self
    }
}
