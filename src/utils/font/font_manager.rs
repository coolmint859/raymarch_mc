use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::{graphics::{BufferId, DrawCommand, Graphics, RenderingState, SequentialExecutor}, utils::{Camera, FontReadResult, FontReaderType, ResourceHandler, ScreenSpace, Task, Transform, font::font::{Font, FontId, FontPrimitive}}};

/// Options for text display
pub struct TextOptions {
    /// the position of the text (top left corner)
    pub transform: Transform,
    /// the height (size) of the text in NDC
    pub height: f32,
}

/// Creates `Font` resources, and allows text to be staged for rendering using known `Font`s. 
/// 
/// Font resources are loaded asynchronously using an internal `ResourceHandler`
pub struct FontManager {
    /// The set of fonts currently parsing
    pending_fonts: ResourceHandler<FontId, FontReadResult>,
    /// The set of fonts ready to be used
    ready_fonts: HashMap<FontId, Font>,
    /// The set of active fonts (fonts with staged text)
    active_fonts: HashSet<FontId>,
    /// The id to the camera the `TextRenderer` renders with
    cam_buf_id: Option<BufferId>
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            pending_fonts: ResourceHandler::new(),
            ready_fonts: HashMap::new(),
            active_fonts: HashSet::new(),
            cam_buf_id: None
        }
    }

    /// Registers a new font. If the font already exists, it's matching handle is returned.
    pub fn request_font(&mut self, font_path: &str, reader: impl FontReaderType) -> FontId {
        let font_id = FontId(Uuid::new_v4());

        let path_copy = font_path.to_string().clone();
        let font_parse_task = Task::cpu_bound(async move {
            reader.parse(&path_copy)
        });

        self.pending_fonts.request_new(&font_id, font_parse_task);
        return font_id;
    }

    /// State text to be rendered with the provided font and options.
    /// 
    /// This does not render the text, it only prepares it to be. After staging text, call record() for a draw command to be added to a command executor.
    pub fn stage_text(&mut self, font_id: &FontId, text: &str, options: TextOptions) {
        if let Some(font) = self.ready_fonts.get_mut(font_id) {
            self.active_fonts.insert(font_id.clone());
            font.stage_text(text, options);
        }
    }

    /// update the `FontManager`, syncing `Font` resources with the main thread.
    pub fn update(&mut self, graphics: &mut Graphics) {
        self.pending_fonts.sync();

        let cam_buf_id = match self.cam_buf_id {
            Some(id) => id,
            None => self.init_camera(graphics),
        };

        let ids: Vec<FontId> = self.pending_fonts.keys()
            .into_iter()
            .map(|key| key.clone())
            .collect();

        for font_id in ids {
            if let Some(raw_font) = self.pending_fonts.remove(&font_id) {
                let font_primitive = FontPrimitive::init(
                    &cam_buf_id,
                    (raw_font.atlas_data, raw_font.atlas_size), 
                    &mut graphics.context,
                    &raw_font.shader
                );

                let font = Font {
                    primitive: font_primitive,
                    glyph_map: raw_font.glyphs,
                    line_height: raw_font.line_height,
                    scale: raw_font.scale,
                };

                self.ready_fonts.insert(font_id.clone(), font);
            }
        }

        for font_id in &self.active_fonts {
            if let Some(font) = self.ready_fonts.get_mut(font_id) {
                font.update(&mut graphics.context);
            }
        }
    }

    /// Records draw commands for any previously staged text to the provided executor.
    pub fn record(&mut self, state: RenderingState, executor: &mut impl SequentialExecutor) {
        let mut draw_cmd = DrawCommand::new(state);

        for font_id in &std::mem::take(&mut self.active_fonts) {
            if let Some(font) = self.ready_fonts.get_mut(font_id) {
                font.render(&mut draw_cmd);
            }
        }

        if draw_cmd.has_draws() {
            executor.add_command(draw_cmd);
        }
    }

    fn init_camera(&mut self, graphics: &mut Graphics) -> BufferId {
        let mut camera = Camera::new(ScreenSpace);
        camera.init(graphics);
        
        self.cam_buf_id = Some(*camera.buf_id());
        return *camera.buf_id()
    }
}