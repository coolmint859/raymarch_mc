use std::{num::NonZero, ops::Deref};

use wgpu::{util::DeviceExt};

use crate::graphics::{Bindable, BindingTarget, BufferId, GpuHandle};

/// Represents structs that can be serialized into raw bytes
pub trait Serializable {
    fn to_bytes(&self) -> &[u8];
}

// Any struct that implements Pod and Zeroable is serializable into bytes
impl<T> Serializable for T 
where T: bytemuck::Pod + bytemuck::Zeroable
{
    /// Converts the T struct into an array slice of bytes
    fn to_bytes(&self) -> &[u8] { bytemuck::bytes_of(self) }
}

/// Represents an update into an existing gpu buffer
pub trait BufferUpdate {
    /// The data payload as an array slice of bytes
    fn bytes(&self) -> &[u8];
    /// The offset at which to apply the update.
    fn offset(&self) -> u64;
}

/// A buffer update from any struct that implements the Serializable trait
/// 
/// Structs which implement bytemuck's POD and Zeroable automatically implement Serializable
pub struct StructuredUpdate<'a, T: Serializable> {
    pub data: &'a T
}

impl<'a, T: Serializable> BufferUpdate for StructuredUpdate<'a, T> {
    #[inline]
    fn bytes(&self) -> &[u8] { self.data.to_bytes() }

    #[inline]
    fn offset(&self) -> u64 { 0 } // all structures have an offset of 0
}

/// A buffer update from raw bytes, inserted at an offset
pub struct RawBytesUpdate<'a> {
    pub offset: u64,
    pub data: &'a [u8]
}

impl<'a> BufferUpdate for RawBytesUpdate<'a> {
    #[inline]
    fn bytes(&self) -> &[u8] { self.data }

    #[inline]
    fn offset(&self) -> u64 { self.offset }
}

/// Represents a buffer binding and entry in a bind group
pub struct BufferBinding {
    buf_id: BufferId,
    ty: wgpu::BufferBindingType,
    visibility: wgpu::ShaderStages,
    has_dyn_offset: bool,
    min_binding_size: Option<NonZero<u64>>
}

impl BufferBinding {
    pub fn new(target: BufferId, ty: wgpu::BufferBindingType) -> Self {
        Self {
            buf_id: target,
            ty,
            visibility: wgpu::ShaderStages::FRAGMENT,
            has_dyn_offset: false,
            min_binding_size: None,
        }
    }

    /// Create a new storage buffer binding
    pub fn as_storage(target: BufferId, read_only: bool) -> Self {
        let ty = wgpu::BufferBindingType::Storage { read_only };
        BufferBinding::new(target, ty)
    }

    /// Create a new uniform buffer binding
    pub fn as_uniform(target: BufferId) -> Self {
        let ty = wgpu::BufferBindingType::Uniform;
        BufferBinding::new(target, ty)
    }

    /// Set the shader stage visibility for the buffer binding
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set the binding to have a dynamic offset
    pub fn with_dynamic_offset(mut self) -> Self {
        self.has_dyn_offset = true;
        self
    }

    /// Set the minimum buffer size for the binding. Must be greater than 0.
    pub fn with_min_size(mut self, size: u64) -> Self {
        self.min_binding_size = Some(NonZero::new(size)
            .expect("[Buffer Binding] Expected minimum binding size to be a non zero unsigned number."));
        self
    }
}

impl Bindable for BufferBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        wgpu::BindingType::Buffer {
            ty: self.ty,
            has_dynamic_offset: self.has_dyn_offset,
            min_binding_size: self.min_binding_size
        }
    }

    fn target(&self) -> BindingTarget {
        BindingTarget::Buffer(self.buf_id)
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}

/// A lightweight handle to a gpu buffer
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferHandle {
    pub buffer: wgpu::Buffer,
}

impl Deref for BufferHandle {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

/// Describes the contents of a buffer
pub enum BufferContents {
    /// A buffer created with initial byte data
    WithData(Vec<u8>),
    /// A buffer created with no initial data but with an initial capacity
    Empty(u64)
}

pub struct Buffer {
    pub label: String,
    pub usage: wgpu::BufferUsages,
    pub contents: BufferContents,
}

impl Buffer {
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
                    Buffer::pad_bytes(data, 16)
                )
            },
            _ => { contents }
        };

        Buffer::new(wgpu::BufferUsages::UNIFORM, contents)
    }

    /// Create a buffer builder with the storage usage type
    pub fn as_storage(contents: BufferContents) -> Self {
        Buffer::new(wgpu::BufferUsages::STORAGE, contents)
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

/// Create a buffer from the given configuration builder
pub async fn create_buffer(gpu: GpuHandle, builder: Buffer) -> Result<BufferHandle, String> {
    let buffer = match &builder.contents {
        BufferContents::Empty(size) => {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&builder.label),
                size: *size,
                usage: builder.usage,
                mapped_at_creation: false
            })
        },
        BufferContents::WithData(data) => {
            gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&builder.label),
                contents: &data,
                usage: builder.usage
            })
        }
    };

    println!("[GpuContext] Created new buffer with label '{}'", builder.label);

    Ok(BufferHandle { buffer })
}