use std::{collections::HashMap, ops::Deref, sync::Arc};

use wgpu::{BindGroupLayout, CommandEncoder, util::DeviceExt};

use crate::graphics::*;

/// Represents a render pass
pub struct RenderPass {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub vertex_count: u32,
    pub instance_count: u32
}

/// Represents a compute pass
pub struct ComputePass {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub work_groups: (u32, u32, u32) // x, y, z
}

/// Represents a render or compute pass.
pub enum GpuPass {
    Render(RenderPass),
    Compute(ComputePass),
}

/// builders for gpu pipelines
pub enum PipelineBuilder<'a> {
    Render(&'a RenderPipelineBuilder),
    Compute(&'a ComputePipelineBuilder)
}

/// unique identifier to a buffer
#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub struct BufferId(pub &'static str);

/// unique identifier for a texture
#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub struct TextureId(pub &'static str);

/// unique identifier for a pipeline
#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub struct PipelineId(pub &'static str);

/// unique identifier for a bind group
#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub struct BindGroupId(pub &'static str);

/// Esncasulates updates to gpu buffers
pub struct BufferUpdate<T: bytemuck::Pod> {
    pub offset: u64,
    pub data_struct: T
}

/// Represents the state of the gpu, providing means to create and modify resources, and execute pipelines
pub struct GpuContext {
    pub gpu: GpuHandle,

    buffers: HashMap<BufferId, BufferHandle>,
    textures: HashMap<TextureId, TextureHandle>,
    bind_groups: HashMap<BindGroupId, BindGroupHandle>,
    r_pipelines: HashMap<PipelineId, RenderPipelineHandle>,
    c_pipelines: HashMap<PipelineId, ComputePipelineHandle>
}

impl GpuContext {
    pub fn new(gpu: &GpuHandle) -> Self {
        Self {
            buffers: HashMap::new(),
            textures: HashMap::new(),
            bind_groups: HashMap::new(),
            r_pipelines: HashMap::new(),
            c_pipelines: HashMap::new(),
            gpu: gpu.clone()
        }
    }

    /// Request a buffer to be created from the provided builder and mapped to the provided id.
    pub fn request_buffer(&mut self, id: &BufferId, builder: &BufferBuilder) {
        if self.buffers.contains_key(id) { return; }

        let buffer = create_buffer(self.gpu.clone(), builder);
        self.buffers.insert(id.clone(), buffer);
    }

    /// Request a texture to be created from the provided builder and mapped to the provided id.
    pub fn request_texture(&mut self, id: &TextureId, builder: &TextureBuilder) {
        if self.textures.contains_key(id) { return; }

        let texture = create_texture(self.gpu.clone(), builder);
        self.textures.insert(id.clone(), texture);
    }

    /// Request a bind group to be created from the provided builder and mapped to the provided id.
    pub fn request_bind_group(&mut self, id: &BindGroupId, builder: &BindGroupBuilder) {
        if self.bind_groups.contains_key(id) { return; }

        let entries = builder.bindings
            .iter()
            .map(|binding| {
                let resource = match &binding.target {
                    BindingTarget::Buffer(id) => {
                        let buffer = self.buffers.get(id).unwrap();
                        wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding())
                    }
                    BindingTarget::Texture(id) => {
                        let texture = self.textures.get(id).unwrap();
                        wgpu::BindingResource::TextureView(texture)
                    }
                };

                wgpu::BindGroupEntry {
                    resource,
                    binding: binding.slot
                }
            })
            .collect();

        let bind_group = create_bind_group(self.gpu.clone(), builder, entries);
        self.bind_groups.insert(id.clone(), bind_group);
    }

    /// Request a pipeline to be created from the provided builder and mapped to the provided id.
    pub fn request_pipeline(&mut self, id: &PipelineId, builder: PipelineBuilder) {
        if self.r_pipelines.contains_key(id) || self.c_pipelines.contains_key(id) { return; }

        match builder {
            PipelineBuilder::Render(r_pip_builder) => {
                let bg_layouts: Vec<&'_ BindGroupLayout> = r_pip_builder.bg_layouts
                    .iter()
                    .map(|bg_id| {
                        let bg = self.bind_groups.get(bg_id).unwrap();
                        
                        bg.layout.as_ref()
                    })
                    .collect();
            
                let r_pipeline= create_render_pipeline(self.gpu.clone(), r_pip_builder, &bg_layouts);
                self.r_pipelines.insert(id.clone(), r_pipeline);
            },
            PipelineBuilder::Compute(c_pip_builder) => {
                let bg_layouts: Vec<&'_ BindGroupLayout> = c_pip_builder.bg_layouts
                    .iter()
                    .map(|bg_id| {
                        let bg = self.bind_groups.get(bg_id).unwrap();
                        
                        bg.layout.as_ref()
                    })
                    .collect();

                let c_pipeline = create_compute_pipeline(self.gpu.clone(), c_pip_builder, &bg_layouts);
                self.c_pipelines.insert(id.clone(), c_pipeline);
            }
        };
    }

    pub fn update_buffer<T: bytemuck::Pod>(&mut self, id: &BufferId, update: BufferUpdate<T>) {
        if let Some(buffer) = self.buffers.get(id) {
            let data = bytemuck::bytes_of(&update.data_struct);
            let update_size = update.offset + data.len() as u64;
            assert!(update_size <= buffer.size());

            self.gpu.queue.write_buffer(buffer, update.offset, data);
        }
    }

    /// Remove a texture from the gpu, releasing the vram allocation
    pub fn remove_texture(&mut self, id: &TextureId) {
        self.textures.remove(id);
    }

    /// Remove a bind group from the gpu, releasing the vram allocation
    pub fn remove_bind_group(&mut self, id: &BindGroupId) {
        self.bind_groups.remove(id);
    }

    /// Execute the provided gpu passes in order
    pub fn execute_passes(&self, passes: Vec<GpuPass>, canvas: &Canvas) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let output = canvas.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let format = canvas.config.format;

        for pass in passes {
            match pass {
                GpuPass::Render(pass) => self.execute_render_pass(&mut encoder, pass, &view),
                GpuPass::Compute(pass) => self.execute_compute_pass(&mut encoder, pass),
            }
        }
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Execute a render pass on the provided encoder
    fn execute_render_pass(&self, encoder: &mut CommandEncoder, pass: RenderPass, view: &wgpu::TextureView) {
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            ..Default::default()
        });

        let pipeline = self.r_pipelines.get(&pass.pipeline_id).unwrap();
        render_pass.set_pipeline(pipeline);

        for (idx, bg_id) in pass.bind_groups.iter().enumerate() {
            let bg = self.bind_groups.get(&bg_id).unwrap();
            render_pass.set_bind_group(idx as u32, bg.deref(), &[]);
        }
        render_pass.draw(0..pass.vertex_count, 0..pass.instance_count);
    }

    /// Execute a compute pass on the provided encoder
    fn execute_compute_pass(&self, encoder: &mut CommandEncoder, pass: ComputePass) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            ..Default::default()
        });

        let pipeline = self.c_pipelines.get(&pass.pipeline_id).unwrap();
        compute_pass.set_pipeline(pipeline);

        for (idx, bg_id) in pass.bind_groups.iter().enumerate() {
            let bg = self.bind_groups.get(&bg_id).unwrap();
            compute_pass.set_bind_group(idx as u32, bg.deref(), &[]);
        }

        let (wx, wy, wz) = pass.work_groups;
        compute_pass.dispatch_workgroups(wx, wy, wz);
    }
}

