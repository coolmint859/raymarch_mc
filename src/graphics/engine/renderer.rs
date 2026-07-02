use std::ops::Deref;

use crate::{game::EnvironmentUniform, graphics::{
    BindGroupBuilder, BindGroupHandle, BufferBuilder, BufferContents, BufferHandle, ComputePipelineBuilder, ComputePipelineHandle, GpuHandle, PerspectiveCamera, RenderPipelineBuilder, RenderPipelineHandle, RenderTarget, ResourceBuilder, ResourceHandle, TextureBuilder, TextureType
}};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4]
}

impl CameraUniform {
    pub fn build_from(camera: &mut PerspectiveCamera, aspect: f32) -> Self {
        let view_proj = camera.get_view_proj(aspect);
        let inv_view_proj = view_proj.inverse();

        CameraUniform { 
            view_proj: view_proj.to_cols_array_2d(), 
            inv_view_proj: inv_view_proj.to_cols_array_2d()
        }
    }
}

/// Executes rendering pipelines
pub struct Renderer {
    cam_buffer: BufferHandle,
    env_buffer: BufferHandle,
    compute_bg: BindGroupHandle,
    voxel_pipeline: ComputePipelineHandle,

    blit_bg: BindGroupHandle,
    blit_pipeline: RenderPipelineHandle
}

impl Renderer {
    pub fn new(gpu: &GpuHandle, config: &wgpu::SurfaceConfiguration) -> Self {
        let cam_buffer = BufferBuilder::as_uniform(BufferContents::Empty(128))
            .with_label("Camera Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST)
            .build(gpu.clone());

        let env_buffer = BufferBuilder::as_uniform(BufferContents::Empty(64))
            .with_label("Environment Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST)
            .build(gpu.clone());

        let render_texture = TextureBuilder::new(TextureType::Computed { width: config.width, height: config.height})
            .with_label("Voxel Storage Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .build(gpu.clone());

        let compute_bg = BindGroupBuilder::new()
            .with_label("Compute Bind Group")
            .with_resource(wgpu::ShaderStages::COMPUTE, cam_buffer.clone().into())
            .with_resource(wgpu::ShaderStages::COMPUTE, env_buffer.clone().into())
            .with_resource(wgpu::ShaderStages::COMPUTE, ResourceHandle::StorageTexture(render_texture.clone()))
            .build(gpu.clone());

        let voxel_shader = gpu.device.create_shader_module(wgpu::include_wgsl!("../../../shaders/ray_march.wgsl"));

        let voxel_pipeline = ComputePipelineBuilder::new()
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[compute_bg.layout.clone()])
            .with_shader(voxel_shader)
            .build(gpu.clone());

        let blit_bg = BindGroupBuilder::new()
            .with_label("Blit Bind Group")
            .with_resource(wgpu::ShaderStages::FRAGMENT, ResourceHandle::SampledTexture(render_texture.clone()))
            .build(gpu.clone());

        let blit_shader = gpu.device.create_shader_module(wgpu::include_wgsl!("../../../shaders/blit.wgsl"));
        
        let blit_pipeline = RenderPipelineBuilder::new()
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[blit_bg.layout.clone()])
            .with_shader(blit_shader)
            .with_target_format(config.format)
            .build(gpu.clone());

        Self {
            cam_buffer,
            env_buffer,
            compute_bg,
            blit_bg,
            voxel_pipeline,
            blit_pipeline
        }
    }

    pub fn update_camera(&mut self, gpu: GpuHandle, camera: &mut PerspectiveCamera, aspect: f32) {
        let cam_uniform = CameraUniform::build_from(camera, aspect);
        let cam_bytes = bytemuck::bytes_of(&cam_uniform);

        gpu.queue.write_buffer(&self.cam_buffer, 0, cam_bytes);
    }

    pub fn update_env(&mut self, gpu: GpuHandle, env_uniform: EnvironmentUniform) {
        let env_bytes = bytemuck::bytes_of(&env_uniform);

        gpu.queue.write_buffer(&self.env_buffer, 0, env_bytes);
    }

    /// Render the currently set render pipeline to the window
    pub fn render(&self, gpu: GpuHandle, target: &impl RenderTarget) -> wgpu::CommandBuffer {
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                ..Default::default()
            });

            compute_pass.set_pipeline(&self.voxel_pipeline);
            compute_pass.set_bind_group(0, self.compute_bg.deref(), &[]);

            let (width, height) = target.dimensions();
            let workgroups_x = (width + 15) / 16;
            let workgroups_y  = (height + 15) / 16;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                    view: target.get_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.blit_pipeline);
            render_pass.set_bind_group(0, self.blit_bg.deref(), &[]);
            render_pass.draw(0..3, 0..1);
        }

        encoder.finish()
    }
}