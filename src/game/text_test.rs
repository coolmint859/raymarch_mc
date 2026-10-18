use glam::vec3;
use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::{MultiBufferExecutor, RenderingState}, utils::{Camera, CharacterRange, FontManager, FontReader, MouseHandler, ScreenSpace, TextOptions, Transform, font::font::FontId}};

pub struct TextTest {
    mouse: MouseHandler<PlayerMouseAction>,
    executor: MultiBufferExecutor,
    camera: Camera<ScreenSpace>,

    fonts: FontManager,
    arial_bm: FontId,
    arial_sdf: FontId,
    mono: FontId,
}

impl TextTest {
    pub fn new() -> Self {
        Self { 
            mouse: MouseHandler::new(),
            executor: MultiBufferExecutor::new(),
            camera: Camera::new(ScreenSpace),

            fonts: FontManager::new(),
            arial_bm: FontId::UNINIT,
            arial_sdf: FontId::UNINIT,
            mono: FontId::UNINIT,
        }
    }

    pub fn init_input(&mut self) {
        self.mouse.register_button(MouseButton::Left, PlayerMouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, PlayerMouseAction::UnlockMouse);
    }
}

impl Screen for TextTest {
    fn init(&mut self, graphics: &mut Graphics) {
        self.init_input();
        self.camera.init(graphics);

        self.arial_sdf = self.fonts.request_font(
            "./assets/arial.ttf", 
            FontReader::as_sdf(16.0, CharacterRange::default())
        );

        self.arial_bm = self.fonts.request_font(
            "./assets/arial.ttf", 
            FontReader::as_bitmap(CharacterRange::default())
        );

        self.mono = self.fonts.request_font(
            "./assets/Monopack.ttf", 
            // FontReader::as_sdf(16.0)
            FontReader::as_sdf(16.0, CharacterRange::default())
        );
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
        self.camera.update(graphics, 0.0);
        let screen_height = graphics.canvas.dimensions().1 as f32;

        // self.fonts.stage_text(
        //     &self.arial_sdf, 
        //     "A", 
        //     TextOptions {
        //         transform: Transform::from_position(vec3(0.5, 0.1, 0.0)),
        //         height: 1.0
        //     }
        // );

        self.fonts.stage_text(
            &self.mono, 
            &format!("(sdf) fps: {:.2}", 1.0 / dt), 
            TextOptions { 
                transform: Transform::from_position(vec3(0.05, 0.95, 0.0)),
                height: 40.0 / screen_height
            }
        );

        self.fonts.stage_text(
            &self.arial_bm, 
            "(bm) The quick brown fox jumps over the lazy dog.", 
            TextOptions { 
                transform: Transform::from_position(vec3(0.05, 0.55, 0.0)),
                height: 95.0 / screen_height
            }
        );

        self.fonts.stage_text(
            &self.arial_sdf, 
            "(sdf) The quick brown fox jumps over the lazy dog.", 
            TextOptions { 
                transform: Transform::from_position(vec3(0.05, 0.45, 0.0)),
                height: 95.0 / screen_height
            }
        );

        self.fonts.update(graphics);
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        self.fonts.record(
            RenderingState {
                output_view: frame.view.clone(),
                clear_color: Some(wgpu::Color::BLUE)
            }, 
            &mut self.executor
        );

        self.executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}