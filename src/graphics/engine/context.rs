use std::{borrow::Cow, collections::HashMap, ops::Deref, sync::Arc};

use wgpu::{BindGroupLayout, CommandEncoder, util::DeviceExt};

use crate::graphics::*;

/// Represents a render pass
#[derive(Clone, Debug)]
pub struct RenderPass {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub vertex_count: u32,
    pub instance_count: u32
}

/// Represents a compute pass
#[derive(Clone, Debug)]
pub struct ComputePass {
    pub pipeline_id: PipelineId,
    pub bind_groups: Vec<BindGroupId>,
    pub work_groups: (u32, u32, u32) // x, y, z
}

/// Represents a render or compute pass.
#[derive(Clone, Debug)]
pub enum GpuPass {
    Render(RenderPass),
    Compute(ComputePass),
}

/// unique identifier to a buffer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct BufferId(pub &'static str);

/// unique identifier for a texture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct TextureId(pub &'static str);

/// unique identifier for a pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct PipelineId(pub &'static str);

/// unique identifier for a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct BindGroupId(pub &'static str);

/// Esncasulates updates to gpu buffers
pub struct BufferUpdate<T: bytemuck::Pod> {
    pub offset: u64,
    pub data_struct: T
}

/// Represents the state of the gpu, providing means to create and modify resources, and execute pipelines
pub struct GpuContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pass_queue: Vec<GpuPass>,
    validator: GpuValidator,

    def_bind_groups: HashMap<BindGroupId, BindGroupBuilder>,
    def_pipelines: HashMap<PipelineId, PipelineBuilder>,

    pub(crate) buffers: ResourceHandler<BufferId, BufferHandle>,
    pub(crate) textures: ResourceHandler<TextureId, TextureHandle>,
    pub(crate) bind_groups: ResourceHandler<BindGroupId, BindGroupHandle>,
    pub(crate) r_pipelines: ResourceHandler<PipelineId, RenderPipelineHandle>,
    pub(crate) c_pipelines: ResourceHandler<PipelineId, ComputePipelineHandle>,

    pub(crate) bg_blueprints: HashMap<BindGroupId, BindGroupBuilder>,
    pub(crate) pip_blueprints: HashMap<PipelineId, PipelineBuilder>,
}

impl GpuContext {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            pass_queue: Vec::new(),
            validator: GpuValidator::new(),

            def_bind_groups: HashMap::new(),
            def_pipelines: HashMap::new(),

