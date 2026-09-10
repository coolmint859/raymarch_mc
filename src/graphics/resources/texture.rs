use std::ops::Deref;

use image::GenericImageView;
use wgpu::Extent3d;

/// A lightweight handle to a gpu texture
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureHandle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub extent: Extent3d,
}

impl Deref for TextureHandle {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// The type of texture creation mechanism. This is used internally to create the underlying wgpu texture
pub enum TextureType {
    /// A texture created via an algorithm
    Procedural(Texture<Procedural>),
    /// A texture created by loading a file from disk
    OnDisk(Texture<OnDisk>),
    /// A texture created via running a compute shader
    Computed(Texture<Computed>)
}

impl TextureType {
    /// Convert the texture into its full payload information. This is called when creating the wgpu texture via the Device
    pub(crate) fn into_payload(self) -> Result<TexturePayload, String> {
        match self {
            TextureType::Procedural(tex) => {
                Ok(tex.into_payload())
            },
            TextureType::OnDisk(tex) => {
                tex.into_payload()
            },
            TextureType::Computed(tex) => {
                Ok(tex.into_payload())
            }
        }
    }
}

impl From<Texture<Procedural>> for TextureType {
    fn from(tex: Texture<Procedural>) -> Self {
        Self::Procedural(tex)
    }
}

impl From<Texture<OnDisk>> for TextureType {
    fn from(tex: Texture<OnDisk>) -> Self {
        Self::OnDisk(tex)
    }
}

impl From<Texture<Computed>> for TextureType {
    fn from(tex: Texture<Computed>) -> Self {
        Self::Computed(tex)
    }
}

/// Specifies the dimensions of a texture
#[derive(Clone, Debug)]
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

/// Description for a procedurally generated texture
pub struct Procedural { 
    pub data: Vec<u8>,
    pub dim: TexDimensions,
}

impl Procedural {
    /// Provide the default format for this texture type
    pub(crate) fn default_fmt() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }
}

/// Description for a texture loaded from disk
pub struct OnDisk {
    pub path: &'static str
}

impl OnDisk {
    /// Provide the default format for this texture type
    pub(crate) fn default_fmt() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }
}

/// Description for a texture generated in a compute shader
pub struct Computed {
    pub dim: TexDimensions
}

impl Computed {
    /// Provide the default format for this texture type
    pub(crate) fn default_fmt() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba16Float
    }
}

/// Condensed version of a texture blueprint. Used to streamline the creation of the corresponding wgpu texture
#[derive(Clone, Debug)]
pub(crate) struct TexturePayload {
    pub label: String,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub dim: TexDimensions, 
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

/// A blueprint for constructing wgpu textures. 
pub struct Texture<T> {
    pub label: String,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    mip_levels: u32,
    ty: T,
}

impl<T> Texture<T> {
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
}

impl Texture<Procedural> {
    /// Create a texture from procedurally generated data
    pub fn procedural(data: Vec<u8>, dim: TexDimensions) -> Self {
        Self {
            label: "procedural_texture".to_string(),
            format: Procedural::default_fmt(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            mip_levels: 1,
            ty: Procedural { data, dim }
        }
    }

    /// Convert the texture into its full payload information. This is called when creating the wgpu texture via the Device
    pub(crate) fn into_payload(self) -> TexturePayload {
        TexturePayload {
            label: self.label,
            dim: self.ty.dim, 
            format: self.format,
            usage: self.usage,
            mip_levels: self.mip_levels,
            data: Some(self.ty.data),
        }
    }
}

impl Texture<OnDisk> {
    /// Create a texture from an image file
    pub fn on_disk(path: &'static str) -> Self {
        Self {
            label: "disk_loaded_texture".to_string(),
            format: OnDisk::default_fmt(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            mip_levels: 1,
            ty: OnDisk { path }
        }
    }

    /// Convert the texture into its full payload information. This is called when creating the wgpu texture via the Device
    pub(crate) fn into_payload(self) -> Result<TexturePayload, String> {
        image::open(self.ty.path)
            .map(|img| {
                let dim = TexDimensions {
                    width: img.width(),
                    height: img.height(),
                    depth: 1,
                    wgpu_dim: wgpu::TextureDimension::D2
                };

                TexturePayload {
                    label: self.label,
                    format: self.format,
                    usage: self.usage,
                    data: Some(img.to_rgba8().into_raw()),
                    mip_levels: self.mip_levels,
                    dim,
                }
            })
            .map_err(|err| format!("Failed to read image file with path {}: {}", self.ty.path, err))
    }
}

impl Texture<Computed> {
    /// Create a texture generated by a compute shader
    pub fn computed(dim: TexDimensions) -> Self {
        Self {
            label: "computed_texture".to_string(),
            format: Computed::default_fmt(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            mip_levels: 1,
            ty: Computed { dim }
        }
    }

    /// Convert the texture into its full payload information. This is called when creating the wgpu texture via the Device
    pub(crate) fn into_payload(self) -> TexturePayload {
        TexturePayload {
            label: self.label,
            dim: self.ty.dim, 
            format: self.format,
            usage: self.usage,
            mip_levels: self.mip_levels,
            data: None, // generated in the shader
        }
    }
}