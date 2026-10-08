use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    Graphics, InputEvent, game::{Screen, ScreenTransition, VoxelRenderer, VoxelWorld}, graphics::*, utils::{Camera, Controllable, EntityController, KeyboardHandler, MouseHandler, RelativePerspective},
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
    controller: EntityController,
    camera: Camera<RelativePerspective>,
    keyboard: KeyboardHandler<PlayerKeyAction>,
    mouse: MouseHandler<PlayerMouseAction>,

    default_cam_pos: Vec3,
    world: VoxelWorld,
    renderer: Option<VoxelRenderer>
}

impl Game {
    pub fn new() -> Self {
        let default_cam_pos = glam::vec3(16.0, 20.0, 16.0);

        let mut camera = Camera::new(RelativePerspective::default());
        camera.transform_mut().move_to(default_cam_pos);

        Self {
            camera,
            controller: EntityController::new(10.0, 0.003),
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            default_cam_pos,
            world: VoxelWorld::new(),
            renderer: None,
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
        self.camera.init(graphics);

        let renderer = VoxelRenderer::init(
            graphics, 
            &self.world, 
        *self.camera.buf_id()
        );
        self.renderer = Some(renderer);

        self.world.toggle_pause();
        self.init_input();
    }

    fn on_resize(&mut self, graphics: &mut Graphics) {
        if let Some(renderer) = &mut self.renderer {
            renderer.on_resize(graphics);
        }
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
        
        if graphics.canvas.cursor_locked() {
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
                        self.camera.transform_mut().move_to(self.default_cam_pos);
                        self.camera.transform_mut().set_rotation(Quat::IDENTITY);
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
        self.camera.update(graphics, dt);

        if let Some(renderer) = &self.renderer {
            let env_buffer_id = renderer.resources.world.env;

            let _ = graphics.context.update_buffer(&env_buffer_id, StructuredUpdate { 
                data: &self.world.env_uniform(),
                offset: 0
            });
        }
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        if let Some(renderer) = &mut self.renderer {
            let mut executor = MultiBufferExecutor::new();
            renderer.record(
                &mut executor, 
                RenderingState { 
                    output_view: frame.view.clone(), 
                    clear_color: Some(wgpu::Color::BLACK) 
                }, 
                graphics.canvas.dimensions()
            );

            executor.record_and_submit(&graphics.context);
        };

        frame.present();

        Ok(())
    }
}