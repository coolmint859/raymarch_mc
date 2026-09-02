use crate::graphics::{LayoutId, VertexBufferLayout};

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
#[derive(Clone, Debug)]
pub struct RenderPipelineType {
    pub vs_main: &'static str, 
    pub fs_main: &'static str, 
    pub format: wgpu::TextureFormat,
    pub vertex_layouts: Vec<VertexBufferLayout>,
}

impl RenderPipelineType {
    pub fn new(vs_main: &'static str, fs_main: &'static str) -> Self {
        Self {
            vs_main,
            fs_main,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            vertex_layouts: Vec::new()
        }
    }

    /// Set the output format of the render pipeline. This must match the target format for the corresponding render pass
    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Add a vertex layout to the rendering pipeline. A corresponding buffer must be added to the render pass
    pub fn with_vertex_layout(mut self, layout: &VertexBufferLayout) -> Self {
        self.vertex_layouts.push(layout.clone());
        self
    }
}

impl Default for RenderPipelineType {
    fn default() -> Self {
        RenderPipelineType::new("vs_main", "fs_main")
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
#[derive(Clone, Debug)]
pub enum PipelineType {
    Render(RenderPipelineType),
    Compute(ComputePipelineType)
}

/// Blueprint for render/compute pipelines
#[derive(Clone, Debug)]
pub struct Pipeline {
    pub label: String,
    pub pip_type: PipelineType,
    pub bg_layouts: Vec<LayoutId>,
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
    pub fn with_bg_layouts(mut self, layouts: &[LayoutId]) -> Self {
        self.bg_layouts.extend_from_slice(layouts);
        self
    }

    /// Add a shader descriptor to the pipeline
    pub fn with_shader(mut self, path: &'static str) -> Self {
        self.shader_path = Some(path);
        self
    }
}
