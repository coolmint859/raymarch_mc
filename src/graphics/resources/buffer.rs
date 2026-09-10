/// Represents structs that can be serialized into raw bytes.
/// 
/// All structs that implement bytemuck::Pod and bytemuck::Zeroable 
/// automatically implement Serializable.
pub trait Serializable {
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

/// Describes the contents of a buffer
pub enum BufferContents {
    /// A buffer created with initial byte data
    WithData(Vec<u8>),
    /// A buffer created with no initial data but with an initial capacity
    Empty(u64)
}

/// A blueprint for constructing wgpu Buffers
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

    /// Create a buffer builder with the vertex usage type
    pub fn as_vertex(contents: BufferContents) -> Self {
        Buffer::new(wgpu::BufferUsages::VERTEX, contents)
    }

    /// Create a buffer builder with the index usage type
    pub fn as_index(contents: BufferContents) -> Self {
        Buffer::new(wgpu::BufferUsages::INDEX, contents)
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