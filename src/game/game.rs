use std::format;

use glam::{Quat, Vec3, vec3};
use winit::keyboard::KeyCode;

use crate::{
    Graphics, game::{ScreenTransition, VoxelRenderer, VoxelWorld}, graphics::*, utils::{Camera, CameraSpace, CharacterRange, Controllable, EntityController, FontManager, FontReader, InputHandler, NoAction, RelativePerspective, TextOptions, Transform, font::font::FontId},
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
}

pub struct Game {
    controller: EntityController,
    camera: Camera<RelativePerspective>,

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

            arial: FontId::UNINIT,
            fonts: FontManager::new(),
            show_fps: true,

            default_cam_pos,
            world: VoxelWorld::new(),
            renderer: VoxelRenderer::new(),
        }
    }

    pub fn init_input(&mut self, input: &mut InputHandler<PlayerKeyAction, NoAction>) {
        input.keyboard.register_key(KeyCode::KeyW, PlayerKeyAction::MoveForward);
        input.keyboard.register_key(KeyCode::KeyA, PlayerKeyAction::StrafeLeft);
        input.keyboard.register_key(KeyCode::KeyS, PlayerKeyAction::MoveBackward);
        input.keyboard.register_key(KeyCode::KeyD, PlayerKeyAction::StrafeRight);
        input.keyboard.register_key(KeyCode::ShiftLeft, PlayerKeyAction::MoveUp);
        input.keyboard.register_key(KeyCode::Space, PlayerKeyAction::MoveDown);
        input.keyboard.register_key(KeyCode::KeyR, PlayerKeyAction::ResetCamera);
        input.keyboard.register_key(KeyCode::KeyT, PlayerKeyAction::ToggleSimulation);
        input.keyboard.register_key(KeyCode::KeyN, PlayerKeyAction::StepSimulation);
        input.keyboard.register_key(KeyCode::KeyF, PlayerKeyAction::ShowFPS);
    }

    pub fn init(&mut self, graphics: &mut Graphics, input: &mut InputHandler<PlayerKeyAction, NoAction>) {
        self.camera.init(graphics);
        self.renderer.init(graphics, &self.world, &self.camera);

        self.arial = self.fonts.request_font(
            "./assets/arial.ttf", 
            FontReader::as_sdf(16.0, CharacterRange::default())
        );

        self.world.toggle_pause();
        self.init_input(input);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        self.renderer.on_resize(graphics);
    }

    pub fn process_input(&mut self, graphics: &mut Graphics, input: &mut InputHandler<PlayerKeyAction, NoAction>, dt: f32) -> ScreenTransition {
        if graphics.canvas.cursor_locked() {
            let dm = input.mouse.poll_motion();
            if dm.dx != 0.0 || dm.dy != 0.0 {
                self.controller.rotate_delta(&mut self.camera, dm.dx, dm.dy);
            }

            for action in input.keyboard.poll_on_held() {
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

            for action in input.keyboard.poll_on_press() {
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

        ScreenTransition::None
    }

    pub fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        self.world.update(dt, false);
        self.camera.update(graphics, dt);

        let cam_vp = self.camera.get_space().view_proj_mat(graphics);
        self.renderer.update_cam_motion(graphics, cam_vp);

        let screen_height = graphics.canvas.height() as f32;

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

    pub fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
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