            buffers: ResourceHandler::new(),
            textures: ResourceHandler::new(),
            bind_groups: ResourceHandler::new(),
            r_pipelines: ResourceHandler::new(),
            c_pipelines: ResourceHandler::new(),
            bg_blueprints: HashMap::new(),
            pip_blueprints: HashMap::new()
        }
    }

    /// Reconfigure the surface texture to match the canvas
    pub fn configure_surface(&self, canvas: &mut Canvas) {
        canvas.surface.configure(&self.device, &canvas.config);
    }

    /// Request a buffer to be created from the provided builder and mapped to the provided id.
    pub fn request_buffer(&mut self, id: &BufferId, builder: BufferBuilder) {
        if self.buffers.contains(id) { return; }

        let buffer_task = Task::non_blocking(
            create_buffer(self.device.clone(), builder)
        );
        self.buffers.request_new(id, buffer_task);
    }

    /// Request a texture to be created from the provided builder and mapped to the provided id.
    pub fn request_texture(&mut self, id: &TextureId, builder: TextureBuilder) {
        if self.textures.contains(id) { return; }

        let texture_task = Task::non_blocking(
            create_texture(self.device.clone(), builder)
        );
        self.textures.request_new(id, texture_task);
    }

    /// Request a bind group to be created from the provided builder and mapped to the provided id.
    pub fn request_bind_group(&mut self, id: &BindGroupId, builder: &BindGroupBuilder) {
        if self.bind_groups.contains(id) { return; }

        if !self.bg_blueprints.contains_key(id) {
            self.bg_blueprints.insert(*id, builder.clone());
        }

        let mut buffer_handles = Vec::new();
        let mut texture_handles = Vec::new();
        let mut expected_tex_len = 0usize;
        let mut expected_buf_len = 0usize;

        for binding in &builder.bindings {
            match &binding.target {
                BindingTarget::Buffer(buf_id) => {
                    if let Some(handle) = self.buffers.get(buf_id) {
                        buffer_handles.push((*buf_id, handle.clone(), binding.slot));
                    }
                    expected_buf_len += 1;
                },
                BindingTarget::Texture(tex_id) => {
                    if let Some(handle) = self.textures.get(tex_id) {
                        texture_handles.push((*tex_id, handle.clone(), binding.slot));
                    }
                    expected_tex_len += 1;
                }
            }
        }

        let ok_buffers = expected_buf_len == buffer_handles.len();
        let ok_textures = expected_tex_len == texture_handles.len();

        if !(ok_buffers && ok_textures) { 
            self.def_bind_groups.insert(*id, builder.clone());
            return; 
        };

        self.def_bind_groups.remove(id);

        let device = self.device.clone();
        let builder = builder.clone();

        let bind_group_task = Task::non_blocking(async move {
            let mut entries = Vec::new();

            for (_id, buf, slot) in &buffer_handles {
                entries.push(wgpu::BindGroupEntry {
                    binding: *slot,
                    resource: buf.as_entire_binding()
                });
            }
            for (_id, tex, slot) in &texture_handles {
                entries.push(wgpu::BindGroupEntry {
                    binding: *slot,
                    resource: wgpu::BindingResource::TextureView(tex)
                });
            }

            create_bind_group(device, builder, entries).await
        });

        self.bind_groups.request_new(id, bind_group_task);
    }

    /// Request a pipeline to be created from the provided builder and mapped to the provided id.
    pub fn request_pipeline(&mut self, id: &PipelineId, builder: PipelineBuilder) {
        if self.r_pipelines.contains(id) || self.c_pipelines.contains(id) { return; }

        if !self.pip_blueprints.contains_key(id) {
            self.pip_blueprints.insert(*id, builder.clone());
        }

        let pip_bgs = builder.bind_groups();
        let mut bg_layouts = Vec::new();

        for bg_id in pip_bgs {
            if let Some(bind_group) = self.bind_groups.get(bg_id) {
                bg_layouts.push(bind_group.layout.clone())
            }
        }

        if pip_bgs.len() != bg_layouts.len() {
            self.def_pipelines.insert(*id, builder.clone());
            return; 
        }

        self.def_pipelines.remove(id);
        match builder {
            PipelineBuilder::Render(r_pip_builder) => {
                let r_pip_task = Task::non_blocking(
                    create_render_pipeline(self.device.clone(), r_pip_builder.clone(), bg_layouts)
                );

                self.r_pipelines.request_new(id, r_pip_task);
            },
            PipelineBuilder::Compute(c_pip_builder) => {
                let c_pip_task = Task::non_blocking(
                    create_compute_pipeline(self.device.clone(), c_pip_builder.clone(), bg_layouts)
                );
                self.c_pipelines.request_new(id, c_pip_task);
            }
        };
    }

    pub fn prepare_frame(&mut self) {
        self.buffers.sync();
        self.textures.sync();
        self.bind_groups.sync();
        self.r_pipelines.sync();
        self.c_pipelines.sync();

        let pending_bgs = std::mem::take(&mut self.def_bind_groups);
        for (id, builder) in &pending_bgs {
            self.request_bind_group(id, builder);
        }

        let pending_pips = std::mem::take(&mut self.def_pipelines);
        for (id, builder) in &pending_pips {
            self.request_pipeline(id, builder.clone());
        }
    }

    pub fn update_buffer<T: bytemuck::Pod>(&mut self, id: &BufferId, update: BufferUpdate<T>) {
        if let Some(buffer) = self.buffers.get(id) {
            let data = bytemuck::bytes_of(&update.data_struct);
            let update_size = update.offset + data.len() as u64;
            assert!(update_size <= buffer.size());

            self.queue.write_buffer(buffer, update.offset, data);
        }
    }

    /// Remove a texture from the gpu, releasing the vram allocation
    pub fn remove_texture(&mut self, id: &TextureId) {
        self.textures.remove(id);
        self.validator.invalidate_texture(id, self);
    }

    /// Remove a buffer from the gpu, releasing the vram allocation
    pub fn remove_buffer(&mut self, id: &BufferId) {
        self.buffers.remove(id);
        self.validator.invalidate_buffer(id, self);
    }

    /// Remove a bind group from the gpu, releasing the vram allocation
    pub fn remove_bind_group(&mut self, id: &BindGroupId) {
        self.bind_groups.remove(id);
        self.validator.invalidate_bind_group(id, self);
    }

    /// Remove a pipeline from the gpu, releasing the vram allocation
    pub fn remove_pipeline(&mut self, id: &PipelineId) {
        self.r_pipelines.remove(id);
        self.c_pipelines.remove(id);
        self.validator.invalidate_pipeline(id);
    }

    pub fn add_pass(&mut self, pass: GpuPass) {
        self.pass_queue.push(pass);
    }

    /// Execute the gpu passes added to the pass queue.
    pub fn finish(&mut self, canvas: &Canvas) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let output = canvas.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let format = canvas.config.format;

        let passes = std::mem::take(&mut self.pass_queue);
        for pass in passes {
            match pass {
                GpuPass::Render(pass) => self.execute_render_pass(&mut encoder, pass, &view),
                GpuPass::Compute(pass) => self.execute_compute_pass(&mut encoder, pass),
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Execute a render pass on the provided encoder
    fn execute_render_pass(&self, encoder: &mut CommandEncoder, pass: RenderPass, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            ..Default::default()
        });

        let Some(pipeline) = self.validator.validate_render_pipeline(&pass.pipeline_id, self) else { 
            // println!("[GpuContext] Failed to validate render pipeline @{:?}", pass.pipeline_id);
            return; 
        };
        render_pass.set_pipeline(pipeline);

        for (idx, bg_id) in pass.bind_groups.iter().enumerate() {
            let Some(bg) = self.validator.verify_bind_group(bg_id, self) else { 
                // println!("[GpuContext] Failed to validate bind group @{:?} for render pipeline @{:?}", bg_id, pass.pipeline_id);
                return; 
            };
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

        let Some(pipeline) = self.validator.validate_compute_pipeline(&pass.pipeline_id, self) else {
            // println!("[GpuContext] Failed to validate compute pipeline @{:?}", pass.pipeline_id);
            return; 
        };
        compute_pass.set_pipeline(pipeline);

        for (idx, bg_id) in pass.bind_groups.iter().enumerate() {
            let Some(bg) = self.validator.verify_bind_group(bg_id, self) else { 
                // println!("[GpuContext] Failed to validate bind group @{:?} for compute pipeline @{:?}", bg_id, pass.pipeline_id);
                return; 
            };
            compute_pass.set_bind_group(idx as u32, bg.deref(), &[]);
        }

        let (wx, wy, wz) = pass.work_groups;
        compute_pass.dispatch_workgroups(wx, wy, wz);
    }
}

