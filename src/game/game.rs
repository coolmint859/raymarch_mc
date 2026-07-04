use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    Graphics, InputEvent, game::{Screen, ScreenTransition, VoxelWorld}, graphics::*, utils::{CameraController, KeyboardHandler, MouseHandler, PerspectiveCamera},
};

#[derive(Clone, Copy)]
pub enum PlayerKeyAction {
    MoveForward,
    MoveBackward,
    StrafeLeft, 
    StrafeRight,
    MoveUp,
    MoveDown,
    ResetCamera,
    Exit,
}

#[derive(Clone, Copy)]
pub enum PlayerMouseAction {
    LockMouse,
    UnlockMouse,
}

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

struct GameIds {
    pub cam_id: BufferId,
    pub env_id: BufferId,
    pub rtex_id: TextureId,

    pub voxel_bg_id: BindGroupId,
    pub blit_bg_id: BindGroupId,

    pub voxel_pip_id: PipelineId,
    pub blit_pip_id: PipelineId,
}

pub struct Game {
    camera: PerspectiveCamera,
    controller: CameraController,
    keyboard: KeyboardHandler<PlayerKeyAction>,
    mouse: MouseHandler<PlayerMouseAction>,

    default_cam_pos: Vec3,
    world: VoxelWorld,

    ids: Option<GameIds>,
}

impl Game {
    pub fn new() -> Self {
        let default_cam_pos = glam::vec3(0.0, 0.0, -2.0);
        let mut camera = PerspectiveCamera::new();
        camera.transform.move_to(default_cam_pos);

        Self {
            camera,
            controller: CameraController::new(5.0, 0.003),
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            default_cam_pos,
            world: VoxelWorld::new(1.0),
            ids: None,
        }
    }

    pub fn init_input(&mut self) {
        self.keyboard.register_key(KeyCode::KeyW, PlayerKeyAction::MoveForward);
        self.keyboard.register_key(KeyCode::KeyA, PlayerKeyAction::StrafeLeft);
        self.keyboard.register_key(KeyCode::KeyS, PlayerKeyAction::MoveBackward);
        self.keyboard.register_key(KeyCode::KeyD, PlayerKeyAction::StrafeRight);
        self.keyboard.register_key(KeyCode::ShiftLeft, PlayerKeyAction::MoveUp);
        self.keyboard.register_key(KeyCode::Space, PlayerKeyAction::MoveDown);
        self.keyboard.register_key(KeyCode::Escape, PlayerKeyAction::Exit);
        self.keyboard.register_key(KeyCode::KeyR, PlayerKeyAction::ResetCamera);

        self.mouse.register_button(MouseButton::Left, PlayerMouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, PlayerMouseAction::UnlockMouse);
    }
}

