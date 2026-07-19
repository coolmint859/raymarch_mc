use std::{borrow::Cow, sync::Arc};

use wgpu::util::DeviceExt;

use crate::graphics::{BindGroup, BindGroupHandle, Buffer, BufferContents, BufferHandle, ComputePipelineType, Pipeline, PipelineHandle, RenderPipelineType, Texture, TextureHandle};

/// Handle to the gpu device and queue
#[derive(Clone, Debug)]
pub struct GpuHandle {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue
}

impl GpuHandle {
    /// Create a buffer from the given configuration builder
    pub async fn create_buffer(&self, builder: Buffer) -> Result<BufferHandle, String> {
        let buffer = match &builder.contents {
            BufferContents::Empty(size) => {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&builder.label),
                    size: *size,
                    usage: builder.usage,
                    mapped_at_creation: false
                })
            },
            BufferContents::WithData(data) => {
                self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&builder.label),
                    contents: &data,
                    usage: builder.usage
                })
            }
        };

        println!("[GpuContext] Created new buffer with label '{}'", builder.label);

        Ok(BufferHandle { buffer })
    }

    /// Create a new texture from the given configuration builder
    pub async fn create_texture(&self, builder: Texture) -> Result<TextureHandle, String> {
        let tex_info = builder.get_info().await?;
        let extent = wgpu::Extent3d {
            width: tex_info.width,
            height: tex_info.height,
            depth_or_array_layers: tex_info.depth
        };
        
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&builder.label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: tex_info.dim,
            format: builder.format,
            usage: builder.usage,
            view_formats: &[],
        });

        if let Some(data) = &tex_info.data {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                }, 
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(builder.bytes_per_pixel() * tex_info.width),
                    rows_per_image: Some(tex_info.height)
                }, 
                extent,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        println!("[GpuContext] Created new texture with label '{}'", builder.label);

        Ok(TextureHandle { texture, view })
    }

    /// Create a new bind group from the given configuration builder and resource map
    pub async fn create_bind_group(
        &self,
        builder: BindGroup, 
        entries: Vec<wgpu::BindGroupEntry<'_>>
    ) -> Result<BindGroupHandle, String> {
        let layout = Arc::new(self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some(&format!("Layout: {}", builder.label)),
            entries: &builder.layout_entries
        }));

        let bind_group = Arc::new(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&builder.label),
            layout: &layout,
            entries: &entries,
        }));

        println!("[GpuContext] Created new bind group with label '{}'", builder.label);

        Ok(BindGroupHandle { 
            layout, 
            bind_group 
        })
    }

    /// Create a new render pipeline from the given configuration builder
    pub async fn create_render_pipeline(
        &self,
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

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{}_source", builder.label)),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
        });

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
            .iter()
            .map(|layout| { layout.as_ref() })
            .collect();

        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", builder.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        &self, 
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

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{}_source", builder.label)),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
        });

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
            .iter()
            .map(|layout| { layout.as_ref() })
            .collect();

        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} Layout", builder.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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
}