use std::fmt::Debug;

use crate::{graphics::{Buffer, BufferId, GpuContext, RawBytesUpdate, Serializable}, utils::{AttributeTable, AttributeTableProxy, BufferInit, Building, ElementSerializable, Empty, VecRef}};

/// A container for storage buffer data
#[derive(Debug)]
pub struct StorageBuffer<S> {
    /// buffer label for gpu profiling
    label: String,
    /// The cpu-side attribute data
    attributes: AttributeTable,
    /// the current state of the group
    state: S
}

impl StorageBuffer<Empty> {
    /// Create a new `GeometryData`
    /// 
    /// `start_loc` - the starting slot location in the shader for which the buffer will bind to
    /// `step` - the wgpu step mode of the gpu buffer
    pub fn new() -> Self {
        Self {
            label: "geometry_data".to_string(),
            attributes: AttributeTable::new(),
            state: Empty
        }
    }

    /// Set the label of the buffer this group represents
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add an attribute to the geometry data
    pub fn with_attribute<T>(self, name: impl Into<String>, data: Vec<T>) -> StorageBuffer<Building>
    where
        T: Serializable + Default + Debug + 'static,
    {
        let builder = StorageBuffer::<Building>::from_empty(self);
        builder.with_attribute(name, data)
    }
}

impl StorageBuffer<Building> {
    /// Create a "currently being built" geometry data from a previously empty one. 
    /// 
    /// This is automatically called and returned from `GeometryData::<Empty>::with_attribute()` 
    pub(crate) fn from_empty(empty: StorageBuffer<Empty>) -> Self {
        Self {
            label: empty.label,
            attributes: empty.attributes,
            state: Building
        }
    }

    /// Set the label of the buffer this group represents
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add an attribute to the geometry data
    pub fn with_attribute<T>(mut self, name: impl Into<String>, data: Vec<T>) -> Self
    where
        T: Serializable + Default + Debug + 'static,
    {
        self.attributes.add_attribute(name.into(), data);
        self
    }

    /// Initialize the `GeometryData` and create the inner vertex/instance buffer with a `GpuContext`. This locks in the attributes, preventing more from being added.
    /// 
    /// Resizes all attribute data to be at least as large as the largest attribute, but no larger than `capacity`.
    /// Make sure `capacity` is large enough to prevent any attribute data from being truncated.
    /// 
    /// Any attribute data previously added is uploaded to the buffer. If `capacity` is less than the length, defaults are uploaded.
    pub fn init(mut self, context: &mut GpuContext, capacity: usize) -> StorageBuffer<BufferInit> {
        self.attributes.resize_with_capacity(capacity);
        let packed_bytes = self.attributes.serialize_interleaved();

        let stride = self.attributes.element_stride();
        let total_bytes = stride * capacity;

        let buf_id = context.request_buffer(
            Buffer::as_storage()
                .with_label(&self.label)
                .with_byte_data(&packed_bytes)
                .with_capacity(total_bytes as u64)
                .writable()
        );
        
        let state = BufferInit {
            capacity,
            buf_id,
            stride
        };

        StorageBuffer::<BufferInit>::from_uninit(self, state)
    }
}

impl StorageBuffer<BufferInit> {
    /// Create initialized geometry data from an uninitialized one. 
    /// 
    /// This is automatically called and returned from `GeometryData::<Building>::init()` 
    pub(crate) fn from_uninit(
        uninit: StorageBuffer<Building>, 
        state: BufferInit,
    ) -> StorageBuffer<BufferInit> {
        Self {
            label: uninit.label,
            attributes: uninit.attributes,
            state
        }
    }

    /// Create an pseudo-initialized `GeometryData` placeholder.
    /// 
    /// Note: this is to help in architectures where intialization happens lazily, but otherwise the group is stored persistently.
    /// It is recommended to initialize the data as normal before using it, as the gpu buffer that the data resides in won't be requested otherwise.
    pub fn placeholder() -> Self {
        Self {
            label: "Placeholder Geometry".to_string(),
            attributes: AttributeTable::new(),
            state: BufferInit { 
                buf_id: BufferId::UNINIT, 
                capacity: 0, 
                stride: 0 
            }
        }
    }

    /// Get the id to the instance buffer of this group
    pub fn buf_id(&self) -> &BufferId {
        &self.state.buf_id
    }

    /// Borrow the `GeometryData` mutably -  returns a `AttributeTableProxy` which references the inner attribute data.
    /// 
    /// Given that any of the borrowed attribute(s) could be resized during a borrow, the returned `AttributeTableProxy` will 
    /// ensure all other attributes match the length of the largest mutated attribute.
    pub fn borrow_mut(&mut self) -> AttributeTableProxy<'_>{
        self.attributes.borrow_mut()
    }

    /// Extend an attribute from an iterator of `T` items
    pub fn extend_attribute<T>(&mut self, name: impl Into<String>, iter: impl IntoIterator<Item = T>)
    where T: Serializable + Default + Debug + 'static
    {
        self.borrow_mut().extend_attribute(name, iter);
    }

    /// Get a reference to an attribute vector, if exists
    pub fn get_attribute<T>(&self, name: &str) -> Option<VecRef<'_, T>> 
    where T: Serializable + Default + Debug + 'static
    {
        self.attributes.get_attribute::<T>(name)
    }

    /// Update the instance buffer this group represents
    pub fn update(&mut self, context: &mut GpuContext) {
        self.attributes.resize_with_capacity(self.state.capacity);

        let packed_bytes = self.attributes.serialize_interleaved();
        let _ = context.update_buffer(
            &self.state.buf_id,
            RawBytesUpdate { data: &packed_bytes, offset: 0 }
        );
    }

    /// Get the capacity of this instance group
    pub fn capacity(&self) -> usize {
        self.state.capacity as usize
    }

    /// Get the number of elements (vertices/instances) in this `GeometryData`
    pub fn elem_count(&self) -> usize{
        self.attributes.elem_count()
    }

    /// Get the byte stride of the elements in the buffer
    pub fn elem_stride(&self) -> usize {
        self.attributes.element_stride()
    }

    /// Get the number of attributes in this `GeometryData`
    pub fn attr_count(&self) -> u32 {
        self.attributes.attr_count() as u32
    }
}