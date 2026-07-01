use std::{ops::Deref, sync::Arc};

use wgpu::util::DeviceExt;

use crate::graphics::{GpuHandle, ResourceBuilder, WgpuResource};

/// The role of the buffer as used by shaders
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferRole {
    Uniform,
    Storage(bool)
}

impl BufferRole {
    /// Convert the buffer role into it's equivalent wgpu Buffer Usage
    fn as_usage(&self) -> wgpu::BufferUsages {
        match self {
            BufferRole::Uniform => wgpu::BufferUsages::UNIFORM,
            BufferRole::Storage(_) => wgpu::BufferUsages::STORAGE
        }
    }

    /// Convert the buffer role into it's equivalent wgpu binding type
    fn as_binding_type(&self) -> wgpu::BindingType {
        match self {
            BufferRole::Uniform => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            BufferRole::Storage(read_only) => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: *read_only },
                has_dynamic_offset: false,
                min_binding_size: None
            }
        }
    }
}

/// A lightweight handle to a gpu buffer
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferHandle {
    pub buffer: Arc<wgpu::Buffer>,
    pub role: BufferRole,
    pub visibility: wgpu::ShaderStages,
}

impl WgpuResource for BufferHandle {
    fn binding_type(&self) -> wgpu::BindingType {
        self.role.as_binding_type()
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }

    fn as_binding(&self) -> wgpu::BindingResource<'_> {
        self.buffer.as_entire_binding()
    }
}

impl Deref for BufferHandle {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &*self.buffer
    }
}

/// Describes the contents of a buffer
pub enum BufferContents {
    /// A buffer created with initial data
    WithData(Vec<u8>),
    /// A buffer created with no initial data but with an initial capacity
    Empty(u64)
}

pub struct BufferBuilder {
    label: String,
    usage: wgpu::BufferUsages,
    contents: BufferContents,
    role: BufferRole,
    visibility: wgpu::ShaderStages
}

impl BufferBuilder {
    pub fn new(role: BufferRole, contents: BufferContents) -> Self {
        Self {
            label: "buffer".to_string(),
            usage: role.as_usage(),
            contents,
            role,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT
        }
    }

    /// Create a buffer builder with the uniform usage type.
    /// 
    /// If the provided contents contains data, the data is padded to a multiple of 16 to 
    /// follow wgpu uniform buffer alignment rules.
    pub fn as_uniform(contents: BufferContents) -> Self {
        // pad the data in contents to 16 bytes if provided
        let contents = match contents {
            BufferContents::WithData(data) => {
                BufferContents::WithData(
                    BufferBuilder::pad_bytes(data, 16)
                )
            },
            _ => { contents }
        };

        BufferBuilder::new(BufferRole::Uniform, contents)
    }

    /// Create a buffer builder with the storage usage type
    pub fn as_storage(contents: BufferContents, read_only: bool) -> Self {
        BufferBuilder::new(BufferRole::Storage(read_only), contents)
    }

    /// Set the label for gpu profiling of the resultant buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add an additional usage flag to the resultant buffer
    pub fn with_additional_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage |= usage;
        self
    }

    /// Set the shader visibility of the resultant buffer
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }

    /// Pads a byte vector to align the size to a multiple of the provided value
    pub fn pad_bytes(mut data: Vec<u8>, alignment: usize) -> Vec<u8> {
        let remainder = data.len() % alignment;
        if remainder > 0 {
            let padding= alignment - remainder;
            data.resize(data.len() + padding, 0u8);
        }

        data
    }
}

impl ResourceBuilder for BufferBuilder {
    type Resource = BufferHandle;

    fn build(&self, gpu: GpuHandle) -> BufferHandle {
        let buffer = match &self.contents {
            BufferContents::Empty(size) => {
                gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&self.label),
                    size: *size,
                    usage: self.usage,
                    mapped_at_creation: false
                })
            },
            BufferContents::WithData(data) => {
                gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&self.label),
                    contents: &data,
                    usage: self.usage
                })
            }
        };

        BufferHandle {
            buffer: Arc::new(buffer),
            visibility: self.visibility,
            role: self.role
        }
    }
}