use std::{collections::HashMap, fs::File, io::Read};

use crate::{graphics::PipelineId, utils::gen_font_atlas};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct FontReaderId(pub &'static str);

/// Data struct holding information about character glyphs for use during rendering
#[derive(Clone, Debug)]
pub struct CharacterGlyph {
    /// The bounding box of the glyph in the atlas
    pub uv_bounds: glam::Vec4,
    /// The layout bounds of the character relative to the cursor
    pub plane_bounds: glam::Vec4,
    /// How far to advance the cursor after placing this character
    pub advance: f32,
}

/// The result of a font reader.
#[derive(Debug)]
pub struct FontReadResult {
    /// The raw byte data for the parsed font atlas
    pub atlas_data: Vec<u8>,
    /// the width/height of the font atlas
    pub atlas_size: u32,
    /// the map of characters to their glyph metrics
    pub glyphs: HashMap<char, CharacterGlyph>,
    /// the height of a line of text rendered with this font
    pub line_height: f32,
    /// the scale that the font was rasterized in
    pub scale: f32,
}

/// Represents a type of font reader, providing the core parsing method and rendering information
pub trait FontReaderType: Send + 'static {
    /// Parse the font into the cpu-side font result
    fn parse(&self, path: &str) -> Result<FontReadResult, String>;
    /// Get the unique identifier for this font reader.
    /// 
    /// This enables the ability to use the same pipeline for any font parsed with this reader.
    fn id(&self) -> FontReaderId;

    /// Get the pipeline information best used to render the font
    fn font_pip(&self) -> FontPipeline;
}

/// Represents the pipeline used to render a font
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontPipeline {
    pub id: PipelineId,
    pub shader: String,
}

/// Reads in font files and creates a texture atlas and glyph map from them
pub struct FontReader<P> {
    atlas_size: u32,
    scale: f32,
    parser: P,
}

/// A font reader type for sdf rendered fonts
pub struct SDF {
    /// the distance between extreme values in the sdf atlas
    pub radius: f32,
}

impl FontReader<SDF> {
    /// Create a font reader that outputs an sdf texture atlas
    pub fn as_sdf(radius: f32) -> Self {
        Self {
            atlas_size: 1024,
            scale: 96.0,
            parser: SDF { radius }
        }
    }
}

impl FontReaderType for FontReader<SDF> {
    fn id(&self) -> FontReaderId {
        FontReaderId("sdf_reader")
    }

    fn font_pip(&self) -> FontPipeline {
        FontPipeline {
            id: PipelineId("sdf_font_pipeline"),
            shader: "./shaders/sdf_font.wgsl".to_string()
        }
    }

    fn parse(&self, path: &str) -> Result<FontReadResult, String> {
        let mut ttf = File::open(path).map_err(|e| e.to_string())?;
        let mut font_data = Vec::new();
        ttf.read_to_end(&mut font_data).map_err(|e| e.to_string())?;

        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        let line_metrics = font.horizontal_line_metrics(self.scale)
            .ok_or("[FontReader<SDF>] Failed to read font line metrics")?;
        let line_height = line_metrics.new_line_size / self.scale;

        let (glyphs, atlas_data) = gen_font_atlas(
            font, 
            self.atlas_size, 
            self.scale, 
            self.parser.radius
        );

        let read_result = FontReadResult {
            glyphs,
            atlas_data,
            atlas_size: self.atlas_size,
            line_height,
            scale: self.scale
        };

        Ok(read_result)
    }
}

