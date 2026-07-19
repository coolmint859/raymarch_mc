use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    Graphics, InputEvent, game::{Screen, ScreenTransition, VoxelPalette, VoxelWorld}, graphics::*, utils::{CameraController, KeyboardHandler, MouseHandler, PerspectiveCamera},
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
    PauseSimulation,
    StepSimulation,
    Exit,
}

#[derive(Clone, Copy)]
pub enum PlayerMouseAction {
    LockMouse,
    UnlockMouse,
}

struct GameIds {
    pub cam_id: BufferId,
    pub env_id: BufferId,
    pub vox_id: BufferId,
    pub reg_id: BufferId,
    pub pal_id: BufferId,
    pub rtex_id: TextureId,

    pub voxel_bg_id: BindGroupId,
    pub blit_bg_id: BindGroupId,

    pub voxel_pip_id: PipelineId,
    pub blit_pip_id: PipelineId,
}

pub struct Game {
    controller: CameraController,
    camera: PerspectiveCamera,
    keyboard: KeyboardHandler<PlayerKeyAction>,
    mouse: MouseHandler<PlayerMouseAction>,

    default_cam_pos: Vec3,
    world: VoxelWorld,

    ids: Option<GameIds>,
}

impl Game {
    pub fn new() -> Self {
        let default_cam_pos = glam::vec3(16.0, 20.0, 16.0);
        let mut camera = PerspectiveCamera::new();
        camera.transform.move_to(default_cam_pos);

        Self {
            camera,
            controller: CameraController::new(10.0, 0.003),
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            default_cam_pos,
            world: VoxelWorld::new(),
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
        self.keyboard.register_key(KeyCode::KeyP, PlayerKeyAction::PauseSimulation);
        self.keyboard.register_key(KeyCode::KeyN, PlayerKeyAction::StepSimulation);

        self.mouse.register_button(MouseButton::Left, PlayerMouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, PlayerMouseAction::UnlockMouse);
    }
}

impl Screen for Game {
    fn init(&mut self, graphics: &mut Graphics) {
        let ids = GameIds {
            cam_id: BufferId("main_camera"),
            env_id: BufferId("environment"),
            vox_id: BufferId("voxels"),
            reg_id: BufferId("Region"),
            pal_id: BufferId("palette"),
            rtex_id: TextureId("render_texture"),
            voxel_bg_id: BindGroupId("voxel_bind_group"),
            voxel_pip_id: PipelineId("voxel_pipeline"),
            blit_bg_id: BindGroupId("blit_bind_group"),
            blit_pip_id: PipelineId("blit_pipeline")
        };

        self.camera.update(graphics.canvas.aspect);
        let camera_data = self.camera.to_uniform().to_bytes().to_vec();
        let camera_buffer = Buffer::as_uniform(BufferContents::WithData(camera_data))
            .with_label("Camera Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.cam_id, camera_buffer);

        let env_data = self.world.env_uniform().to_bytes().to_vec();
        let env_buffer = Buffer::as_uniform(BufferContents::WithData(env_data))
            .with_label("Environment Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.env_id, env_buffer);

        let palette_data = VoxelPalette::create().colors;
        let palette_buffer = Buffer::as_uniform(BufferContents::WithData(palette_data))
            .with_label("Palette Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.pal_id, palette_buffer);

        let voxel_data = self.world.voxel_data();
        let voxel_buffer = Buffer::as_storage(BufferContents::WithData(voxel_data))
            .with_label("Voxel Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.vox_id, voxel_buffer);

        let region_data = self.world.region_data();
        let region_buffer = Buffer::as_storage(BufferContents::WithData(region_data))
            .with_label("Region Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&ids.reg_id, region_buffer);

        let render_texture = Texture::new(TextureType::Computed)
            .with_label("Voxel Storage Texture")
            .with_size_2d(graphics.canvas.config.width, graphics.canvas.config.height)
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING);
        graphics.gpu.request_texture(&ids.rtex_id, render_texture);

        let raymarch_bind_group = BindGroup::new()
            .with_label("Compute Bind Group")
            .with_entry(BufferBinding::as_uniform(ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(ids.env_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(ids.pal_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(ids.reg_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(ids.rtex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&ids.voxel_bg_id, &raymarch_bind_group);

        let raymarch_pipeline = Pipeline::new(PipelineType::Compute(ComputePipelineType::default()))
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[ids.voxel_bg_id])
            .with_shader("./shaders/ray_march.wgsl");
        graphics.gpu.request_pipeline(&ids.voxel_pip_id, &raymarch_pipeline);
 
        let blit_bind_group = BindGroup::new()
            .with_label("Blit Bind Group")
            .with_entry(TextureBinding::as_sampled(ids.rtex_id, TextureTypeSampled::default()));
        graphics.gpu.request_bind_group(&ids.blit_bg_id, &blit_bind_group);

        let blit_pipeline = Pipeline::new(PipelineType::Render(RenderPipelineType::default()))
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[ids.blit_bg_id])
            .with_shader("./shaders/blit.wgsl");
        graphics.gpu.request_pipeline(&ids.blit_pip_id, &blit_pipeline);

        self.ids = Some(ids);
        self.world.toggle_pause();
        self.init_input();
    }

    fn on_resize(&mut self, graphics: &mut Graphics) {
        let Some(ref ids) = self.ids else { return; };

        graphics.gpu.remove_texture(&ids.rtex_id);
        let render_texture = Texture::new(TextureType::Computed)
            .with_label("Voxel Storage Texture")
            .with_size_2d(graphics.canvas.config.width, graphics.canvas.config.height)
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING);
        graphics.gpu.request_texture(&ids.rtex_id, render_texture);

        graphics.gpu.remove_bind_group(&ids.voxel_bg_id);
        let raymarch_bind_group = BindGroup::new()
            .with_label("Compute Bind Group")
            .with_entry(BufferBinding::as_uniform(ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(ids.env_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(ids.pal_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(ids.reg_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(ids.rtex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&ids.voxel_bg_id, &raymarch_bind_group);

        graphics.gpu.remove_bind_group(&ids.blit_bg_id);
        let blit_bind_group = BindGroup::new()
            .with_label("Blit Bind Group")
            .with_entry(TextureBinding::as_sampled(ids.rtex_id, TextureTypeSampled::default()));
        graphics.gpu.request_bind_group(&ids.blit_bg_id, &blit_bind_group);
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
                    _ => {}
                }
            }

            // println!("cam pos: {:?}", self.camera.transform.get_position())

            for action in self.keyboard.poll_on_press() {
                match action {
                    PlayerKeyAction::PauseSimulation => self.world.toggle_pause(),
                    PlayerKeyAction::StepSimulation => self.world.update(dt, true),
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
        self.world.update(dt, false);
        self.camera.update(graphics.canvas.aspect);

        graphics.gpu.update_buffer(&ids.cam_id, StructuredUpdate {
            data: &self.camera.to_uniform(),
        });

        graphics.gpu.update_buffer(&ids.env_id, StructuredUpdate { 
            data: &self.world.env_uniform(),
        });
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let Some(ref ids) = self.ids else { return Ok(()); };
        
        let wx = (graphics.canvas.config.width + 15) / 16;
        let wy = (graphics.canvas.config.height + 15) / 16;

        graphics.gpu.add_pass(PassInfo::Compute(
            ComputePassInfo {
                pipeline_id: ids.voxel_pip_id,
                bind_groups: vec![ids.voxel_bg_id],
                work_groups: (wx, wy, 1)
            })
        );
        graphics.gpu.add_pass(PassInfo::Render(
            RenderPassInfo { 
                pipeline_id: ids.blit_pip_id,
                bind_groups: vec![ids.blit_bg_id], 
                vertex_count: 3, 
                instance_count: 1 
            })
        );

        graphics.gpu.finish(&graphics.canvas)
    }
}