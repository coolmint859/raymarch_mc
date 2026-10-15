use std::{borrow::Cow, println};

use crate::graphics::{BindGroup, BindGroupLayoutHandle, Buffer, Compute, Pipeline, PipelineHandle, Render, SamplerType, Texture, TextureHandle, TextureSource};

/// Handle to the gpu device and queue
#[derive(Clone, Debug)]
pub struct GpuHandle {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue
}

impl GpuHandle {
    /// Create a buffer from the given configuration builder
    pub fn create_buffer(&self, blueprint: Buffer, init_data: Option<Vec<u8>>, capacity: u64) -> Result<wgpu::Buffer, String> {
        let payload = blueprint.into_payload(init_data, capacity);

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&payload.label),
            size: payload.capacity,
            usage: payload.usage,
            mapped_at_creation: false
        });

        if let Some(data) = payload.init_data {
            self.queue.write_buffer(&buffer, 0, &data);
        };

        println!("[GpuContext] Created new buffer with label '{}'", payload.label);

        Ok(buffer)
    }

    /// Create a new texture from the given configuration builder
    pub fn create_texture(&self, blueprint: Texture, source: impl TextureSource) -> Result<TextureHandle, String> {
        let (data, dim) = source.into_data(blueprint.format)?;
        let tex_payload = blueprint.into_payload(data, dim);

        let extent = wgpu::Extent3d {
            width: tex_payload.size.width,
            height: tex_payload.size.height,
            depth_or_array_layers: tex_payload.size.depth
        };
        
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&tex_payload.label),
            size: extent,
            mip_level_count: tex_payload.mip_levels,
            sample_count: 1,
            dimension: tex_payload.size.wgpu_dim,
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
                    bytes_per_row: Some(tex_payload.bytes_per_pixel() * tex_payload.size.width),
                    rows_per_image: Some(tex_payload.size.height)
                },
                extent,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        println!("[GpuContext] Created new texture with label '{}'", tex_payload.label);

        Ok(TextureHandle { texture, view, extent })
    }

    /// Create a new sampler from the given configuration builder
    pub fn create_sampler(&self, blueprint: impl SamplerType) -> Result<wgpu::Sampler, String> {
        let payload = blueprint.into_payload()?;
        let sampler = self.device.create_sampler(&payload.desc);

        Ok(sampler)
    }

    pub fn create_bg_layout(&self, blueprint: BindGroup) -> Result<wgpu::BindGroupLayout, String> {
        let layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some(&format!("Layout: {}", blueprint.label)),
            entries: &blueprint.layout.entries
        });

        println!("[GpuContext] Created new bind group layout with label '{}'", blueprint.label);

        Ok(layout)
    }

    /// Create a new bind group from the given configuration builder and resource map
    pub fn create_bind_group(
        &self,
        blueprint: BindGroup, 
        entries: Vec<wgpu::BindGroupEntry<'_>>,
        layout: BindGroupLayoutHandle,
    ) -> Result<wgpu::BindGroup, String> {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&blueprint.label),
            layout: &layout,
            entries: &entries,
        });

        println!("[GpuContext] Created new bind group with label '{}'", blueprint.label);

        Ok(bind_group)
    }

    /// Create a new render pipeline from the given configuration builder
    pub fn create_render_pipeline(
        &self,
        blueprint: Pipeline<Render>,
        bg_layouts: Vec<wgpu::BindGroupLayout>
    ) -> Result<PipelineHandle, String> {
        let shader_path = blueprint.shader_path
            .as_ref()
            .expect("[Render Pipeline] Expected pipeline to be configured with a path to a shader, but none was found");

        let shader_source = match std::fs::read_to_string(&shader_path) {
            Ok(source) => source,
            Err(e) => {
                println!("Failed to read shader file: {e}");
                return Err(format!("[Render Pipeline] Failed to read shader file '{}': {e}", shader_path));
            }
        };

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&blueprint.label),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
        });

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
            .iter()
            .map(|layout| layout)
            .collect();

        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", blueprint.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let vertex_layouts: Vec<_> = blueprint.ty.vertex_layouts
            .iter()
            .map(|l| l.as_wgpu_layout())
            .collect();

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&blueprint.label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(&blueprint.ty.vs_main),
                compilation_options: Default::default(),
                buffers: &vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(&blueprint.ty.fs_main),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: blueprint.ty.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        println!("[GpuContext] Created new render pipeline with label '{}'", blueprint.label);

        Ok(PipelineHandle::Render(pipeline))
    }

    pub fn create_compute_pipeline(
        &self, 
        blueprint: Pipeline<Compute>,
        bg_layouts: Vec<wgpu::BindGroupLayout>
    ) -> Result<PipelineHandle, String> {
        let shader_path = blueprint.shader_path
            .as_ref()
            .expect("[Compute Pipeline] Expected pipeline to be configured with a path to a shader, but none was found");

        let shader_source = match std::fs::read_to_string(&shader_path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("[Compute Pipeline] Failed to read shader file '{}': {e}", shader_path));
            }
        };

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{}_source", blueprint.label)),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source))
        });

        let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
            .iter()
            .map(|layout| layout)
            .collect();

        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} Layout", blueprint.label)),
            bind_group_layouts: &bg_layout_refs,
            immediate_size: 0,
        });

        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&blueprint.label),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some(&blueprint.ty.main),
            compilation_options: Default::default(),
            cache: None
        });

        println!("[GpuContext] Created new compute pipeline with label '{}'", blueprint.label);

        Ok(PipelineHandle::Compute(pipeline))
    }
}