use std::{cell::{Cell, RefCell}, collections::{HashMap, HashSet}, sync::atomic::{AtomicU32, Ordering}};

use crate::{graphics::{Buffer, BufferId, Graphics, RawBytesUpdate, Serializable, VertexBufferLayout}, utils::{ColumnVec, VecRef, VertexAttribute}};

static GROUP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Represents geometry that have no attributes or data
pub struct Empty;

/// Represents geometry that has attributes/data but are not yet initialized
pub struct Building;

/// Represents geometry that has been initialized
pub struct Initialized {
    /// the id to the vertex/instance buffer on the gpu
    pub buf_id: BufferId,
    /// the total capacity of the group
    pub capacity: u64,
    /// the current length of the group
    pub len: u64,
}

/// A proxy struct for `GeometryData` for allowing mutable borrows of one or more attributes.
pub struct GeometryProxy<'a> {
    inst_data: &'a mut HashMap<String, RefCell<Box<dyn ColumnVec>>>,
    state: &'a mut Initialized,
    max_len: Cell<usize>,
    borrowed: RefCell<HashSet<String>>,
}

impl<'a> GeometryProxy<'a> {
    /// Mutably borrow an attribute from the `GeometryData` that this Batch represents, if exists
    pub fn get_attribute_mut<T>(&self, name: impl Into<String>) -> Option<&mut Vec<T>> 
    where T: Serializable + Default + 'static
    {
        let name_str = name.into();
        if !self.borrowed.borrow_mut().insert(name_str.clone()) {
            return None;
        }

        let attr = self.inst_data.get(&name_str)?;
        
        // Safety: The lifetime marker 'a ensures that the pointer to the attribute data lives as long as the batch does.
        // Batches only ever live in closures, so the batch will eventually be dropped, allowing the pointer to be safely dropped too.
        let attr_ptr = attr.as_ptr() as *mut Box<dyn ColumnVec>;
        let boxed_attr = unsafe { &mut *attr_ptr};

        let vec = boxed_attr.downcast_mut::<Vec<T>>()?;
        self.max_len.set(self.max_len.get().max(vec.len()));

        Some(vec)
    }
}

impl<'a> Drop for GeometryProxy<'a> {
    fn drop(&mut self) {
        let new_len = self.max_len.get().min(self.state.capacity as usize);
        for attr in self.inst_data.values() {
            attr.borrow_mut().resize_default(new_len);
        }
        self.state.len = new_len as u64;
    }
}

/// A container for vertex/instance data
pub struct GeometryData<S> {
    /// buffer label for gpu profiling
    label: String,
    /// The layout of the instance buffer
    layout: VertexBufferLayout,
    /// The cpu-side instance data
    inst_data: HashMap<String, RefCell<Box<dyn ColumnVec>>>,
    /// The names of the instance attributes that should be packed into the instance buffer
    gpu_attrs: Vec<String>,
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
            inst_data: HashMap::new(),
            gpu_attrs: Vec::new(),
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
        T: Serializable + Default + 'static,
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
            inst_data: empty.inst_data,
            gpu_attrs: empty.gpu_attrs,
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
        T: Serializable + Default + 'static,
    {
        for _ in 0..attr.count() {
            self.layout.add_attribute(attr.format());
        }

        self.inst_data.insert(attr.name().into(), RefCell::new(Box::new(data)));
        self.gpu_attrs.push(attr.name().into());

        self
    }

    /// Initialize the geometry data's vertex/instance buffer. This locks in the attributes, preventing more from being added.
    /// 
    /// Resizes all attribute data to be at least as large as the largest attribute, but no larger than `capacity`.
    /// Make sure `capacity` is large enough to prevent any attribute data from being truncated.
    /// 
    /// Any attribute data previously added is uploaded to the buffer.
    pub fn init(self, graphics: &mut Graphics, capacity: u64) -> GeometryData<Initialized> {
        let id_num = GROUP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("geometry_data_{id_num}"));
        let buf_id = BufferId(Box::leak(id));
        
        let mut target_len = 0_u64;
        for attr in self.inst_data.values() {
            target_len = attr.borrow().len().max(target_len as usize) as u64;
        }
        target_len = target_len.min(capacity);

        for attr in self.inst_data.values() {
            attr.borrow_mut().resize_default(target_len as usize);
        }
        
        let state = Initialized {
            capacity,
            len: target_len,
            buf_id,
        };

        let initialized = GeometryData::<Initialized>::from_uninit(self, state);
        let packed = initialized.to_packed();

        let total_bytes = initialized.layout.stride() * capacity;
        graphics.context.request_buffer(
            &initialized.buf_id(), 
            Buffer::as_vertex()
                .with_label(&initialized.label)
                .with_capacity(total_bytes)
                .with_byte_data(&packed)
                .writable()
        );

        initialized
    }
}

impl GeometryData<Initialized> {
    /// Create initialized geomtry data from an uninitialized one. 
    /// 
    /// This is automatically called and returned from `GeometryData::<Building>::init()` 
    pub(crate) fn from_uninit(
        uninit: GeometryData<Building>, 
        state: Initialized,
    ) -> GeometryData<Initialized> {
        Self {
            label: uninit.label,
            layout: uninit.layout,
            inst_data: uninit.inst_data,
            gpu_attrs: uninit.gpu_attrs,
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
            inst_data: HashMap::new(),
            gpu_attrs: Vec::new(),
            state: Initialized { capacity: 0, len: 0, buf_id: BufferId("placeholder") }
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

    /// Borrow the GeometryData mutably -  returns a `GeometryProxy` which references the inner attribute data.
    /// 
    /// Given that any of the borrowed attribute(s) could be resized during a borrow, the returned `GeometryProxy` will 
    /// ensure all other attributes match the length of the largest mutated attribute, unless greater than the capacity.
    pub fn borrow_mut(&mut self) -> GeometryProxy<'_>{
        GeometryProxy {
            inst_data: &mut self.inst_data,
            state: &mut self.state,
            max_len: Cell::new(0),
            borrowed: RefCell::new(HashSet::new())
        }
    }

    /// Get a reference to an attribute vector, if exists
    pub fn get_attribute<T: Serializable + Default + 'static>(&self, name: &str) -> Option<VecRef<'_, T>> {
        let attr = self.inst_data.get(name)?;

        let guard = attr.borrow();
        let data = guard.downcast_ref::<Vec<T>>()? as *const Vec<T>;

        Some(VecRef { _guard: guard, data})
    }

    /// Update the instance buffer this group represents
    pub fn update(&self, graphics: &mut Graphics) {
        let packed = self.to_packed();

        let _ = graphics.context.update_buffer(
            &self.state.buf_id,
            RawBytesUpdate { data: &packed, offset: 0 }
        );
    }

    /// Get the capacity of this instance group
    pub fn capacity(&self) -> usize {
        self.state.capacity as usize
    }

    /// Get the length of this instance group
    pub fn len(&self) -> usize{
        self.state.len as usize
    }

    /// Get this group as a packed `Vec<u8>` (interleaved)
    pub fn to_packed(&self) -> Vec<u8> {
        let mut packed: Vec<u8> = Vec::new();

        for i in 0..self.state.len as usize {
            for name in &self.gpu_attrs {
                if let Some(cell) = self.inst_data.get(name) {
                    let data = cell.borrow();
                    let attr_bytes = data.bytes_of(i)
                        .expect("[GeometryData] Attribute data missing!");
                    
                    packed.extend_from_slice(attr_bytes);
                }
            } 
        }

        packed
    }
}