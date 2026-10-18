use std::format;

use glam::{Quat, Vec3, vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    Graphics, InputEvent, game::{Screen, ScreenTransition, VoxelRenderer, VoxelWorld}, graphics::*, utils::{Camera, Controllable, EntityController, FontManager, FontReader, KeyboardHandler, MouseHandler, RelativePerspective, TextOptions, Transform, font::font::FontId},
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
    ToggleSimulation,
    StepSimulation,
    ShowFPS,
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

    arial: FontId,
    fonts: FontManager,
    show_fps: bool,

    default_cam_pos: Vec3,
    world: VoxelWorld,
    renderer: VoxelRenderer
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

            arial: FontId::UNINIT,
            fonts: FontManager::new(),
            show_fps: true,

            default_cam_pos,
            world: VoxelWorld::new(),
            renderer: VoxelRenderer::new(),
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
        self.keyboard.register_key(KeyCode::KeyT, PlayerKeyAction::ToggleSimulation);
        self.keyboard.register_key(KeyCode::KeyN, PlayerKeyAction::StepSimulation);
        self.keyboard.register_key(KeyCode::KeyF, PlayerKeyAction::ShowFPS);

        self.mouse.register_button(MouseButton::Left, PlayerMouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, PlayerMouseAction::UnlockMouse);
    }
}

impl Screen for Game {
    fn init(&mut self, graphics: &mut Graphics) {
        self.camera.init(graphics);
        self.renderer.init(graphics, &self.world, *self.camera.buf_id());

        self.arial = self.fonts.request_font(
            "./assets/arial.ttf", 
            FontReader::as_sdf(16.0)
        );

        self.world.toggle_pause();
        self.init_input();
    }

    fn on_resize(&mut self, graphics: &mut Graphics) {
        self.renderer.on_resize(graphics);
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
                    PlayerKeyAction::ToggleSimulation => self.world.toggle_pause(),
                    PlayerKeyAction::StepSimulation => self.world.update(dt, true),
                    PlayerKeyAction::ShowFPS => self.show_fps = !self.show_fps,
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

        let screen_height = graphics.canvas.dimensions().1 as f32;

        if self.show_fps {
            self.fonts.stage_text(
                &self.arial, 
                &format!("FPS: {:.2}", 1.0 / dt), 
                TextOptions {
                    transform: Transform::from_position(vec3(0.05, 0.95, 0.0)),
                    height: 40.0 / screen_height
                }
            );
        }

        let show_fps_str = "Show FPS (F)";
        let toggle_sim_str = "Toggle Simulation: (T)";
        let step_sim = "Step Simulation: (N)";

        self.fonts.stage_text(
            &self.arial, 
            &format!("{}\n{}\n{}", show_fps_str, toggle_sim_str, step_sim), 
            TextOptions {
                transform: Transform::from_position(vec3(0.05, 0.1, 0.0)),
                height: 25.0 / screen_height
            }
        );

        self.renderer.update_environment(graphics, StructuredUpdate { 
            data: &self.world.env_uniform(),
            offset: 0
        });

        self.fonts.update(graphics);
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        let mut executor = MultiBufferExecutor::new();
        self.renderer.record(
            &mut executor, 
            RenderingState { 
                output_view: frame.view.clone(), 
                clear_color: Some(wgpu::Color::BLACK) 
            }, 
            graphics.canvas.dimensions()
        );

        self.fonts.record(
            RenderingState {
                output_view: frame.view.clone(),
                clear_color: None
            }, 
            &mut executor
        );

        executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}