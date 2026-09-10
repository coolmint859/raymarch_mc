use std::borrow::Cow;

use wgpu::util::DeviceExt;

use crate::graphics::{BindGroup, BindGroupLayoutHandle, Buffer, BufferContents, ComputeType, Pipeline, PipelineHandle, RenderType, Sampler, TextureHandle, TextureType};

/// Handle to the gpu device and queue
#[derive(Clone, Debug)]
pub struct GpuHandle {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue
}

impl GpuHandle {
    /// Create a buffer from the given configuration builder
    pub fn create_buffer(&self, buffer_def: Buffer) -> Result<wgpu::Buffer, String> {
        let buffer = match &buffer_def.contents {
            BufferContents::Empty(size) => {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&buffer_def.label),
                    size: *size,
                    usage: buffer_def.usage,
                    mapped_at_creation: false
                })
            },
            BufferContents::WithData(data) => {
                self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&buffer_def.label),
                    contents: &data,
                    usage: buffer_def.usage
                })
            }
        };

        println!("[GpuContext] Created new buffer with label '{}'", buffer_def.label);

        Ok(buffer)
    }

    /// Create a new texture from the given configuration builder
    pub fn create_texture(&self, texture_def: TextureType) -> Result<TextureHandle, String> {
        let tex_payload = texture_def.into_payload()?;

        let extent = wgpu::Extent3d {
            width: tex_payload.dim.width,
            height: tex_payload.dim.height,
            depth_or_array_layers: tex_payload.dim.depth
        };
        
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&tex_payload.label),
            size: extent,
            mip_level_count: tex_payload.mip_levels,
            sample_count: 1,
            dimension: tex_payload.dim.wgpu_dim,
            format: tex_payload.format,
            usage: tex_payload.usage,
            view_formats: &[],
        });

        if let Some(data) = &tex_payload.data {
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
                    bytes_per_row: Some(tex_payload.bytes_per_pixel() * tex_payload.dim.width),
                    rows_per_image: Some(tex_payload.dim.height)
                },
                extent,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        println!("[GpuContext] Created new texture with label '{}'", tex_payload.label);

        Ok(TextureHandle { texture, view, extent })
    }

    /// Create a new sampler from the given configuration builder
    pub fn create_sampler(&self, sampler_def: Sampler) -> Result<wgpu::Sampler, String> {
        let sampler = self.device.create_sampler(&sampler_def.desc);

        Ok(sampler)
    }

    pub fn create_bg_layout(&self, bg_layout_def: BindGroup ) -> Result<wgpu::BindGroupLayout, String> {
        let layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some(&format!("Layout: {}", bg_layout_def.label)),
            entries: &bg_layout_def.layout_entries
        });

        println!("[GpuContext] Created new bind group layout with label '{}'", bg_layout_def.label);

        Ok(layout)
    }

    /// Create a new bind group from the given configuration builder and resource map
    pub fn create_bind_group(
        &self,
        bg_def: BindGroup, 
        entries: Vec<wgpu::BindGroupEntry<'_>>,
        layout: BindGroupLayoutHandle,
    ) -> Result<wgpu::BindGroup, String> {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&bg_def.label),
            layout: &layout,
            entries: &entries,
        });

        println!("[GpuContext] Created new bind group with label '{}'", bg_def.label);

        Ok(bind_group)
    }

    /// Create a new render pipeline from the given configuration builder
    pub fn create_render_pipeline(
        &self,
        pip_def: Pipeline<RenderType>,
        bg_layouts: Vec<wgpu::BindGroupLayout>
    ) -> Result<PipelineHandle, String> {
        let shader_path = pip_def.shader_path
            .as_ref()
            .expect("[Render Pipeline] Expected pipeline to be configured with a path to a shader, but none was found");

        let shader_source = match std::fs::read_to_string(&shader_path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("[Render Pipeline] Failed to read shader file '{}': {e}", shader_path));
            }
        };

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{}_source", pip_def.label)),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
        });

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
            .iter()
            .map(|layout| layout)
            .collect();

        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", pip_def.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let vertex_layouts: Vec<_> = pip_def.ty.vertex_layouts
            .iter()
            .map(|l| l.as_wgpu_layout())
            .collect();

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&pip_def.label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(&pip_def.ty.vs_main),
                compilation_options: Default::default(),
                buffers: &vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(&pip_def.ty.fs_main),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: pip_def.ty.format,
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

        println!("[GpuContext] Created new render pipeline with label '{}'", pip_def.label);

        Ok(PipelineHandle::Render(pipeline))
    }

    pub fn create_compute_pipeline(
        &self, 
        pip_def: Pipeline<ComputeType>,
        bg_layouts: Vec<wgpu::BindGroupLayout>
    ) -> Result<PipelineHandle, String> {
        let shader_path = pip_def.shader_path
            .as_ref()
            .expect("[Compute Pipeline] Expected pipeline to be configured with a path to a shader, but none was found");

        let shader_source = match std::fs::read_to_string(&shader_path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("[Compute Pipeline] Failed to read shader file '{}': {e}", shader_path));
            }
        };

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{}_source", pip_def.label)),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
        });

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
            .iter()
            .map(|layout| layout)
            .collect();

        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} Layout", pip_def.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&pip_def.label),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some(&pip_def.ty.main),
            compilation_options: Default::default(),
            cache: None
        });

        println!("[GpuContext] Created new compute pipeline with label '{}'", pip_def.label);

        Ok(PipelineHandle::Compute(pipeline))
    }
}