impl Screen for Game {
    fn init(&mut self, graphics: &mut Graphics) {
        let ids = GameIds {
            cam_id: BufferId("main_camera"),
            env_id: BufferId("environment"),
            rtex_id: TextureId("render_texture"),
            voxel_bg_id: BindGroupId("voxel_bind_group"),
            voxel_pip_id: PipelineId("voxel_pipeline"),
            blit_bg_id: BindGroupId("blit_bind_group"),
            blit_pip_id: PipelineId("blit_pipeline")
        };

        let cam_buf_builder = BufferBuilder::as_uniform(BufferContents::Empty(128))
            .with_label("Camera Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.cam_id, &cam_buf_builder);

        let env_buf_builder = BufferBuilder::as_uniform(BufferContents::Empty(64))
            .with_label("Environment Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.env_id, &env_buf_builder);

        let tex_type = TextureType::Computed { 
            width: graphics.canvas.config.width, 
            height: graphics.canvas.config.height
        };
        let rtex_builder = TextureBuilder::new(tex_type)
            .with_label("Voxel Storage Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING);
        graphics.gpu.request_texture(&ids.rtex_id, &rtex_builder);

        let voxel_bg_builder = BindGroupBuilder::new()
            .with_label("Compute Bind Group")
            .with_buffer(ids.cam_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_buffer(ids.env_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_texture(ids.rtex_id.clone(), TextureRole::Storage, wgpu::ShaderStages::COMPUTE);
        graphics.gpu.request_bind_group(&ids.voxel_bg_id, &voxel_bg_builder);

        let voxel_pip_builder = ComputePipelineBuilder::new()
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[ids.voxel_bg_id])
            .with_shader(include_str!("../../shaders/ray_march.wgsl"));
        graphics.gpu.request_pipeline(&ids.voxel_pip_id, PipelineBuilder::Compute(voxel_pip_builder.clone()));
 
        let blit_bg_builder = BindGroupBuilder::new()
            .with_label("Blit Bind Group")
            .with_texture(ids.rtex_id.clone(), TextureRole::Sampled { filterable: true }, wgpu::ShaderStages::FRAGMENT);
        graphics.gpu.request_bind_group(&ids.blit_bg_id, &blit_bg_builder);

        let blit_pip_id = PipelineId("blit_pipeline");
        let blit_pip_builder = RenderPipelineBuilder::new()
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[ids.blit_bg_id])
            .with_shader(include_str!("../../shaders/blit.wgsl"))
            .with_target_format(graphics.canvas.config.format);
        graphics.gpu.request_pipeline(&blit_pip_id, PipelineBuilder::Render(blit_pip_builder.clone()));

        self.ids = Some(ids);
        self.init_input();
    }

    fn on_resize(&mut self, graphics: &mut Graphics) {
        let Some(ref ids) = self.ids else { return; };

        graphics.gpu.remove_texture(&ids.rtex_id);
        let tex_type = TextureType::Computed { 
            width: graphics.canvas.config.width, 
            height: graphics.canvas.config.height
        };
        let rtex_builder = TextureBuilder::new(tex_type)
            .with_label("Voxel Storage Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING);
        graphics.gpu.request_texture(&ids.rtex_id, &rtex_builder);

        graphics.gpu.remove_bind_group(&ids.voxel_bg_id);
        let voxel_bg_builder = BindGroupBuilder::new()
            .with_label("Compute Bind Group")
            .with_buffer(ids.cam_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_buffer(ids.env_id.clone(), BufferRole::Uniform, wgpu::ShaderStages::COMPUTE)
            .with_texture(ids.rtex_id.clone(), TextureRole::Storage, wgpu::ShaderStages::COMPUTE);
        graphics.gpu.request_bind_group(&ids.voxel_bg_id, &voxel_bg_builder);

        graphics.gpu.remove_bind_group(&ids.blit_bg_id);
        let blit_bg_builder = BindGroupBuilder::new()
            .with_label("Blit Bind Group")
            .with_texture(ids.rtex_id.clone(), TextureRole::Sampled { filterable: true }, wgpu::ShaderStages::FRAGMENT);
        graphics.gpu.request_bind_group(&ids.blit_bg_id, &blit_bg_builder);
    }

    fn input_event(&mut self, event: crate::InputEvent) {
        match event {
            InputEvent::Key(key_event) => {
                self.keyboard.key_event(key_event)
            },
            InputEvent::MouseButton { state, button } => {
                self.mouse.button_event(state, button);
            },
            InputEvent::MouseMotion { dx, dy } => {
                self.mouse.motion_event(dx, dy);
            }
        }
    }

    fn process_input(&mut self, graphics: &mut Graphics, dt: f32) -> ScreenTransition {
        for action in self.mouse.poll_on_press() {
            match action {
                PlayerMouseAction::LockMouse => graphics.canvas.set_cursor_lock(true),
                PlayerMouseAction::UnlockMouse => graphics.canvas.set_cursor_lock(false),
            }
        }
        
        for action in self.keyboard.peek_on_press() {
            match action {
                PlayerKeyAction::Exit => return ScreenTransition::Exit,
                _ => {}
            }
        }
        
        if graphics.canvas.is_cursor_locked {
            let dm = self.mouse.poll_motion();
            if dm.dx != 0.0 || dm.dy != 0.0 {
                self.controller.rotate_delta(&mut self.camera, dm.dx, dm.dy);
            }

            for action in self.keyboard.poll_on_held() {
                match action {
                    PlayerKeyAction::MoveForward => self.controller.move_forward(&mut self.camera, dt),
                    PlayerKeyAction::MoveBackward => self.controller.move_backward(&mut self.camera, dt),
                    PlayerKeyAction::StrafeLeft => self.controller.strafe_left(&mut self.camera, dt),
                    PlayerKeyAction::StrafeRight => self.controller.strafe_right(&mut self.camera, dt),
                    PlayerKeyAction::MoveUp => self.controller.move_up(&mut self.camera, dt),
                    PlayerKeyAction::MoveDown => self.controller.move_down(&mut self.camera, dt),
                    PlayerKeyAction::ResetCamera => {
                        self.camera.transform.move_to(self.default_cam_pos);
                        self.camera.transform.set_rotation(Quat::IDENTITY);
                        self.controller.reset_delta();
                    },
                    _ => {}
                }
            }
        }

        self.keyboard.clear_events();
        self.mouse.clear_events();

        ScreenTransition::None
    }

    fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        let Some(ref ids) = self.ids else { return; };
        self.world.update(dt);

        graphics.gpu.update_buffer(&ids.cam_id, BufferUpdate {
            data_struct: CameraUniform::build_from(&mut self.camera, graphics.canvas.aspect),
            offset: 0
        });

        graphics.gpu.update_buffer(&ids.env_id, BufferUpdate { 
            data_struct: self.world.calc_environment(),
            offset: 0
        });
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let Some(ref ids) = self.ids else { return Ok(()); };
        
        let wx = (graphics.canvas.config.width + 15) / 16;
        let wy = (graphics.canvas.config.height + 15) / 16;

        graphics.gpu.add_pass(GpuPass::Compute(
            ComputePass {
                pipeline_id: ids.voxel_pip_id,
                bind_groups: vec![ids.voxel_bg_id],
                work_groups: (wx, wy, 1)
            })
        );
        graphics.gpu.add_pass(GpuPass::Render(
            RenderPass { 
                pipeline_id: ids.blit_pip_id,
                bind_groups: vec![ids.blit_bg_id], 
                vertex_count: 3, 
                instance_count: 1 
            })
        );

        graphics.gpu.finish(&graphics.canvas)
    }
}