/// Create a buffer from the given configuration builder
pub fn create_buffer(gpu: GpuHandle, builder: &BufferBuilder) -> BufferHandle {
    let buffer = match &builder.contents {
        BufferContents::Empty(size) => {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&builder.label),
                size: *size,
                usage: builder.usage,
                mapped_at_creation: false
            })
        },
        BufferContents::WithData(data) => {
            gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&builder.label),
                contents: &data,
                usage: builder.usage
            })
        }
    };

    println!("[GpuContext] Created new buffer with label '{}'", builder.label);

    BufferHandle {
        buffer: Arc::new(buffer),
    }
}

/// Create a new texture from the given configuration builder
pub fn create_texture(gpu: GpuHandle, builder: &TextureBuilder) -> TextureHandle {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&builder.label),
        size: builder.texture_type.extent(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: builder.dimensions,
        format: builder.format,
        usage: builder.usage,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    println!("[GpuContext] Created new texture with label '{}'", builder.label);

    TextureHandle {
        texture: Arc::new(texture),
        view: Arc::new(view),
    }
}

/// Create a new bind group from the given configuration builder and resource map
pub fn create_bind_group(
    gpu: GpuHandle, 
    builder: &BindGroupBuilder, 
    entries: Vec<wgpu::BindGroupEntry<'_>>
) -> BindGroupHandle {
    let layout = Arc::new(gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some(&format!("Layout: {}", builder.label)),
        entries: &builder.layout_entries
    }));

    let bind_group = Arc::new(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&builder.label),
        layout: &layout,
        entries: &entries,
    }));

    println!("[GpuContext] Created new bind group with label '{}'", builder.label);

    BindGroupHandle { 
        layout, 
        bind_group 
    }
}

/// Create a new render pipeline from the given configuration builder
pub fn create_render_pipeline(
    gpu: GpuHandle, 
    builder: &RenderPipelineBuilder,
    bg_layouts: &[&'_ BindGroupLayout]
) -> RenderPipelineHandle {
    let shader = builder.shader_module.as_ref()
        .expect("[Render Pipeline] Expected pipeline to be configured with a shader module, but none was found.");
    let format = builder.target_format
        .expect("[Render Pipeline] Expected pipeline to be configured with a target format, but none was found.");

    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{}_layout", builder.label)),
        bind_group_layouts: bg_layouts,
        immediate_size: 0,
    });

    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&builder.label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some(&builder.vs_main),
            compilation_options: Default::default(),
            buffers: &[], // Full-screen procedurally drawn triangle requires no input VBO buffers!
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(&builder.fs_main),
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

    println!("[GpuContext] Created new render pipeline with label '{}'", builder.label);

    RenderPipelineHandle {
        pipeline: Arc::new(pipeline),
    }
}

pub fn create_compute_pipeline(
    gpu: GpuHandle, 
    builder: &ComputePipelineBuilder,
    bg_layouts: &[&'_ BindGroupLayout]
) -> ComputePipelineHandle {
    let shader = builder.shader_module.as_ref()
        .expect("[Compute Pipeline] Expected pipeline to be configured with a shader module, but none was found.");

    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{} Layout", builder.label)),
        bind_group_layouts: bg_layouts,
        immediate_size: 0,
    });

    let pipeline = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&builder.label),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some(&builder.main),
        compilation_options: Default::default(),
        cache: None
    });

    println!("[GpuContext] Created new compute pipeline with label '{}'", builder.label);

    ComputePipelineHandle { 
        pipeline: Arc::new(pipeline) 
    }
}