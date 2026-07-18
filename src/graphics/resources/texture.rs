use std::ops::Deref;

use image::GenericImageView;

use crate::graphics::{Bindable, BindingTarget, GpuHandle, TextureId};

/// Options for configurating a storage texture binding
pub struct TextureTypeStorage {
    access: wgpu::StorageTextureAccess, 
    fmt: wgpu::TextureFormat
}

impl Default for TextureTypeStorage {
    fn default() -> Self {
        Self {
            access: wgpu::StorageTextureAccess::WriteOnly,
            fmt: wgpu::TextureFormat::Rgba16Float
        }
    }
}

/// options for configuring a sampled texture binding
pub struct TextureTypeSampled {
    filterable: bool, 
    multisampled: bool,
}

impl Default for TextureTypeSampled {
    fn default() -> Self {
        Self { filterable: false, multisampled: false }
    }
}

/// Represents a texture binding and entry in a bind group
pub struct TextureBinding {
    tex_id: TextureId,
    ty: wgpu::BindingType,
    visibility: wgpu::ShaderStages,
    view_dimensions: wgpu::TextureViewDimension
}

impl TextureBinding {
    pub fn new(target: TextureId, ty: wgpu::BindingType) -> Self {
        Self {
            tex_id: target,
            ty,
            visibility: wgpu::ShaderStages::FRAGMENT,
            view_dimensions: wgpu::TextureViewDimension::D2
        }
    }

    /// Create a new storage texture binding
    pub fn as_storage(target: TextureId, options: TextureTypeStorage) -> Self {
        let ty = wgpu::BindingType::StorageTexture { 
            access: options.access,
            format: options.fmt, 
            view_dimension: wgpu::TextureViewDimension::D2
        };

        TextureBinding::new(target, ty)
    }

    /// Create a new sampled texture binding
    pub fn as_sampled(target: TextureId, options: TextureTypeSampled) -> Self {
        let ty = wgpu::BindingType::Texture { 
            sample_type: wgpu::TextureSampleType::Float { filterable: options.filterable }, 
            view_dimension: wgpu::TextureViewDimension::D2, 
            multisampled: options.multisampled
        };

        TextureBinding::new(target, ty)
    }

    /// Set the view dimensions for the texture binding
    pub fn with_view_dimensions(mut self, view_dimensions: wgpu::TextureViewDimension) -> Self {
        self.view_dimensions = view_dimensions;
        self
    }

    /// Set the shader stage visibility for the texture binding
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }
}

impl Bindable for TextureBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        self.ty
    }

    fn target(&self) -> BindingTarget {
        BindingTarget::Texture(self.tex_id)
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}

/// A lightweight handle to a gpu texture
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureHandle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Deref for TextureHandle {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// Defines the type of texture creation mechanism
pub enum TextureType {
    /// A texture created via an algorithm
    Procedural { data: Vec<u8>},
    // /// A texture created by loading a file from disk
    OnDisk { path: String },
    /// A texture created via running a compute shader
    Computed
}

#[derive(Clone, Debug)]
pub(crate) struct TextureInfo {
    pub width: u32, 
    pub height: u32, 
    pub depth: u32, 
    pub data: Option<Vec<u8>>,
    pub dim: wgpu::TextureDimension,
}

pub struct Texture {
    pub label: String,
    pub texture_type: TextureType,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,

    dimensions: Option<TextureInfo>
}

impl Texture {
    pub fn new(ty: TextureType) -> Self {
        Self {
            label: "texture".to_string(),
            texture_type: ty,
            format: wgpu::TextureFormat::Rgba8Unorm,
            dimensions: None,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
        }
    }

    /// Set the label for gpu profiling of the resultant buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Set the format of the texture
    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_size_2d(mut self, width: u32, height: u32) -> Self {
        self.dimensions = Some(TextureInfo {
            width, 
            height, 
            depth: 1, 
            data: None,
            dim: wgpu::TextureDimension::D2,
        });
        self
    }

    pub fn with_size_3d(mut self, width: u32, height: u32, depth: u32) -> Self {
        self.dimensions = Some(TextureInfo {
            width, 
            height, 
            depth, 
            data: None,
            dim: wgpu::TextureDimension::D3,
        });
        self
    }

    /// Add a additional usage for the texture
    pub fn with_additional_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage |= usage;
        self
    }

    /// Determine the number of bytes per pixel based on the currently set format
    pub(crate) fn bytes_per_pixel(&self) -> u32 {
        match self.format {
            wgpu::TextureFormat::R8Unorm | wgpu::TextureFormat::R8Snorm => 1,
            wgpu::TextureFormat::Rg8Unorm | wgpu::TextureFormat::Rg8Snorm => 2,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb => 4,
            wgpu::TextureFormat::R32Float => 4,
            wgpu::TextureFormat::Rgba32Float => 16,
            // Add more formats as your engine expands, or fallback safely
            _ => panic!("Unsupported texture format for automatic layout calculation: {:?}", self.format),
        }
    }

    /// Get the format and data info associated with this texture
    pub(crate) async fn get_info(&self) -> Result<TextureInfo, String> {
        match &self.texture_type {
            TextureType::Computed => {
                self.dimensions.clone().ok_or(format!("[Texture] Computed Textures need to specified with a size via .with_size_2d() or .with_size_3d()"))
            },
            TextureType::Procedural { data } => {
                self.dimensions.clone()
                    .map(|mut dim| {
                        dim.data = Some(data.to_vec());
                        dim
                    })
                    .ok_or(format!("[Texture] Procedural Textures need to specified with a size via .with_size_2d() or .with_size_3d()"))
            }
            TextureType::OnDisk { path } => {
                image::open(path)
                    .map(|img| {
                        TextureInfo {
                            width: img.width(),
                            height: img.height(),
                            depth: 1,
                            data: Some(img.to_rgb8().into_raw()),
                            dim: wgpu::TextureDimension::D2,
                        }
                    })
                    .map_err(|err| format!("Failed to read image file with path {}: {}", path, err))
            }
        }
    }
}

/// Create a new texture from the given configuration builder
pub async fn create_texture(gpu: GpuHandle, builder: Texture) -> Result<TextureHandle, String> {
    let tex_info = builder.get_info().await?;
    let extent = wgpu::Extent3d {
        width: tex_info.width,
        height: tex_info.height,
        depth_or_array_layers: tex_info.depth
    };
    
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&builder.label),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: tex_info.dim,
        format: builder.format,
        usage: builder.usage,
        view_formats: &[],
    });

    if let Some(data) = &tex_info.data {
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            }, 
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(builder.bytes_per_pixel() * tex_info.width),
                rows_per_image: Some(tex_info.height)
            }, 
            extent,
        );
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    println!("[GpuContext] Created new texture with label '{}'", builder.label);

    Ok(TextureHandle { texture, view })
}