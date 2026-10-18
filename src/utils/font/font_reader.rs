use std::{collections::HashMap, fs::File, io::Read};

use crate::utils::{AtlasConfig, AtlasType, CharacterGlyph, generate_atlas};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct FontReaderId(pub &'static str);

/// The result of a font reader.
#[derive(Debug)]
pub struct FontReadResult {
    /// the path to the shader used to render fonts
    pub shader: String,
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
    fn font_shader(&self) -> String;
}

/// Reads in font files and creates a texture atlas and glyph map from them
pub struct FontReader<P> {
    /// The size of the atlas in pixels
    atlas_size: u32,
    /// The scale for the glyph rasterization in Em units
    scale: f32,
    /// A parser implementation
    parser: P,
}

/// A font reader type for bitmap fonts
pub struct BitMap;
impl FontReader<BitMap> {
    /// Create a font reader that outputs an bitmap texture atlas
    pub fn as_bitmap() -> Self {
        Self {
            atlas_size: 1024,
            scale: 96.0,
            parser: BitMap
        }
    }
}

impl FontReaderType for FontReader<BitMap> {
    fn id(&self) -> FontReaderId {
        FontReaderId("bitmap_reader")
    }

    fn font_shader(&self) -> String {
        return "./shaders/bitmap_font.wgsl".to_string()
    }

    fn parse(&self, path: &str) -> Result<FontReadResult, String> {
        let mut ttf = File::open(path).map_err(|e| e.to_string())?;
        let mut font_data = Vec::new();
        ttf.read_to_end(&mut font_data).map_err(|e| e.to_string())?;

        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        let line_metrics = font.horizontal_line_metrics(self.scale)
            .ok_or("[FontReader<SDF>] Failed to read font line metrics")?;
        let line_height = line_metrics.new_line_size / self.scale;

        let (glyphs, atlas_data) = generate_atlas(
            AtlasConfig {
                font,
                scale: self.scale,
                size: self.atlas_size,
                spacing: 2,
                ty: AtlasType::BitMap
            }
        );

        let read_result = FontReadResult {
            shader: self.font_shader(),
            glyphs,
            atlas_data,
            atlas_size: self.atlas_size,
            line_height,
            scale: self.scale
        };

        Ok(read_result)
    }
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

    fn font_shader(&self) -> String {
        return "./shaders/sdf_font.wgsl".to_string()
    }

    fn parse(&self, path: &str) -> Result<FontReadResult, String> {
        let mut ttf = File::open(path).map_err(|e| e.to_string())?;
        let mut font_data = Vec::new();
        ttf.read_to_end(&mut font_data).map_err(|e| e.to_string())?;

        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        let line_metrics = font.horizontal_line_metrics(self.scale)
            .ok_or("[FontReader<SDF>] Failed to read font line metrics")?;
        let line_height = line_metrics.new_line_size / self.scale;

        let (glyphs, atlas_data) = generate_atlas(
            AtlasConfig {
                font,
                scale: self.scale,
                size: self.atlas_size,
                spacing: 2,
                ty: AtlasType::SDF { radius: self.parser.radius }
            }
        );

        let read_result = FontReadResult {
            shader: self.font_shader(),
            glyphs,
            atlas_data,
            atlas_size: self.atlas_size,
            line_height,
            scale: self.scale
        };

        Ok(read_result)
    }
}

