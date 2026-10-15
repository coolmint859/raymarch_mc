use std::marker::PhantomData;

use uuid::Uuid;

use crate::graphics::{BufferId, ResourceId};

/// Represents types that can be serialized into slices of bytes.
/// 
/// All types that implement `bytemuck::Pod` and `bytemuck::Zeroable` 
/// automatically implement `Serializable`.
pub trait Serializable {
    /// Converts `self` into a slice of bytes
    fn to_bytes(&self) -> &[u8];
}

impl<T> Serializable for T 
where T: bytemuck::Pod + bytemuck::Zeroable
{
    fn to_bytes(&self) -> &[u8] { bytemuck::bytes_of(self) }
}

/// Represents buffer sources with no initial data.
pub struct Empty;
/// Represents buffer sources with some initial data.
pub struct WithData;

/// A wrapper around initial data sources for gpu buffers
pub struct BufferSource<State> {
    pub data: Option<Vec<u8>>,
    pub capacity: u64,
    _state: PhantomData<State>
}

impl BufferSource<Empty> {
    /// No initial data.
    /// 
    /// If the buffer this source is populated with is a uniform buffer, then `capacity` 
    /// is rounded to the nearest 16 bytes, in accordance with wgpu uniform buffer alignment rules.
    pub fn empty(capacity: u64) -> Self {
        Self {
            data: None,
            capacity,
            _state: PhantomData
        }
    }
}

impl BufferSource<WithData> {
    /// Initial data from a type `T` that implements `Serializable`
    pub fn from_pod<T: Serializable>(data: T) -> Self {
        let bytes = data.to_bytes().to_vec();

        Self {
            capacity: bytes.len() as u64,
            data: Some(bytes),
            _state: PhantomData,
        }
    }

    /// Initial data from a raw byte vector
    pub fn from_raw(data: Vec<u8>) -> Self {
        Self {
            capacity: data.len() as u64,
            data: Some(data),
            _state: PhantomData
        }
    }

    /// Set the capacity of the buffer populated with this source data.
    /// 
    /// If `capacity` is less then the provided data's byte length, then the length of the data is used instead and `capacity` is ignored.
    /// 
    /// If the buffer this source is populated with is a uniform buffer, then `capacity` 
    /// is rounded to the nearest 16 bytes, in accordance with wgpu uniform buffer alignment rules.
    pub fn with_capacity(mut self, capacity: u64) -> Self {
        let capacity = match &self.data {
            Some(data)  => (data.len() as u64).max(capacity),
            None => capacity
        };

        self.capacity = capacity;
        self
    }
}


/// Represents an update into an existing buffer
pub trait BufferUpdate {
    /// The data payload as an array slice of bytes
    fn bytes(&self) -> &[u8];
    /// The offset at which to insert the data at into the buffer.
    fn offset(&self) -> u64;
}

/// A buffer update from any type that implements the `Serializable` trait
/// 
/// Types which implement `bytemuck::POD` and `bytemuck::Zeroable` automatically implement `Serializable`
pub struct StructuredUpdate<'a, T: Serializable> {
    /// The struct payload
    pub data: &'a T,
    /// The offset at which to insert the struct data at into the buffer
    pub offset: u64,
}

impl<'a, T: Serializable> BufferUpdate for StructuredUpdate<'a, T> {
    #[inline]
    fn bytes(&self) -> &[u8] { self.data.to_bytes() }

    #[inline]
    fn offset(&self) -> u64 { self.offset }
}

/// A buffer update from raw bytes, inserted at an offset
pub struct RawBytesUpdate<'a> {
    /// The raw data payload
    pub data: &'a [u8],
    /// The offset at which to insert the raw data at into the buffer
    pub offset: u64,
}

impl<'a> BufferUpdate for RawBytesUpdate<'a> {
    #[inline]
    fn bytes(&self) -> &[u8] { self.data }

    #[inline]
    fn offset(&self) -> u64 { self.offset }
}

/// Condensed version of the generic buffer struct.
pub struct BufferPayload {
    pub label: String,
    pub usage: wgpu::BufferUsages,
    pub capacity: u64,
    pub init_data: Option<Vec<u8>>
}

/// A blueprint for constructing wgpu Buffers, where T is the buffer type.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Buffer {
    pub label: String,
    pub usage: wgpu::BufferUsages,
    pub alignment: Option<u64>
}

impl Buffer {
    /// Create a buffer blueprint with the uniform usage type.
    /// 
    /// All Uniform buffers have a default capacity of 16 to follow wgpu alignment rules
    pub fn as_uniform() -> Self {
        Self {
            label: "uniform_buffer".to_string(),
            usage: wgpu::BufferUsages::UNIFORM,
            alignment: Some(16)
        }
    }

    /// Create a buffer blueprint with the storage usage type.
    pub fn as_storage() -> Self {
        Self {
            label: "storage_buffer".to_string(),
            usage: wgpu::BufferUsages::STORAGE,
            alignment: None
        }
    }

    /// Create a buffer blueprint with the vertex usage type.
    pub fn as_vertex() -> Self {
        Self {
            label: "vertex_buffer".to_string(),
            usage: wgpu::BufferUsages::VERTEX,
            alignment: None
        }
    }

    /// Create a buffer blueprint with the index usage type.
    pub fn as_index() -> Self {
        Self {
            label: "index_buffer".to_string(),
            usage: wgpu::BufferUsages::INDEX,
            alignment: None
        }
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

    /// Allow the buffer to be written to.
    pub fn writable(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_DST;
        self
    }

    /// Allow the buffer to be read from.
    pub fn readable(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_SRC;
        self
    }

    /// Allow the buffer to be read from and writable to.
    pub fn read_write(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC;
        self
    }

    /// Pads `data` with 0s to align the length to a multiple of `alignment`
    pub fn pad_bytes(mut data: Vec<u8>, alignment: usize) -> Vec<u8> {
        let remainder = data.len() % alignment;
        if remainder > 0 {
            let padding = alignment - remainder;
            data.resize(data.len() + padding, 0u8);
        }

        data
    }

    /// Rounds `value` to the nearest multiple of `alignment`.
    pub fn align(mut value: u64, alignment: u64) -> u64 {
        let remainder = value % alignment;
        if remainder > 0 {
            let padding = alignment - remainder;
            value += padding;
        }

        value
    }

    /// Convert this buffer blueprint into it's payload representation, given some initial data and capacity.
    /// 
    /// Aligns `capacity` to the nearest multiple of 16 if this buffer has the uniform usage.
    pub(crate) fn into_payload(self, data: Option<Vec<u8>>, capacity: u64) -> BufferPayload {
        let capacity = if let Some(alignment) = self.alignment {
            Buffer::align(capacity, alignment)
        } else {
            capacity
        };

        BufferPayload {
            label: self.label,
            usage: self.usage,
            capacity,
            init_data: data
        }
    }
}

impl ResourceId for Buffer {
    type Id = BufferId;

    fn create_id(&self) -> Self::Id {
        BufferId(Uuid::new_v4())
    }
}