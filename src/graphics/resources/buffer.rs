use std::{ops::Deref, sync::Arc};

/// Describes the role of a buffer as used in a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferRole {
    /// The buffer is used as a uniform in a shader
    Uniform,
    /// The buffer is used for storage in a shader
    Storage{ read_only: bool }
}

impl BufferRole {
    /// Convert the buffer role into it's equivalent wgpu binding type
    pub fn as_binding_type(&self) -> wgpu::BindingType {
        match self {
            BufferRole::Uniform => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            BufferRole::Storage{read_only} => wgpu::BindingType::Buffer {
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
    pub label: String,
    pub usage: wgpu::BufferUsages,
    pub contents: BufferContents,
}

impl BufferBuilder {
    pub fn new(usage: wgpu::BufferUsages, contents: BufferContents) -> Self {
        Self {
            label: "buffer".to_string(),
            usage,
            contents,
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

        BufferBuilder::new(wgpu::BufferUsages::UNIFORM, contents)
    }

    /// Create a buffer builder with the storage usage type
    pub fn as_storage(contents: BufferContents) -> Self {
        BufferBuilder::new(wgpu::BufferUsages::STORAGE, contents)
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