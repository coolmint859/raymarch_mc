use crate::{Graphics, InputEvent, WindowDriver, WindowState, game::{Game, PlayerKeyAction, QuadTest, TextTest}, utils::{InputHandler, NoAction}};

/// Represents transitions between screens
pub enum ScreenTransition {
    None,
    Game,
    QuadTest,
    TextTest,
    Exit,
}

/// Represents the set of known screens. 
/// 
/// Since this is an enum, it implies only one can be used at a time: the "active" screen.
pub enum ActiveScreen {
    Game {
        screen: Game,
        input: InputHandler<PlayerKeyAction, NoAction>
    },
    QuadTest {
        screen: QuadTest,
        input: InputHandler<NoAction, NoAction>
    },
    TextTest {
        screen: TextTest,
        input: InputHandler<NoAction, NoAction>
    }
}

/// Routes window events and driver logic to the active screen, as determined by user input
pub struct ScreenRouter {
    active_screen: ActiveScreen
}

impl ScreenRouter {
    pub fn new (init_screen: ActiveScreen) -> Self {
        Self { active_screen: init_screen }
    }
}

impl WindowDriver for ScreenRouter {
    fn init(&mut self, graphics: &mut Graphics) {
        match &mut self.active_screen {
            ActiveScreen::Game { screen, input } => {
                screen.init(graphics, input);
            },
            ActiveScreen::QuadTest { screen, input  } => {
                screen.init(graphics, input);
            },
            ActiveScreen::TextTest { screen, input } => {
                screen.init(graphics, input);
            }
        }
    }

    fn process_event(&mut self, event: InputEvent) {
        match &mut self.active_screen {
            ActiveScreen::Game { screen: _, input } => {
                input.process_event(event);
            },
            ActiveScreen::QuadTest { screen: _, input } => {
                input.process_event(event);
            },
            ActiveScreen::TextTest { screen: _, input } => {
                input.process_event(event);
            }
        }
    }

    fn process_input(&mut self, graphics: &mut Graphics, dt: f32) -> WindowState {
        let transition = match &mut self.active_screen {
            ActiveScreen::Game { screen, input } => {
                let t = screen.process_input(graphics, input, dt);
                input.clear_events();

                t
            },
            ActiveScreen::QuadTest { screen, input } => {
                let t = screen.process_input(graphics, input, dt);
                input.clear_events();

                t
            },
            ActiveScreen::TextTest { screen, input } => {
                let t = screen.process_input(graphics, input, dt);
                input.clear_events();

                t
            }
        };

        match transition {
            ScreenTransition::None => WindowState::Continue,
            ScreenTransition::Game => {
                let mut game = Game::new();
                let mut input = InputHandler::new();

                game.init(graphics, &mut input);
                self.active_screen = ActiveScreen::Game { 
                    screen: game, 
                    input, 
                };
                WindowState::Continue
            },
            ScreenTransition::QuadTest => {
                let mut quad_test = QuadTest::new();
                let mut input = InputHandler::new();

                quad_test.init(graphics,&mut input);
                self.active_screen = ActiveScreen::QuadTest { 
                    screen: quad_test, 
                    input
                };
                WindowState::Continue
            },
            ScreenTransition::TextTest => {
                let mut text_test = TextTest::new();
                let mut input = InputHandler::new();

                text_test.init(graphics,&mut input);
                self.active_screen = ActiveScreen::TextTest { 
                    screen: text_test, 
                    input
                };
                WindowState::Continue
            }
            ScreenTransition::Exit => WindowState::Exit
        }
    }

    fn on_resize(&mut self, graphics: &mut Graphics) {
        match &mut self.active_screen {
            ActiveScreen::Game { screen, input: _ } => {
                screen.on_resize(graphics);
            },
            ActiveScreen::QuadTest { screen, input: _ } => {
                screen.on_resize(graphics)
            },
            ActiveScreen::TextTest { screen, input: _ } => {
                screen.on_resize(graphics);
            }
        }
    }

    fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        match &mut self.active_screen {
            ActiveScreen::Game { screen, input: _ } => {
                screen.update(graphics, dt);
            },
            ActiveScreen::QuadTest { screen, input: _ } => {
                screen.update(graphics, dt);
            },
            ActiveScreen::TextTest { screen, input: _ } => {
                screen.update(graphics, dt);
            }
        }
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        match &mut self.active_screen {
            ActiveScreen::Game { screen, input: _ } => {
                screen.render(graphics)
            },
            ActiveScreen::QuadTest { screen, input: _ } => {
                screen.render(graphics)
            },
            ActiveScreen::TextTest { screen, input: _ } => {
                screen.render(graphics)
            }
        }
    }
}