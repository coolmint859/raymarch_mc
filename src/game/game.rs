use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    KeyAction, MouseAction, controls::{CameraController, KeyboardHandler, MouseHandler}, game::VoxelWorld, graphics::*,
};

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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EnvironmentUniform {
    pub sun_dir: [f32; 4],
    pub sun_color: [f32; 4],
    pub sky_zenith: [f32; 4],
    pub sky_horizon: [f32; 4]
}

pub struct Game {
    camera: PerspectiveCamera,
    controller: CameraController,
    default_cam_pos: Vec3,
    world: VoxelWorld,

    cam_id: BufferId,
    env_id: BufferId,
    rtex_id: TextureId,

    voxel_bg_id: BindGroupId,
    blit_bg_id: BindGroupId,

    voxel_pip_id: PipelineId,
    blit_pip_id: PipelineId,
}

impl Game {
    pub fn init(context: &mut GpuContext, config: &wgpu::SurfaceConfiguration) -> Self {
        let default_cam_pos = glam::vec3(0.0, 0.0, -2.0);
        let controller = CameraController::new(5.0, 0.003);

        let mut camera = PerspectiveCamera::new();
        camera.transform.move_to(default_cam_pos.clone());
        let world = VoxelWorld::new();

        let cam_id = BufferId("main_camera");
        let cam_buf_builder = BufferBuilder::as_uniform(BufferContents::Empty(128))
            .with_label("Camera Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        context.request_buffer(&cam_id, &cam_buf_builder);

        let env_id = BufferId("environment");
        let env_buf_builder = BufferBuilder::as_uniform(BufferContents::Empty(64))
            .with_label("Environment Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        context.request_buffer(&env_id, &env_buf_builder);

        let rtex_id = TextureId("render_texture");
        let rtex_builder = TextureBuilder::new(TextureType::Computed { width: config.width, height: config.height})
            .with_label("Voxel Storage Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING);
        context.request_texture(&rtex_id, &rtex_builder);

        let voxel_bg_id = BindGroupId("voxel_bind_group");
        let voxel_bg_builder = BindGroupBuilder::new()
            .with_label("Compute Bind Group")
            .with_buffer(cam_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_buffer(env_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_texture(rtex_id.clone(), TextureRole::Storage, wgpu::ShaderStages::COMPUTE);
        context.request_bind_group(&voxel_bg_id, &voxel_bg_builder);

        let voxel_shader = context.gpu.device.create_shader_module(wgpu::include_wgsl!("../../shaders/ray_march.wgsl"));

        let voxel_pip_id = PipelineId("voxel_pipeline");
        let voxel_pip_builder = ComputePipelineBuilder::new()
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[voxel_bg_id])
            .with_shader(voxel_shader);
        context.request_pipeline(&voxel_pip_id, PipelineBuilder::Compute(&voxel_pip_builder));

        let blit_bg_id = BindGroupId("blit_bind_group");
        let blit_bg_builder = BindGroupBuilder::new()
            .with_label("Blit Bind Group")
            .with_texture(rtex_id.clone(), TextureRole::Sampled { filterable: true }, wgpu::ShaderStages::FRAGMENT);
        context.request_bind_group(&blit_bg_id, &blit_bg_builder);

        let blit_shader = context.gpu.device.create_shader_module(wgpu::include_wgsl!("../../shaders/blit.wgsl"));
        
        let blit_pip_id = PipelineId("blit_pipeline");
        let blit_pip_builder = RenderPipelineBuilder::new()
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[blit_bg_id])
            .with_shader(blit_shader)
            .with_target_format(config.format);
        context.request_pipeline(&blit_pip_id, PipelineBuilder::Render(&blit_pip_builder));

        Self {
            camera,
            controller,
            default_cam_pos,
            world,
            cam_id,
            env_id,
            rtex_id,
            voxel_bg_id,
            voxel_pip_id,
            blit_bg_id,
            blit_pip_id
        }
    }

    pub fn init_input(
        &self, 
        keyboard: &mut KeyboardHandler<KeyAction>, 
        mouse: &mut MouseHandler<MouseAction>
    ) {
        keyboard.register_key(KeyCode::KeyW, KeyAction::MoveForward);
        keyboard.register_key(KeyCode::KeyA, KeyAction::StrafeLeft);
        keyboard.register_key(KeyCode::KeyS, KeyAction::MoveBackward);
        keyboard.register_key(KeyCode::KeyD, KeyAction::StrafeRight);
        keyboard.register_key(KeyCode::ShiftLeft, KeyAction::MoveUp);
        keyboard.register_key(KeyCode::Space, KeyAction::MoveDown);
        keyboard.register_key(KeyCode::Escape, KeyAction::Exit);
        keyboard.register_key(KeyCode::KeyR, KeyAction::ResetCamera);

        mouse.register_button(MouseButton::Left, MouseAction::LockMouse);
        mouse.register_button(MouseButton::Right, MouseAction::UnlockMouse);
    }

    pub fn on_resize(&mut self, context: &mut GpuContext, canvas: &Canvas) {
        let canvas_desc = canvas.info();

        context.remove_texture(&self.rtex_id);
        let rtex_builder = TextureBuilder::new(TextureType::Computed { width: canvas_desc.width, height: canvas_desc.height})
            .with_label("Voxel Storage Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING);
        context.request_texture(&self.rtex_id, &rtex_builder);

        context.remove_bind_group(&self.voxel_bg_id);
        let voxel_bg_builder = BindGroupBuilder::new()
            .with_label("Compute Bind Group")
            .with_buffer(self.cam_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_buffer(self.env_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_texture(self.rtex_id.clone(), TextureRole::Storage, wgpu::ShaderStages::COMPUTE);
        context.request_bind_group(&self.voxel_bg_id, &voxel_bg_builder);

        context.remove_bind_group(&self.blit_bg_id);
        let blit_bg_builder = BindGroupBuilder::new()
            .with_label("Blit Bind Group")
            .with_texture(self.rtex_id.clone(), TextureRole::Sampled { filterable: true }, wgpu::ShaderStages::FRAGMENT);
        context.request_bind_group(&self.blit_bg_id, &blit_bg_builder);
    }

    pub fn process_input(
        &mut self, 
        keyboard: &mut KeyboardHandler<KeyAction>, 
        mouse: &mut MouseHandler<MouseAction>, 
        dt: f32
    ) {
        let dm = mouse.poll_motion();
        if dm.dx != 0.0 || dm.dy != 0.0 {
            self.controller.rotate_delta(&mut self.camera, dm.dx, dm.dy);
        }

        for action in keyboard.poll_on_held() {
            match action {
                KeyAction::MoveForward => self.controller.move_forward(&mut self.camera, dt),
                KeyAction::MoveBackward => self.controller.move_backward(&mut self.camera, dt),
                KeyAction::StrafeLeft => self.controller.strafe_left(&mut self.camera, dt),
                KeyAction::StrafeRight => self.controller.strafe_right(&mut self.camera, dt),
                KeyAction::MoveUp => self.controller.move_up(&mut self.camera, dt),
                KeyAction::MoveDown => self.controller.move_down(&mut self.camera, dt),
                KeyAction::ResetCamera => {
                    self.camera.transform.move_to(self.default_cam_pos);
                    self.camera.transform.set_rotation(Quat::IDENTITY);
                    self.controller.reset_delta();
                },
                _ => {}
            }
        }
    }

    pub fn update(&mut self, context: &mut GpuContext, canvas: &Canvas, dt: f32) {
        self.world.update(dt);

        context.update_buffer(&self.cam_id, BufferUpdate {
            data_struct: CameraUniform::build_from(&mut self.camera, canvas.info().aspect),
            offset: 0
        });

        context.update_buffer(&self.env_id, BufferUpdate { 
            data_struct: self.world.calc_environment(),
            offset: 0
        });
    }

    pub fn create_passes(&mut self, canvas: &Canvas) -> Vec<GpuPass> {
        let canvas_desc = canvas.info();
        let wx = (canvas_desc.width + 15) / 16;
        let wy = (canvas_desc.height + 15) / 16;

        vec![
            GpuPass::Compute(ComputePass {
                pipeline_id: self.voxel_pip_id,
                bind_groups: vec![self.voxel_bg_id],
                work_groups: (wx, wy, 1)
            }),
            GpuPass::Render(RenderPass { 
                pipeline_id: self.blit_pip_id,
                bind_groups: vec![self.blit_bg_id], 
                vertex_count: 3, 
                instance_count: 1 
            })
        ]
    }
}