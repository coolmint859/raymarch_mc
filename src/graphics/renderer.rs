use std::ops::Deref;

use glam::Vec3;

use crate::graphics::{
    BindGroupBuilder, BindGroupHandle, BufferBuilder, BufferContents, BufferHandle, Camera3D, Canvas, GpuHandle, RenderPipelineBuilder, RenderPipelineHandle, RenderTarget, ResourceBuilder
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
    cam_bind_group: BindGroupHandle,
    voxel_pipeline: RenderPipelineHandle,
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

        let cam_bind_group = BindGroupBuilder::new()
            .with_label("Camera Bind Group")
            .with_resource(wgpu::ShaderStages::FRAGMENT, cam_buf_handle.clone().into())
            .build(gpu.clone());

        let shader = gpu.device.create_shader_module(wgpu::include_wgsl!("../../shaders/shader.wgsl"));
        
        let voxel_pipeline = RenderPipelineBuilder::new()
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[cam_bind_group.layout.clone()])
            .with_shader(shader)
            .with_target_format(config.format)
            .build(gpu.clone());

        Self {
            camera: Camera3D::new(),
            cam_buffer: cam_buf_handle,
            cam_bind_group,
            voxel_pipeline
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
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: & [Some(wgpu::RenderPassColorAttachment {
                    view: target.get_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.05, b: 0.1, a: 1.0}),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.voxel_pipeline);
            render_pass.set_bind_group(0, self.cam_bind_group.deref(), &[]);
            render_pass.draw(0..3, 0..1);
        }

        encoder.finish()
    }
}