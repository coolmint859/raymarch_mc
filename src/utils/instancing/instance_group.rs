use std::{cell::RefCell, collections::HashMap, marker::PhantomData, sync::atomic::{AtomicU32, Ordering}};

use crate::{graphics::{Buffer, BufferId, Graphics, RawBytesUpdate, Serializable, VertexBufferLayout}, utils::{ColumnVec, VertexAttribute, VecMut, VecRef}};

static GROUP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Represents instance groups that have no attributes or data
pub struct Empty;
/// Represents instance groups that have attributes/data but are not yet initialized
pub struct Building;
/// Represents instance groups that have been initialized
pub struct Initialized;

/// Instance data for renderable entities
pub struct InstanceGroup<State> {
    /// The id to the instance buffer
    buf_id: BufferId,
    /// buffer label for gpu profiling
    label: String,
    /// The layout of the instance buffer
    layout: VertexBufferLayout,
    /// The cpu-side instance data
    inst_data: HashMap<String, RefCell<Box<dyn ColumnVec>>>,
    /// The names of the instance attributes that should be packed into the instance buffer
    gpu_attrs: Vec<String>,
    /// The total number of allowable instances
    capacity: u64,
    /// marker for group state
    _state: PhantomData<State>
}

impl InstanceGroup<Empty> {
    /// Create a new `InstanceGroup`
    /// 
    /// `start_loc` - the starting slot location in the shader for which the instance buffer will bind to
    /// `capacity` - the total number of instances this group can hold.
    pub fn new(start_loc: u32, capacity: u64) -> Self {
        let id_num = GROUP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("instance_group_{id_num}"));

        Self {
            buf_id: BufferId(Box::leak(id)),
            label: "instance_buffer".to_string(),
            layout: VertexBufferLayout::as_instance_step(start_loc),
            inst_data: HashMap::new(),
            gpu_attrs: Vec::new(),
            capacity,
            _state: std::marker::PhantomData
        }
    }

    /// Set the label of the instance buffer this group represents
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add an attribute to the instance group
    /// 
    /// This fills the internal attribute `Vec<T>` with any missing data up to the specified capacity, ensuring that at least some data exists in the buffer.
    /// (This is why `T` must implement `Default`)
    pub fn with_attribute<A, T>(mut self, attr: A, mut data: Vec<T>) -> InstanceGroup<Building>
    where
        A: VertexAttribute + 'static,
        T: Serializable + Default + 'static,
    {
        for _ in 0..attr.count() {
            self.layout.add_attribute(attr.format());
        }

        data.resize_with(self.capacity as usize, T::default);
        self.inst_data.insert(attr.name().into(), RefCell::new(Box::new(data)));
        self.gpu_attrs.push(attr.name().into());

        InstanceGroup::<Building>::from_empty(self)
    }
}

impl InstanceGroup<Building> {
    /// Create an instance group from a previously empty one. 
    /// 
    /// This is automatically called and returned from `InstanceGroup::<Empty>::with_attribute()` 
    pub(crate) fn from_empty(empty: InstanceGroup<Empty>) -> Self {
        Self {
            buf_id: empty.buf_id,
            label: empty.label,
            layout: empty.layout,
            inst_data: empty.inst_data,
            gpu_attrs: empty.gpu_attrs,
            capacity: empty.capacity,
            _state: std::marker::PhantomData
        }
    }

    /// Set the label of the instance buffer this group represents
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add an attribute to the instance group
    /// 
    /// This fills the internal attribute `Vec<T>` with any missing data up to the specified capacity, ensuring that at least some data exists in the buffer.
    /// (This is why `T` must implement `Default`)
    pub fn with_attribute<A, T>(mut self, attr: A, mut data: Vec<T>) -> Self
    where
        A: VertexAttribute + 'static,
        T: Serializable + Default + 'static,
    {
        for _ in 0..attr.count() {
            self.layout.add_attribute(attr.format());
        }

        data.resize_with(self.capacity as usize, T::default);
        self.inst_data.insert(attr.name().into(), RefCell::new(Box::new(data)));
        self.gpu_attrs.push(attr.name().into());

        self
    }

    /// Initialize the instance group. This requests the buffer that the instance data will live on.
    pub fn init(self, graphics: &mut Graphics) -> InstanceGroup<Initialized> {
        let total_bytes = self.layout.stride() * self.capacity;

        graphics.context.request_buffer(
            &self.buf_id, 
            Buffer::as_vertex()
                .with_label(&self.label)
                .with_capacity(total_bytes)
                .writable()
        );

        InstanceGroup::<Initialized>::from_uninit(self)
    }
}

impl InstanceGroup<Initialized> {
    /// Create an initialized instance group from an uninitialized one. 
    /// 
    /// This is automatically called and returned from `InstanceGroup::<Building>::init()` 
    pub(crate) fn from_uninit(uninit: InstanceGroup<Building>) -> InstanceGroup<Initialized> {
        Self {
            buf_id: uninit.buf_id,
            label: uninit.label,
            layout: uninit.layout,
            inst_data: uninit.inst_data,
            gpu_attrs: uninit.gpu_attrs,
            capacity: uninit.capacity,
            _state: std::marker::PhantomData
        }
    }

    /// Create an initialized instance group placeholder.
    /// 
    /// Note: this is to help in architectures where intialization happens lazily, but otherwise the group is stored persistently.
    /// It is recommended to initialize the group as normal before using it, as the gpu buffer that the group represents won't be requested otherwise.
    pub fn placeholder() -> Self {
        Self {
            buf_id: BufferId("empty_instance_group"),
            label: "empty_instance_buffer".to_string(),
            layout: VertexBufferLayout::as_instance_step(0),
            inst_data: HashMap::new(),
            gpu_attrs: Vec::new(),
            capacity: 0,
            _state: std::marker::PhantomData
        }
    }

    /// Get the id to the instance buffer of this group
    pub fn buf_id(&self) -> &BufferId {
        &self.buf_id
    }

    /// Get a reference to this instance group's layout
    pub fn layout(&self) -> &VertexBufferLayout {
        &self.layout
    }

    /// Get a mutable reference to an attribute vector, if exists
    pub fn get_attribute_mut<T: Serializable + 'static>(&self, name: &str) -> Option<VecMut<'_, T>> {
        let attr = self.inst_data.get(name)?;

        let mut guard = attr.borrow_mut();
        let data = guard.downcast_mut::<Vec<T>>()? as *mut Vec<T>;

        Some(VecMut { _guard: guard, data})
    }

    /// Get a reference to an attribute vector, if exists
    pub fn get_attribute<T: Serializable + 'static>(&self, name: &str) -> Option<VecRef<'_, T>> {
        let attr = self.inst_data.get(name)?;

        let guard = attr.borrow();
        let data = guard.downcast_ref::<Vec<T>>()? as *const Vec<T>;

        Some(VecRef { _guard: guard, data})
    }

    /// Update the instance buffer this group represents
    pub fn update(&mut self, graphics: &mut Graphics) {
        let mut packed: Vec<u8> = Vec::new();

        for i in 0..self.capacity as usize {
            for name in &self.gpu_attrs {
                if let Some(cell) = self.inst_data.get(name) {
                    let data = cell.borrow();
                    let attr_bytes = data.bytes_of(i).unwrap();
                    packed.extend_from_slice(attr_bytes);
                }
            }
        }

        let _ = graphics.context.update_buffer(
            &self.buf_id,
            RawBytesUpdate { data: &packed, offset: 0 }
        );
    }

    /// Get the capacity of this instance group
    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }
}