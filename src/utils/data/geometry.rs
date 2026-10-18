use std::fmt::Debug;

use crate::{graphics::{Buffer, BufferId, BufferSource, GpuContext, RawBytesUpdate, Serializable, VertexBufferLayout}, utils::{AttributeTable, AttributeTableProxy, ElementSerializable, VecRef}};

/// Represents attributes in a vertex/instance buffer
pub trait VertexAttribute {
    /// Get the name of this attribute
    fn name(&self) -> impl Into<String>;

    /// The number of slots this attribute requires (default is 1).
    fn count(&self) -> u32 { 1 }

    /// The format of this attribute.
    fn format(&self) -> wgpu::VertexFormat;
}

/// A float vertex attribute. `f32` in shader
pub struct FloatAttribute(pub &'static str);
impl VertexAttribute for FloatAttribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32 }
}

/// A 2d vector vertex attribute. `vec2<f32>` in shader
pub struct Vec2Attribute(pub &'static str);
impl VertexAttribute for Vec2Attribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }
}

/// A 3d vector vertex attribute. `vec3<f32>` in shader
pub struct Vec3Attribute(pub &'static str);
impl VertexAttribute for Vec3Attribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }
}

/// A 4d vector vertex attribute. `vec4<f32>` in shader
pub struct Vec4Attribute(pub &'static str);
impl VertexAttribute for Vec4Attribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

/// Four 4d-vectors, each `vec4<f32>` in shader 
pub struct TransformAttribute(pub &'static str);
impl VertexAttribute for TransformAttribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn count(&self) -> u32 { 4 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

/// Represents geometry that have no attributes or data
#[derive(Debug)]
pub struct Empty;

/// Represents geometry that has attributes/data but are not yet initialized
#[derive(Debug)]
pub struct Building;

/// Represents a buffer that has been initialized
#[derive(Debug)]
pub struct BufferInit {
    /// the id to the buffer on the gpu
    pub buf_id: BufferId,
    /// the total element capacity of the buffer
    pub capacity: usize,
    /// the byte stride of a single element in the buffer
    pub stride: usize
}

/// A container for vertex/instance data
#[derive(Debug)]
pub struct GeometryData<S> {
    /// buffer label for gpu profiling
    label: String,
    /// The layout of the instance buffer
    layout: VertexBufferLayout,
    /// The cpu-side attribute data
    attributes: AttributeTable,
    /// the current state of the group
    state: S
}

impl GeometryData<Empty> {
    /// Create a new `GeometryData`
    /// 
    /// `start_loc` - the starting slot location in the shader for which the buffer will bind to
    /// `step` - the wgpu step mode of the gpu buffer
    fn new(start_loc: u32, step: wgpu::VertexStepMode) -> Self {
        let layout = match step {
            wgpu::VertexStepMode::Vertex => VertexBufferLayout::as_vertex_step(start_loc),
            wgpu::VertexStepMode::Instance => VertexBufferLayout::as_instance_step(start_loc)
        };

        Self {
            label: "geometry_data".to_string(),
            layout,
            attributes: AttributeTable::new(),
            state: Empty
        }
    }

    /// Create geometry data for a vertex buffer
    pub fn as_vertex_group(start_loc: u32) -> Self {
        GeometryData::new(start_loc, wgpu::VertexStepMode::Vertex)
    }

    /// Create geometry data for an instance buffer
    pub fn as_instance_group(start_loc: u32) -> Self {
        GeometryData::new(start_loc, wgpu::VertexStepMode::Instance)
    }

    /// Set the label of the buffer this group represents
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add an attribute to the geometry data
    pub fn with_attribute<A, T>(self, attr: A, data: Vec<T>) -> GeometryData<Building>
    where
        A: VertexAttribute + 'static,
        T: Serializable + Default + Debug + 'static,
    {
        let builder = GeometryData::<Building>::from_empty(self);
        builder.with_attribute(attr, data)
    }
}

impl GeometryData<Building> {
    /// Create a "currently being built" geometry data from a previously empty one. 
    /// 
    /// This is automatically called and returned from `GeometryData::<Empty>::with_attribute()` 
    pub(crate) fn from_empty(empty: GeometryData<Empty>) -> Self {
        Self {
            label: empty.label,
            layout: empty.layout,
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
    pub fn with_attribute<A, T>(mut self, attr: A, data: Vec<T>) -> Self
    where
        A: VertexAttribute + 'static,
        T: Serializable + Default + Debug + 'static,
    {
        for _ in 0..attr.count() {
            self.layout.add_attribute(attr.format());
        }

        self.attributes.add_attribute(attr.name().into(), data);
        self
    }

    /// Initialize the `GeometryData` and create the inner vertex/instance buffer with a `GpuContext`. This locks in the attributes, preventing more from being added.
    /// 
    /// Resizes all attribute data to be at least as large as the largest attribute, but no larger than `capacity`.
    /// Make sure `capacity` is large enough to prevent any attribute data from being truncated.
    /// 
    /// Any attribute data previously added is uploaded to the buffer. If `capacity` is less than the length, defaults are uploaded.
    pub fn init(mut self, context: &mut GpuContext, capacity: usize) -> GeometryData<BufferInit> {
        self.attributes.resize_with_capacity(capacity);
        let packed_bytes = self.attributes.serialize_interleaved();

        let stride = self.layout.stride() as usize;
        let total_bytes = (stride * capacity) as u64;

        let buf_id = context.request_buffer(
            Buffer::as_vertex().with_label(&self.label).writable(),
            BufferSource::from_raw(packed_bytes).with_capacity(total_bytes)
        );
        
        let state = BufferInit {
            capacity,
            buf_id,
            stride
        };

        GeometryData::<BufferInit>::from_uninit(self, state)
    }
}

impl GeometryData<BufferInit> {
    /// Create initialized geometry data from an uninitialized one. 
    /// 
    /// This is automatically called and returned from `GeometryData::<Building>::init()` 
    pub(crate) fn from_uninit(
        uninit: GeometryData<Building>, 
        state: BufferInit,
    ) -> GeometryData<BufferInit> {
        Self {
            label: uninit.label,
            layout: uninit.layout,
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
            layout: VertexBufferLayout::as_instance_step(0),
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

    /// Get a reference to this instance group's layout
    pub fn layout(&self) -> &VertexBufferLayout {
        &self.layout
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

    /// Get the number of attributes in this `GeometryData`
    pub fn attr_count(&self) -> u32 {
        self.attributes.attr_count() as u32
    }

    pub fn clear(&mut self) {
        self.attributes.clear()
    }
}