/// Create a buffer from the given configuration builder
pub async fn create_buffer(device: Arc<wgpu::Device>, builder: BufferBuilder) -> Result<BufferHandle, String> {
    let buffer = match &builder.contents {
        BufferContents::Empty(size) => {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&builder.label),
                size: *size,
                usage: builder.usage,
                mapped_at_creation: false
            })
        },
        BufferContents::WithData(data) => {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&builder.label),
                contents: &data,
                usage: builder.usage
            })
        }
    };

    println!("[GpuContext] Created new buffer with label '{}'", builder.label);

    Ok(BufferHandle {
        buffer: Arc::new(buffer),
    })
}

/// Create a new texture from the given configuration builder
pub async fn create_texture(device: Arc<wgpu::Device>, builder: TextureBuilder) -> Result<TextureHandle, String> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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

    Ok(TextureHandle {
        texture: Arc::new(texture),
        view: Arc::new(view),
    })
}

/// Create a new bind group from the given configuration builder and resource map
pub async fn create_bind_group(
    device: Arc<wgpu::Device>, 
    builder: BindGroupBuilder, 
    entries: Vec<wgpu::BindGroupEntry<'_>>
) -> Result<BindGroupHandle, String> {
    let layout = Arc::new(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some(&format!("Layout: {}", builder.label)),
        entries: &builder.layout_entries
    }));

    let bind_group = Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
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
    device: Arc<wgpu::Device>, 
    builder: RenderPipelineBuilder,
    bg_layouts: Vec<Arc<BindGroupLayout>>
) -> Result<RenderPipelineHandle, String> {
    let shader_source = builder.shader_source
        .as_ref()
        .expect("[Render Pipeline] Expected pipeline to be configured with a shader descriptor, but none was found");

    let format = builder.target_format
        .expect("[Render Pipeline] Expected pipeline to be configured with a target format, but none was found.");

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{}_source", builder.label)),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source))
    });

    let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
        .iter()
        .map(|layout| { layout.as_ref() })
        .collect();

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{}_layout", builder.label)),
        bind_group_layouts: &bg_layout_refs,
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

    Ok(RenderPipelineHandle {
        pipeline: Arc::new(pipeline),
    })
}

pub async fn create_compute_pipeline(
    device: Arc<wgpu::Device>, 
    builder: ComputePipelineBuilder,
    bg_layouts: Vec<Arc<BindGroupLayout>>
) -> Result<ComputePipelineHandle, String> {
    let shader_source = builder.shader_source
        .as_ref()
        .expect("[Compute Pipeline] Expected pipeline to be configured with a shader descriptor, but none was found");

    let bg_layout_refs: Vec<&wgpu::BindGroupLayout> = bg_layouts
        .iter()
        .map(|layout| { layout.as_ref() })
        .collect();

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{} Layout", builder.label)),
        bind_group_layouts: &bg_layout_refs,
        immediate_size: 0,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{}_source", builder.label)),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source))
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&builder.label),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some(&builder.main),
        compilation_options: Default::default(),
        cache: None
    });

    println!("[GpuContext] Created new compute pipeline with label '{}'", builder.label);

    Ok(ComputePipelineHandle { 
        pipeline: Arc::new(pipeline) 
    })
}