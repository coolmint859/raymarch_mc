use std::{ops::Deref, sync::Arc};

use crate::graphics::GpuHandle;

/// Describes the role of a texture as used by in a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureRole {
    /// The texture is used to be sampled using a sampler in the shader
    Sampled{ filterable: bool },
    /// A texture thats stored and accessed only through shaders
    Storage,
}

impl TextureRole {
    /// // Convert the texture role into it's equivalent wgpu binding type
    pub fn as_binding_type(&self) -> wgpu::BindingType {
        match self {
            TextureRole::Sampled{ filterable} => wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: *filterable },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false
            },
            TextureRole::Storage => wgpu::BindingType::StorageTexture { 
                access: wgpu::StorageTextureAccess::WriteOnly, 
                format: wgpu::TextureFormat::Rgba8Unorm, 
                view_dimension: wgpu::TextureViewDimension::D2
            }
        }
    }
}

/// A lightweight handle to a gpu texture
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureHandle {
    pub texture: Arc<wgpu::Texture>,
    pub view: Arc<wgpu::TextureView>,
}

impl Deref for TextureHandle {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &*self.view
    }
}

/// Defines the type of texture creation mechanism
pub enum TextureType {
    /// A texture created via an algorithm
    Procedural { width: u32, height: u32, data: Vec<u8>},
    // /// A texture created by loading a file from disk
    // OnDisk { path: String },
    /// A texture created via running a compute shader
    Computed { width: u32, height: u32 }
}

impl TextureType {
    pub fn extent(&self) -> wgpu::Extent3d {
        match self {
            TextureType::Procedural { width, height, data: _ } => {
                wgpu::Extent3d {
                    width: *width,
                    height: *height,
                    depth_or_array_layers: 1
                }
            },
            TextureType::Computed { width, height } => {
                wgpu::Extent3d {
                    width: *width,
                    height: *height,
                    depth_or_array_layers: 1
                }
            }
        }
    }
}

pub struct TextureBuilder {
    pub label: String,
    pub texture_type: TextureType,
    pub format: wgpu::TextureFormat,
    pub dimensions: wgpu::TextureDimension,
    pub usage: wgpu::TextureUsages,
}

impl TextureBuilder {
    pub fn new(ty: TextureType) -> Self {
        Self {
            label: "texture".to_string(),
            texture_type: ty,
            format: wgpu::TextureFormat::Rgba8Unorm,
            dimensions: wgpu::TextureDimension::D2,
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

    /// Set the texture dimension configuration
    pub fn with_dimension(mut self, dimensions: wgpu::TextureDimension) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// Add a additional usage for the texture
    pub fn with_additional_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage |= usage;
        self
    }
}

/// Create a new texture from the given configuration builder
pub async fn create_texture(gpu: GpuHandle, builder: TextureBuilder) -> Result<TextureHandle, String> {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&builder.label),
        size: builder.texture_type.extent(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: builder.dimensions,
        format: builder.format,
        usage: builder.usage,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    println!("[GpuContext] Created new texture with label '{}'", builder.label);

    Ok(TextureHandle {
        texture: Arc::new(texture),
        view: Arc::new(view),
    })
}