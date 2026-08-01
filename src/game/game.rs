use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    Graphics, InputEvent, game::{BlitPass, GlobalResources, RayMarchPass, Screen, ScreenTransition, TaaPass, VoxelWorld}, graphics::*, utils::{CameraController, KeyboardHandler, MouseHandler, PerspectiveCamera},
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

pub struct Game {
    controller: CameraController,
    camera: PerspectiveCamera,
    keyboard: KeyboardHandler<PlayerKeyAction>,
    mouse: MouseHandler<PlayerMouseAction>,

    default_cam_pos: Vec3,
    world: VoxelWorld,
    globals: GlobalResources,

    rm_pass: RayMarchPass,
    taa_pass: TaaPass,
    blit_pass: BlitPass,
}

impl Game {
    pub fn new() -> Self {
        let default_cam_pos = glam::vec3(16.0, 20.0, 16.0);
        let mut camera = PerspectiveCamera::new();
        camera.transform.move_to(default_cam_pos);

        let globals = GlobalResources::new();

        Self {
            camera,
            controller: CameraController::new(10.0, 0.003),
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            default_cam_pos,
            world: VoxelWorld::new(),
            rm_pass: RayMarchPass::new(globals.ids.clone()),
            taa_pass: TaaPass::new(globals.ids.clone()),
            blit_pass: BlitPass::new(globals.ids.clone()),
            globals,
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
        self.camera.update(graphics.canvas.aspect);

        self.globals.init(graphics, &self.camera);
        self.rm_pass.init(&self.world, graphics);
        self.taa_pass.init(graphics);
        self.blit_pass.init(graphics);

        self.world.toggle_pause();
        self.init_input();
    }

    fn on_resize(&mut self, graphics: &mut Graphics) {
        self.globals.on_resize(graphics);
        self.rm_pass.on_resize(graphics);
        self.taa_pass.on_resize(graphics);
        self.blit_pass.on_resize(graphics);
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
        self.world.update(dt, false);
        self.camera.update(graphics.canvas.aspect);

        graphics.gpu.update_buffer(&self.globals.ids.cam_id, StructuredUpdate {
            data: &self.camera.to_uniform(graphics.frame),
        });

        graphics.gpu.update_buffer(&self.globals.ids.env_id, StructuredUpdate { 
            data: &self.world.env_uniform(),
        });
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let wx = (graphics.canvas.config.width + 15) / 16;
        let wy = (graphics.canvas.config.height + 15) / 16;

        graphics.gpu.add_command(self.rm_pass.get(wx, wy));
        graphics.gpu.add_command(self.taa_pass.get(wx, wy));
        graphics.gpu.add_command(self.blit_pass.get());
        graphics.gpu.finish(&graphics.canvas)
    }
}