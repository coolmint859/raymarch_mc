use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::MultiBufferExecutor, utils::{Camera, FontReader, MouseHandler, ScreenSpace, TextOptions, TextRenderer, Transform, font_asset::FontId}};

pub struct QuadTest {
    mouse: MouseHandler<PlayerMouseAction>,
    executor: MultiBufferExecutor,
    text_renderer: TextRenderer,
    arial: Option<FontId>,
    mono: Option<FontId>,

    // controller: CameraController,
    camera: Camera<ScreenSpace>,
}

impl QuadTest {
    pub fn new() -> Self {
        Self { 
            mouse: MouseHandler::new(),
            arial: None,
            mono: None,
            text_renderer: TextRenderer::new(),
            executor: MultiBufferExecutor::new(),
            // controller: CameraController::new(0.5, 0.003),
            camera: Camera::new(ScreenSpace),
        }
    }

    pub fn init_input(&mut self) {
        self.mouse.register_button(MouseButton::Left, PlayerMouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, PlayerMouseAction::UnlockMouse);
    }
}

impl Screen for QuadTest {
    fn init(&mut self, graphics: &mut Graphics) {
        self.init_input();

        // self.arial = Some(self.text_renderer.request_font(
        //     "./assets/arial.ttf", 
        //     FontReader::as_sdf(16.0)
        // ));

        self.mono = Some(self.text_renderer.request_font(
            "./assets/Monopack.ttf", 
            FontReader::as_sdf(16.0)
        ));

        self.camera.init(graphics);
    }

    fn input_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::MouseButton { state, button } => {
                self.mouse.button_event(state, button);
            },
            InputEvent::MouseMotion { dx, dy } => {
                self.mouse.motion_event(dx, dy);
            },
            _ => {}
        }
    }

    fn process_input(&mut self, graphics: &mut Graphics, _dt: f32) -> ScreenTransition {
        for action in self.mouse.poll_on_press() {
            match action {
                PlayerMouseAction::LockMouse => graphics.canvas.set_cursor_lock(true),
                PlayerMouseAction::UnlockMouse => graphics.canvas.set_cursor_lock(false),
            }
        }
        self.mouse.clear_events();

        ScreenTransition::None
    }

    fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        self.camera.update(graphics, dt);

        if let Some(monopack) = &self.mono {
            self.text_renderer.stage_text(
                monopack, 
                &format!("fps: {:.2}", 1.0 / dt), 
                TextOptions { 
                    transform: Transform::default().with_position(glam::vec3(0.05, 0.95, 0.0)),
                    height: 40.0 / graphics.canvas.dimensions().1 as f32
                }
            );
        }

        // if let Some(arial) = &self.arial {
        //     self.text_renderer.stage_text(
        //         arial, 
        //         "Hello there.", 
        //         TextOptions { 
        //             transform: Transform::default().with_position(glam::vec3(0.1, 0.1, 0.0)),
        //             height: 100.0 / graphics.canvas.dimensions().1 as f32
        //         }
        //     );
        // }

        self.text_renderer.sync(&self.camera, &mut graphics.context);
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        self.text_renderer.record(&frame, &mut self.executor);
        self.executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}