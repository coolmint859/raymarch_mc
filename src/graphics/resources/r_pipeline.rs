use std::{ops::Deref, sync::Arc};

use crate::graphics::{GpuHandle, ResourceBuilder};

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

pub struct RenderPipelineBuilder {
    label: String,
    bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>,
    shader_module: Option<wgpu::ShaderModule>,
    vs_main: String,
    fs_main: String,
    target_format: Option<wgpu::TextureFormat>
}

impl RenderPipelineBuilder {
    pub fn new() -> Self {
        Self {
            label: "render_pipeline".to_string(),
            bg_layouts: Vec::new(),
            shader_module: None,
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
    pub fn with_bg_layouts(mut self, layouts: &[Arc<wgpu::BindGroupLayout>]) -> Self {
        self.bg_layouts.extend_from_slice(layouts);
        self
    }

    /// Set the shader program this pipeline will execute with
    pub fn with_shader(mut self, module: wgpu::ShaderModule) -> Self {
        self.shader_module = Some(module);
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

impl ResourceBuilder for RenderPipelineBuilder {
    type Resource = RenderPipelineHandle;

    fn build(&self, gpu: GpuHandle) -> Self::Resource {
        let shader = self.shader_module.as_ref()
            .expect("[Render Pipeline] Expected pipeline to be configured with a shader module, but none was found.");
        let format = self.target_format
            .expect("[Render Pipeline] Expected pipeline to be configured with a target format, but none was found.");

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = self.bg_layouts
            .iter()
            .map(|arc| arc.as_ref()) // or just &**arc
            .collect();

        let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", self.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&self.label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(&self.vs_main),
                compilation_options: Default::default(),
                buffers: &[], // Full-screen procedurally drawn triangle requires no input VBO buffers!
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(&self.fs_main),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        RenderPipelineHandle {
            pipeline: Arc::new(pipeline),
        }
    }
}