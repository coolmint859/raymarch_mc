use std::{collections::{HashMap, HashSet}, println};

use glam::{Quat, Vec3};

use crate::{graphics::{CanvasFrame, DrawCommand, GpuContext, RawBytesUpdate, SequentialExecutor}, utils::{Camera, CameraSpace, FontReaderType, Transform, font_asset::{CharInstance, FontId, Quad}, font_registry::{CHAR_LIMIT, FontRegistry}}};

/// Options for text display
pub struct TextOptions {
    /// the position of the text (top left corner)
    pub transform: Transform,
    /// the height (size) of the text in NDC
    pub height: f32,
}

/// Stages text to be rendered with specific fonts with specific shaders
pub struct TextRenderer {
    /// Handles font parsing and asset loading / storage,
    font_registry: FontRegistry,
    /// The set of active fonts (fonts with staged text)
    active_fonts: HashSet<FontId>,

    /// The geometry a character is rendered on
    char_geometry: Quad,
    /// The character instances of a font yet to be rendered
    instances: HashMap<FontId, Vec<CharInstance>>,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            char_geometry: Quad::new(),
            font_registry: FontRegistry::new(),
            instances: HashMap::new(),
            active_fonts: HashSet::new(),
        }
    }

    /// Registers a new font. If the font already exists, it's matching handle is returned.
    pub fn request_font(&mut self, font_path: &str, reader: impl FontReaderType) -> FontId {
        let font_id = FontId { path: font_path.to_string(), pip: reader.font_pip() };
        if !self.instances.contains_key(&font_id) {
            self.instances.insert(font_id.clone(), Vec::with_capacity(CHAR_LIMIT as usize));
        }
        
        self.font_registry.request_font(&font_id, reader);
        return font_id;
    }

    /// State text to be rendered with the provided font and options.
    /// 
    /// This does not render the text, it only prepares it to be. After staging text, call record() for it to be added to a command executor.
    pub fn stage_text(&mut self, font_id: &FontId, text: &str, options: TextOptions) {
        if let Some(font) = self.font_registry.get_font_asset(font_id) {
            // println!("font id: {:?}", font.assets.font_id);
            self.active_fonts.insert(font_id.clone());

            let world_mat = options.transform.to_updated();
            let size = options.height / (font.line_height * font.scale);
     
            let mut cursor = Vec3::ZERO;
            for character in text.chars() {
                if character == '\n' {
                    cursor.x = 0.0;
                    cursor.y -= font.line_height * size;

                    continue;
                }

                if let Some(glyph) = font.glyph_map.get(&character) {
                    if character == ' ' {
                        cursor.x += glyph.advance * size;
                        continue;
                    }

                    let x_scale = glyph.plane_bounds.z * size;
                    let y_scale = glyph.plane_bounds.w * size;

                    let x_pos = cursor.x + (glyph.plane_bounds.x * size) + (x_scale * 0.5);
                    let y_pos = cursor.y + (glyph.plane_bounds.y * size) + (y_scale * 0.5);

                    let local_transform = Transform::new(
                        Vec3::new(x_pos, y_pos, cursor.z), 
                        Quat::IDENTITY, 
                        Vec3::new(x_scale, y_scale, 1.0)
                    );

                    let final_matrix = world_mat * local_transform.to_updated();

                    let instance = CharInstance {
                        transform: final_matrix.to_cols_array(),
                        bounds: glyph.uv_bounds.to_array()
                    };

                    if let Some(instances) = self.instances.get_mut(font_id) {
                        instances.push(instance);
                    }

                    cursor.x += glyph.advance * size;
                }
            }
        }
    }

    /// sync the font resources with the main thread.
    pub fn sync<S: CameraSpace>(&mut self, camera: &Camera<S>, context: &mut GpuContext) {
        self.char_geometry.request_buffers(context);
        self.font_registry.sync(camera, context);

        for font_id in &self.active_fonts {
            if let (Some(chars), Some(font)) = (
                self.instances.get_mut(font_id),
                self.font_registry.get_font_asset(font_id)
            ) {
                let _ = context.update_buffer(&font.assets.cbuffer_id, RawBytesUpdate {
                    data: bytemuck::cast_slice(chars),
                    offset: 0,
                });
            }
        }
    }

    /// Records draw commands for any previously staged text to the provided executor.
    pub fn record(&mut self, frame: &CanvasFrame, executor: &mut impl SequentialExecutor) {
        // println!("Active fonts: {}", self.active_fonts.len());
        for font_id in &std::mem::take(&mut self.active_fonts) {
            if let (Some(chars), Some(font)) = (
                self.instances.get_mut(font_id),
                self.font_registry.get_font_asset(font_id)
            ) {
                let instance_count = chars.len() as u32;

                let draw_text = DrawCommand::new(font_id.pip.id.clone(), frame.view.clone(), 0..6)
                    .with_bind_groups(&[font.assets.bg.id])
                    .with_vertex_buffers(&[self.char_geometry.vbuffer_id, font.assets.cbuffer_id])
                    .with_index_buffer(self.char_geometry.ibuffer_id, wgpu::IndexFormat::Uint16)
                    .with_instances(0, instance_count);

                executor.add_command(draw_text);

                chars.clear();
            }
        }
    }
}