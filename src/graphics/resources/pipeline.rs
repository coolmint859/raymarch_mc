use crate::graphics::{LayoutId, VertexBufferLayout};

/// Represents a handle to a render/compute pipeline
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipelineHandle {
    Render(wgpu::RenderPipeline),
    Compute(wgpu::ComputePipeline)
} 

impl PipelineHandle {
    /// Get the render pipeline handle if this handle is the Render variant
    pub fn to_render(&self) -> Option<wgpu::RenderPipeline> {
        match self {
            PipelineHandle::Render(handle) => Some(handle.clone()),
            PipelineHandle::Compute(_) => None
        }
    }

    /// Get the compute pipeline handle if this handle is the Compute variant
    pub fn to_compute(&self) -> Option<wgpu::ComputePipeline> {
        match self {
            PipelineHandle::Compute(handle) => Some(handle.clone()),
            PipelineHandle::Render(_) => None
        }
    }
}

/// Holds configuration state for a render pipeline blueprint
#[derive(Clone, Debug)]
pub struct RenderType {
    pub vs_main: &'static str, 
    pub fs_main: &'static str, 
    pub format: wgpu::TextureFormat,
    pub vertex_layouts: Vec<VertexBufferLayout>,
}

impl RenderType {
    pub fn new(vs_main: &'static str, fs_main: &'static str) -> Self {
        Self {
            vs_main,
            fs_main,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            vertex_layouts: Vec::new()
        }
    }
}

/// Holds configurations state for a compute pipeline creation
#[derive(Clone, Copy, Debug)]
pub struct ComputeType {
    pub main: &'static str
}

impl ComputeType {
    pub fn new(main: &'static str) -> Self {
        Self { main }
    }
}

/// The type of gpu pipeline. This is used internally by the context to create the underlying wgpu pipelines
#[derive(Clone, Debug)]
pub enum PipelineType {
    Render(Pipeline<RenderType>),
    Compute(Pipeline<ComputeType>)
}

impl PipelineType {
    /// Get the bind group layouts associated with the pipeline
    pub fn bg_layouts(&self) -> &Vec<LayoutId> {
        return match self {
            PipelineType::Render(pip) => {
                &pip.bg_layouts
            }
            PipelineType::Compute(pip) => {
                &pip.bg_layouts
            }
        }
    }
}

impl From<Pipeline<RenderType>> for PipelineType {
    fn from(pip: Pipeline<RenderType>) -> Self {
        Self::Render(pip)
    }
}

impl From<Pipeline<ComputeType>> for PipelineType {
    fn from(pip: Pipeline<ComputeType>) -> Self {
        Self::Compute(pip)
    }
}

/// A blueprint for constructing render and compute pipelines
#[derive(Clone, Debug)]
pub struct Pipeline<T> {
    pub label: String,
    pub bg_layouts: Vec<LayoutId>,
    pub shader_path: Option<&'static str>,
    pub ty: T,
}

impl<T> Pipeline<T> {
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

impl Pipeline<RenderType> {
    /// Create a new render pipeline.
    pub fn as_render() -> Self {
        Self {
            label: "render_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_path: None,
            ty: RenderType::new("vs_main", "fs_main")
        }
    }

    /// Set the entry points of the render shader for the render pipeline
    pub fn with_entry_points(mut self, vs_main: &'static str, fs_main: &'static str) -> Self {
        self.ty.vs_main = vs_main;
        self.ty.fs_main = fs_main;
        self
    }

    /// Set the output format of the render pipeline. This must match the target format for the corresponding render pass
    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.ty.format = format;
        self
    }

    /// Add a vertex layout to the rendering pipeline. A corresponding buffer must be added to the render pass
    pub fn with_vertex_layout(mut self, layout: VertexBufferLayout) -> Self {
        self.ty.vertex_layouts.push(layout);
        self
    }
}

impl Pipeline<ComputeType> {
    /// Create a new compute pipeline
    pub fn as_compute() -> Self {
        Self {
            label: "compute_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_path: None,
            ty: ComputeType::new("cs_main")
        }
    }

    /// Set the entry point of the compute shader for the compute pipeline
    pub fn with_entry_point(mut self, main: &'static str) -> Self {
        self.ty.main = main;
        self
    }
}
