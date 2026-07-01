use std::{ops::Deref, sync::Arc};
use crate::graphics::{GpuHandle, ResourceBuilder};

/// Describes the role of the texture as used by the gpu
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureRole {
    /// A sampled texture for use in rendering
    Sampled,
    /// A texture thats stored and accessed only through shaders
    Storage,
}

impl TextureRole {
    /// // Convert the texture role into it's equivalent wgpu binding type
    pub fn as_binding_type(&self) -> wgpu::BindingType {
        match self {
            TextureRole::Sampled => wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
    texture: Arc<wgpu::Texture>,
    view: Arc<wgpu::TextureView>,
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
    label: String,
    texture_type: TextureType,
    format: wgpu::TextureFormat,
    dimensions: wgpu::TextureDimension,
    usage: wgpu::TextureUsages,
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

impl ResourceBuilder for TextureBuilder {
    type Resource = TextureHandle;

    fn build(&self, gpu: GpuHandle) -> Self::Resource {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&self.label),
            size: self.texture_type.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: self.dimensions,
            format: self.format,
            usage: self.usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        TextureHandle {
            texture: Arc::new(texture),
            view: Arc::new(view),
        }
    }
}