use std::marker::PhantomData;

/// Represents structs that can be serialized into raw bytes.
/// 
/// All structs that implement bytemuck::Pod and bytemuck::Zeroable 
/// automatically implement Serializable.
pub trait Serializable {
    /// Convert this struct into a slice of bytes
    fn to_bytes(&self) -> &[u8];
}

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
    /// The offset at which to insert the data at into the buffer.
    fn offset(&self) -> u64;
}

/// A buffer update from any struct that implements the Serializable trait
/// 
/// Structs which implement bytemuck's POD and Zeroable automatically implement Serializable
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

/// Represents resources that can be condensed into a buffer payload
pub trait BufferType: Send + 'static {
    fn into_payload(self) -> Result<BufferPayload, String>;
}

/// A blueprint for constructing wgpu Buffers, where T is the buffer type.
pub struct Buffer<T> {
    pub label: String,
    pub usage: wgpu::BufferUsages,
    pub capacity: u64,
    pub init_data: Option<Vec<u8>>,
    pub ty: PhantomData<T>
}

impl<T> Buffer<T> {
    /// Set the label for gpu profiling of the resultant buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Provide this buffer with initial byte data. 
    /// 
    /// The buffer will always be at least big enough to hold this data, however if a larger capacity was set, the buffer will be that size.
    pub fn with_byte_data(mut self, data: &[u8]) -> Self {
        self.init_data = Some(data.to_vec());
        self
    }

    /// Provide this buffer with initial data from a Serializable struct. 
    /// 
    /// The buffer will always be at least big enough to hold this data, however if a larger capacity was set, the buffer will be that size.
    pub fn with_struct_data(mut self, data: impl Serializable) -> Self {
        self.init_data = Some(data.to_bytes().to_vec());
        self
    }

    /// Set the initial capacity. 
    /// 
    /// If initial data was set, this value is only used if its larger than the size of the data.
    /// 
    /// If the buffer type is Uniform, this value will be rounded up to the nearest multiple of 16.
    pub fn with_capacity(mut self, capacity: u64) -> Self {
        self.capacity = capacity;
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

    /// Allow the buffer to be readable from.
    pub fn readable(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_SRC;
        self
    }

    /// Allow the buffer to be readable from and writable to.
    pub fn read_write(mut self) -> Self {
        self.usage |= wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC;
        self
    }
}

/// A uniform buffer type
pub struct Uniform;

impl Buffer<Uniform> {
    /// Create a buffer blueprint with the uniform usage type.
    /// 
    /// If the provided contents contains data, the data is padded to a multiple of 16 to 
    /// follow wgpu uniform buffer alignment rules.
    pub fn as_uniform() -> Self {
        Self {
            label: "uniform_buffer".to_string(),
            capacity: 16,
            usage: wgpu::BufferUsages::UNIFORM,
            init_data: None,
            ty: std::marker::PhantomData
        }
    }

    /// Pads a byte vector to align the size to a multiple of the provided value
    pub fn pad_bytes(mut data: Vec<u8>, alignment: usize) -> Vec<u8> {
        let remainder = data.len() % alignment;
        if remainder > 0 {
            let padding = alignment - remainder;
            data.resize(data.len() + padding, 0u8);
        }

        data
    }

    /// Round the value to the nearest multiple of the provided alignment
    pub fn align(mut value: u64, alignment: u64) -> u64 {
        let remainder = value % alignment;
        if remainder > 0 {
            let padding = alignment - remainder;
            value += padding;
        }

        value
    }
}

impl BufferType for Buffer<Uniform> {
    fn into_payload(self) -> Result<BufferPayload, String> {
        let init_data = self.init_data.map(|data| {
            Buffer::<Uniform>::pad_bytes(data, 16)
        });

        let capacity = Buffer::<Uniform>::align(self.capacity, 16);

        Ok(BufferPayload { 
            label: self.label, 
            usage: self.usage, 
            capacity,
            init_data
        })
    }
}

/// A storage buffer type
pub struct Storage;

impl Buffer<Storage> {
    /// Create a buffer blueprint with the storage usage type.
    pub fn as_storage() -> Self {
        Self {
            label: "storage_buffer".to_string(),
            usage: wgpu::BufferUsages::STORAGE,
            capacity: 16,
            init_data: None,
            ty: std::marker::PhantomData,
        }
    }
}

impl BufferType for Buffer<Storage> {
    fn into_payload(self) -> Result<BufferPayload, String> {
        Ok(BufferPayload { 
            label: self.label, 
            usage: self.usage, 
            capacity: self.capacity,
            init_data: self.init_data,
        })
    }
}

/// A vertex buffer type
pub struct Vertex;

impl Buffer<Vertex> {
    /// Create a buffer blueprint with the vertex usage type.
    pub fn as_vertex() -> Self {
        Self {
            label: "vertex_buffer".to_string(),
            usage: wgpu::BufferUsages::VERTEX,
            capacity: 16,
            init_data: None,
            ty: std::marker::PhantomData
        }
    }
}

impl BufferType for Buffer<Vertex> {
    fn into_payload(self) -> Result<BufferPayload, String> {
        Ok(BufferPayload { 
            label: self.label, 
            usage: self.usage, 
            capacity: self.capacity,
            init_data: self.init_data,
        })
    }
}

/// An index buffer type
pub struct Index; 

impl Buffer<Index> {
    /// Create a buffer blueprint with the index usage type.
    pub fn as_index() -> Self {
        Self {
            label: "index_buffer".to_string(),
            usage: wgpu::BufferUsages::INDEX,
            capacity: 16,
            init_data: None,
            ty: std::marker::PhantomData
        }
    }
}

impl BufferType for Buffer<Index> {
    fn into_payload(self) -> Result<BufferPayload, String> {
        Ok(BufferPayload { 
            label: self.label, 
            usage: self.usage, 
            capacity: self.capacity,
            init_data: self.init_data,
        })
    }
}