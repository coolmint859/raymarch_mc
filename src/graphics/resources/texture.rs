use std::{ops::Deref};

use image::GenericImageView;
use uuid::Uuid;
use wgpu::Extent3d;

use crate::graphics::{ResourceId, TextureId};

/// A lightweight handle to a gpu texture
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TextureHandle {
    /// The Handle to the actual wgpu `Texture`
    pub texture: wgpu::Texture,
    /// The view into the associated wgpu `Texture`
    pub view: wgpu::TextureView,
    /// The dimensions (width/height/depth) of the texture
    pub extent: Extent3d,
}

impl Deref for TextureHandle {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// Specifies the dimensions of a texture
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TexDimensions {
    pub width: u32,
    pub height: u32, 
    pub depth: u32,
    pub wgpu_dim: wgpu::TextureDimension
}

impl TexDimensions {
    /// A 2D texture
    pub fn size_2d(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
            wgpu_dim: wgpu::TextureDimension::D2
        }
    }

    /// A 3D texture
    pub fn size_3d(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
            wgpu_dim: wgpu::TextureDimension::D3
        }
    }
}

/// Represents sources for textures, condensing the blueprint into a payload for use during uploading.
pub trait TextureSource: Send + 'static {
    /// Converts `self` into the raw texture data and dimensions given a `format`, if possible.
    fn into_data(self, format: wgpu::TextureFormat) -> Result<(Option<Vec<u8>>, TexDimensions), String>;
}

/// A `TextureSource` for textures created from image data
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OnDisk {
    /// The path to the image data
    pub path: &'static str 
}

impl TextureSource for OnDisk {
    fn into_data(self, format: wgpu::TextureFormat) -> Result<(Option<Vec<u8>>, TexDimensions), String> {
        match image::open(self.path)
            .map_err(|err| format!("Failed to read image file with path {}: {}", self.path, err)) 
        {
            Ok(img) => {
                let dim = TexDimensions {
                    width: img.width(),
                    height: img.height(),
                    depth: 1,
                    wgpu_dim: wgpu::TextureDimension::D2
                };

                let raw_data = match format {
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                        img.to_bgra8().into_raw()
                    },
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
                        img.to_rgba8().into_raw()
                    },
                    wgpu::TextureFormat::R8Unorm => {
                        img.to_luma8().into_raw()
                    }
                    _ => {
                        return Err(format!(
                            "Unsupported Texture Format {:?} for image data from path @'{}'", format, self.path
                        ));
                    }
                };

                Ok((Some(raw_data), dim))
            },
            Err(e) => return Err(e)
        }
    }
}


/// A `TextureSource` for textures created from procedural generation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Procedural { 
    /// The procedurally generated texture data. Must match the dimensions
    pub data: Vec<u8>,
    /// The dimensions (width/height/depth) of the texture data
    pub dim: TexDimensions,
}

impl TextureSource for Procedural {
    fn into_data(self, _format: wgpu::TextureFormat) -> Result<(Option<Vec<u8>>, TexDimensions), String> {
        Ok((Some(self.data), self.dim))
    }
}

/// A `TextureSource` for textures created by running a compute shader
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Computed {
    /// The dimensions (width/height/depth) of the output from the compute shader
    pub dim: TexDimensions
}

impl TextureSource for Computed {
    fn into_data(self, _format: wgpu::TextureFormat) -> Result<(Option<Vec<u8>>, TexDimensions), String> {
        Ok((None, self.dim))
    }
}

/// A blueprint for constructing wgpu textures.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Texture {
    /// Label for gpu profiling
    pub label: String,
    /// The format of the texture, default is `wgpu::TextureFormat::Rgba8Unorm`
    pub format: wgpu::TextureFormat,
    /// The usage of the texture, default is `wgpu::TextureUsages::TEXTURE_BINDING`
    pub usage: wgpu::TextureUsages,
    /// The mip levels of the texture, default is 1.
    pub mip_levels: u32,
}

impl Texture {
    pub fn new() -> Self {
        Self {
            label: "Texture".to_string(),
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            mip_levels: 1,
        }
    }

    /// Set the label for gpu profiling of the resultant texture
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Set the format of the texture
    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the mipmap levels for the texture.
    pub fn with_mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// Add a additional usage for the texture
    pub fn with_additional_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage |= usage;
        self
    }

    /// Allow the texture to be written to
    pub fn writable(mut self) -> Self {
        self.usage |= wgpu::TextureUsages::COPY_DST;
        self
    }

    /// Allow the texture to be read from
    pub fn readable(mut self) -> Self {
        self.usage |= wgpu::TextureUsages::COPY_SRC;
        self
    }
    
    /// Allow the texture to be written to and read from
    pub fn read_write(mut self) -> Self {
        self.usage |= wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC;
        self
    }

    /// Allows the texture to be used for storage in a bind group
    pub fn storage_bindable(mut self) -> Self {
        self.usage |= wgpu::TextureUsages::STORAGE_BINDING;
        self
    }

    /// Allows the texture to be used as a render attachment
    pub fn render_attachment(mut self) -> Self {
        self.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        self
    }

    /// Convert this texture blueprint into it's payload representation, given some initial data and dimensions.
    pub(crate) fn into_payload(&self, data: Option<Vec<u8>>, dim: TexDimensions) -> TexturePayload {
        TexturePayload {
            label: self.label.clone(),
            format: self.format,
            usage: self.usage,
            mip_levels: self.mip_levels,
            size: dim,
            data
        }
    }
}

impl ResourceId for Texture {
    type Id = TextureId;

    fn create_id(&self) -> Self::Id {
        TextureId(Uuid::new_v4())
    }
}

/// Condensed version of a texture blueprint. Used to streamline the creation of the corresponding wgpu texture
pub struct TexturePayload {
    pub label: String,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub size: TexDimensions, 
    pub mip_levels: u32,
    pub data: Option<Vec<u8>>,
}

impl TexturePayload {
    /// Determine the number of bytes per pixel based on the currently set format
    pub(crate) fn bytes_per_pixel(&self) -> u32 {
        match self.format {
            wgpu::TextureFormat::R8Unorm | wgpu::TextureFormat::R8Snorm => 1,
            wgpu::TextureFormat::Rg8Unorm | wgpu::TextureFormat::Rg8Snorm => 2,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb => 4,
            wgpu::TextureFormat::R32Float => 4,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            _ => panic!("Unsupported texture format for automatic layout calculation: {:?}", self.format),
        }
    }
}