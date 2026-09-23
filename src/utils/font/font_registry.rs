use std::collections::HashMap;

use crate::{graphics::{GpuContext, ResourceHandler, Task}, utils::{Camera, CameraSpace, FontReadResult, FontReaderType, font_asset::{Font, FontAssets, FontId}}};

/// the max number of rendered characters per font
pub const CHAR_LIMIT: u64 = 500;

/// Converts the result of font readers into gpu assets and stores them for future use
pub(crate) struct FontRegistry {
    /// Contains the set of currently parsing fonts
    pending_fonts: ResourceHandler<FontId, FontReadResult>,
    /// Contains the set of fonts ready to be used
    ready_fonts: HashMap<FontId, Font>,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self {
            pending_fonts: ResourceHandler::new(),
            ready_fonts: HashMap::new(),
        }
    }

    /// request a new font to be created from the provided reader
    pub fn request_font(&mut self, font_id: &FontId, reader: impl FontReaderType) {
        if self.pending_fonts.contains(font_id) || self.ready_fonts.contains_key(font_id) { return; }

        let path_copy = font_id.path.clone();
        let font_parse_task = Task::cpu_bound(async move {
            reader.parse(&path_copy)
        });

        self.pending_fonts.request_new(font_id, font_parse_task);
    }

    /// Syncs the registry with the main thread, converting any completed fonts into their asset representation
    pub fn sync<S: CameraSpace>(&mut self, camera: &Camera<S>, context: &mut GpuContext) {
        self.pending_fonts.sync();

        let ids: Vec<FontId> = self.pending_fonts.keys()
            .into_iter()
            .map(|key| key.clone())
            .collect();

        for font_id in ids {
            if let Some(raw_font) = self.pending_fonts.remove(&font_id) {
                let font_asset = FontAssets::new(font_id.clone());
                font_asset.create_assets(
                    camera,
                    (raw_font.atlas_data, raw_font.atlas_size), 
                    context
                );

                let font = Font {
                    assets: font_asset.clone(),
                    glyph_map: raw_font.glyphs,
                    line_height: raw_font.line_height,
                    scale: raw_font.scale,
                };

                self.ready_fonts.insert(font_id.clone(), font);
            }
        }
    }

    /// Returns the font asset associated with the provided font id, if available.
    pub fn get_font_asset(&mut self, font_id: &FontId) -> Option<&Font> {
        return self.ready_fonts.get(&font_id)
    }
}