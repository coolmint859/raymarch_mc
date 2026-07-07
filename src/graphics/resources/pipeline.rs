use std::{borrow::Cow, ops::Deref, sync::Arc};

use crate::graphics::{BindGroupId, GpuHandle};

/// Represents a handle to a render/compute pipeline
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipelineHandle {
    Render(RenderPipelineHandle),
    Compute(ComputePipelineHandle)
} 

impl PipelineHandle {
    /// Get the render pipeline handle if this handle is the Render variant
    pub fn as_render(&self) -> Option<RenderPipelineHandle> {
        match self {
            PipelineHandle::Render(handle) => Some(handle.clone()),
            PipelineHandle::Compute(_) => None
        }
    }

    /// Get the compute pipeline handle if this handle is the Compute variant
    pub fn as_compute(&self) -> Option<ComputePipelineHandle> {
        match self {
            PipelineHandle::Compute(handle) => Some(handle.clone()),
            PipelineHandle::Render(_) => None
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
pub struct PipelineBuilder {
    pub label: String,
    pub pip_type: PipelineType,
    pub bg_layouts: Vec<BindGroupId>,
    pub shader_source: Option<&'static str>,
}

impl PipelineBuilder {
    pub fn new(ty: PipelineType) -> Self {
        Self {
            label: "pipeline".to_string(),
            pip_type: ty,
            bg_layouts: Vec::new(),
            shader_source: None,
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
}


/// Create a new render pipeline from the given configuration builder
pub async fn create_render_pipeline(
    gpu: GpuHandle,
    builder: PipelineBuilder,
    ty: RenderPipelineType,
    bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>
) -> Result<PipelineHandle, String> {
    let shader_source = builder.shader_source
        .as_ref()
        .expect("[Render Pipeline] Expected pipeline to be configured with a shader descriptor, but none was found");

    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{}_source", builder.label)),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source))
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

    Ok(PipelineHandle::Render(
        RenderPipelineHandle {
            pipeline: Arc::new(pipeline),
        }
    ))
}

pub async fn create_compute_pipeline(
    gpu: GpuHandle, 
    builder: PipelineBuilder,
    ty: ComputePipelineType,
    bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>
) -> Result<PipelineHandle, String> {
    let shader_source = builder.shader_source
        .as_ref()
        .expect("[Compute Pipeline] Expected pipeline to be configured with a shader descriptor, but none was found");

    let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
        .iter()
        .map(|layout| { layout.as_ref() })
        .collect();

    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{} Layout", builder.label)),
        bind_group_layouts: &bg_layout_refs,
        immediate_size: 0,
    });

    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{}_source", builder.label)),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source))
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

    Ok(PipelineHandle::Compute(
        ComputePipelineHandle { 
            pipeline: Arc::new(pipeline) 
        }
    ))
}