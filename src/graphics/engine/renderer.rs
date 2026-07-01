use std::ops::Deref;

use glam::Vec3;

use crate::graphics::{
    BindGroupBuilder, BindGroupHandle, BufferBuilder, BufferContents, BufferHandle, Camera3D, Canvas, ComputePipelineBuilder, ComputePipelineHandle, GpuHandle, RenderPipelineBuilder, RenderPipelineHandle, RenderTarget, ResourceBuilder, ResourceHandle, TextureBuilder, TextureRole, TextureType
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4]
}

impl CameraUniform {
    pub fn build_from(camera: &mut Camera3D, aspect: f32) -> Self {
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
    /// The camera which views the scene rendered to the surface texture
    camera: Camera3D,

    cam_buffer: BufferHandle,
    compute_bg: BindGroupHandle,
    voxel_pipeline: ComputePipelineHandle,

    blit_bg: BindGroupHandle,
    blit_pipeline: RenderPipelineHandle
}

impl Renderer {
    pub fn new(gpu: &GpuHandle, config: &wgpu::SurfaceConfiguration) -> Self {
        let mut camera = Camera3D::new();

        let cam_uniform = CameraUniform::build_from(&mut camera, 1.0);
        let cam_bytes = bytemuck::bytes_of(&cam_uniform).to_vec();
        let cam_buf_handle = BufferBuilder::as_uniform(BufferContents::WithData(cam_bytes))
            .with_label("Camera Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST)
            .build(gpu.clone());

        let render_texture = TextureBuilder::new(TextureType::Computed { width: config.width, height: config.height})
            .with_label("Voxel Storage Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .build(gpu.clone());

        let compute_bg = BindGroupBuilder::new()
            .with_label("Compute Bind Group")
            .with_resource(wgpu::ShaderStages::COMPUTE, cam_buf_handle.clone().into())
            .with_resource(wgpu::ShaderStages::COMPUTE, ResourceHandle::Texture(render_texture.clone(), TextureRole::Storage))
            .build(gpu.clone());

        let voxel_shader = gpu.device.create_shader_module(wgpu::include_wgsl!("../../../shaders/ray_march.wgsl"));

        let voxel_pipeline = ComputePipelineBuilder::new()
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[compute_bg.layout.clone()])
            .with_shader(voxel_shader)
            .build(gpu.clone());

        let blit_bg = BindGroupBuilder::new()
            .with_label("Blit Bind Group")
            .with_resource(wgpu::ShaderStages::FRAGMENT, ResourceHandle::Texture(render_texture.clone(), TextureRole::Sampled))
            .build(gpu.clone());

        let blit_shader = gpu.device.create_shader_module(wgpu::include_wgsl!("../../../shaders/blit.wgsl"));
        
        let blit_pipeline = RenderPipelineBuilder::new()
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[blit_bg.layout.clone()])
            .with_shader(blit_shader)
            .with_target_format(config.format)
            .build(gpu.clone());

        Self {
            camera,
            cam_buffer: cam_buf_handle,
            compute_bg,
            blit_bg,
            voxel_pipeline,
            blit_pipeline
        }
    }

    pub fn update_camera(&mut self, gpu: GpuHandle, canvas: &Canvas, time: f32) {
        let radius = 3.5;
        self.camera.set_position(Vec3 { x: time.sin() * radius, y: 0.0, z: time.cos() * radius});

        let uniforms = CameraUniform::build_from(&mut self.camera, canvas.aspect_ratio);
        gpu.queue.write_buffer(&self.cam_buffer, 0, bytemuck::bytes_of(&uniforms));
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

            let workgroups_x = (1920 + 15) / 16;
            let workgroups_y  = (1080 + 15) / 16;
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