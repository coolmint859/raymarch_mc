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

/// Represents resources that can be condensed into a texture payload.
pub trait TextureType: Send + 'static {
    /// Convert the texture into its full payload information, if possible. This is called when creating the wgpu texture via the Device
    fn into_payload(self) -> Result<TexturePayload, String>;
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
}

impl TextureType for Texture<Procedural> {
    fn into_payload(self) -> Result<TexturePayload, String> {
        Ok(TexturePayload {
            label: self.label,
            size: self.ty.dim, 
            format: self.format,
            usage: self.usage,
            mip_levels: self.mip_levels,
            data: Some(self.ty.data),
        })
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
}

impl TextureType for Texture<OnDisk> {
    fn into_payload(self) -> Result<TexturePayload, String> {
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
                    size: dim,
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
}

impl TextureType for Texture<Computed> {
    fn into_payload(self) -> Result<TexturePayload, String> {
        Ok(TexturePayload {
            label: self.label,
            size: self.ty.dim, 
            format: self.format,
            usage: self.usage,
            mip_levels: self.mip_levels,
            data: None, // generated in the shader
        })
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