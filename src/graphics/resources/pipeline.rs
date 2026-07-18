use std::{borrow::Cow, sync::Arc};

use crate::graphics::{BindGroupId, GpuHandle};

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


/// Create a new render pipeline from the given configuration builder
pub async fn create_render_pipeline(
    gpu: GpuHandle,
    builder: Pipeline,
    ty: RenderPipelineType,
    bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>
) -> Result<PipelineHandle, String> {
    let shader_path = builder.shader_path
        .as_ref()
        .expect("[Render Pipeline] Expected pipeline to be configured with a path to a shader, but none was found");

    let shader_source = match std::fs::read_to_string(&shader_path) {
        Ok(source) => source,
        Err(e) => {
            return Err(format!("[Render Pipeline] Failed to read shader file '{}': {e}", shader_path));
        }
    };

    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{}_source", builder.label)),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
    });

    let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
        .iter()
        .map(|layout| { layout.as_ref() })
        .collect();

    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{}_layout", builder.label)),
        bind_group_layouts: &bg_layout_refs,
        immediate_size: 0,
    });

    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&builder.label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some(&ty.vs_main),
            compilation_options: Default::default(),
            buffers: &[], // Full-screen procedurally drawn triangle requires no input VBO buffers!
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(&ty.fs_main),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: ty.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    println!("[GpuContext] Created new render pipeline with label '{}'", builder.label);

    Ok(PipelineHandle::Render(pipeline))
}

pub async fn create_compute_pipeline(
    gpu: GpuHandle, 
    builder: Pipeline,
    ty: ComputePipelineType,
    bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>
) -> Result<PipelineHandle, String> {
    let shader_path = builder.shader_path
        .as_ref()
        .expect("[Compute Pipeline] Expected pipeline to be configured with a path to a shader, but none was found");

    let shader_source = match std::fs::read_to_string(&shader_path) {
        Ok(source) => source,
        Err(e) => {
            return Err(format!("[Compute Pipeline] Failed to read shader file '{}': {e}", shader_path));
        }
    };

    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{}_source", builder.label)),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
    });

    let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
        .iter()
        .map(|layout| { layout.as_ref() })
        .collect();

    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{} Layout", builder.label)),
        bind_group_layouts: &bg_layout_refs,
        immediate_size: 0,
    });

    let pipeline = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&builder.label),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some(&ty.main),
        compilation_options: Default::default(),
        cache: None
    });

    println!("[GpuContext] Created new compute pipeline with label '{}'", builder.label);

    Ok(PipelineHandle::Compute(pipeline))
}