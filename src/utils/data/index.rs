use crate::graphics::{Buffer, BufferId, GpuContext, RawBytesUpdate, Serializable};

/// Represents index data
#[derive(Debug, Clone)]
pub enum IndexData {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl IndexData {
    pub fn format(&self) -> wgpu::IndexFormat {
        match self {
            IndexData::U16(_) => wgpu::IndexFormat::Uint16,
            IndexData::U32(_) => wgpu::IndexFormat::Uint32
        }
    }

    pub fn len(&self) -> usize {
        match self {
            IndexData::U16(data) => data.len(),
            IndexData::U32(data) => data.len(),
        }
    }
}

impl Serializable for IndexData {
    fn to_bytes(&self) -> &[u8] {
        match self {
            IndexData::U16(data) => bytemuck::cast_slice(data),
            IndexData::U32(data) => bytemuck::cast_slice(data)
        }
    }
}

/// Stores index data for a buffer used by a `GpuContext`.
#[derive(Debug, Clone)]
pub struct IndexBuffer {
    indices: IndexData,
    label: String,
    id: BufferId,
    is_dirty: bool,
}

impl IndexBuffer {
    /// Create a new `IndexBuffer` with format `Uint16`
    pub fn new_u16(indices: Vec<u16>) -> Self {
        Self {
            indices: IndexData::U16(indices),
            label: "U16 Index Buffer".to_string(),
            id: BufferId::UNINIT,
            is_dirty: true
        }
    }

    /// Create a new `IndexBuffer` with format `Uint32`
    pub fn new_u32(indices: Vec<u32>) -> Self {
        Self {
            indices: IndexData::U32(indices),
            label: "U32 Index Buffer".to_string(),
            id: BufferId::UNINIT,
            is_dirty: true
        }
    }

    /// Set the label for gpu profiling of this buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Initialize this buffer with a `GpuContext`.
    pub fn init(&mut self, context: &mut GpuContext) {
        self.id = context.request_buffer(
            Buffer::as_index()
                .with_label(&self.label)
                .with_byte_data(self.indices.to_bytes())
                .writable()
        );
        self.is_dirty = false;
    }

    /// Update the corresponding buffer on the gpu. This should be called whenever the index data is modified.
    pub fn update(&mut self, context: &mut GpuContext) {
        if !self.is_dirty { return; }

        let _ = context.update_buffer(&self.id, RawBytesUpdate {
            data: &self.indices.to_bytes(),
            offset: 0
        });

        self.is_dirty = false;
    }

    /// Returns a reference to the index data stored in this buffer
    pub fn get_indices(&self) -> &IndexData {
        &self.indices
    }

    /// Returns a mutable reference to the index data stored in this buffer
    pub fn get_indices_mut(&mut self) -> &mut IndexData {
        &mut self.indices
    }

    /// Returns the id to this buffer
    pub fn id(&self) -> &BufferId {
        &self.id
    }

    /// Returns the format of this buffer
    pub fn format(&self) -> wgpu::IndexFormat {
        self.indices.format()
    }

    /// Returns the size (length) of the index data 
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Returns the byte size of the index data 
    pub fn byte_size(&self) -> usize {
        self.indices.to_bytes().len()
    }

    /// Unintialize this buffer from a `GpuContext`.
    /// 
    /// Note that even if the buffer didn't exist in the provided context, this method overwrites the buffer's id to the unitialized value.
    /// 
    /// To reinitialize the buffer again, call `init()`
    pub fn uninit(&mut self, context: &mut GpuContext) {
        context.remove_buffer(&self.id);
        self.id = BufferId::UNINIT;
    }
}
