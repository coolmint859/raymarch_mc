use std::{cell::RefCell, collections::HashSet, fmt::Debug};

use indexmap::IndexMap;

use crate::{graphics::Serializable, utils::{BlockSerializable, ColumnVec, ElementSerializable, VecMut, VecRef}};

/// A proxy struct for `AttributeTable` for allowing mutable borrows of one or more attributes.
pub struct AttributeTableProxy<'a> {
    /// Reference to the attribute data held by a `AttributeTable`
    attributes: &'a mut IndexMap<String, RefCell<Box<dyn ColumnVec>>>,
    /// The length of any given attribute
    len: &'a mut usize,
    /// The set of currently mutably borrowed attributes
    borrowed: RefCell<HashSet<String>>,
}

impl<'a> AttributeTableProxy<'a> {
    /// Get a mutable reference to an attribute, if exists
    pub fn get_attribute_mut<T>(&self, name: impl Into<String>) -> Option<VecMut<'_, T>> 
    where T: Serializable + Default + Debug + 'static
    {
        let name_str = name.into();
        if !self.borrowed.borrow_mut().insert(name_str.clone()) {
            return None;
        }

        let attr = self.attributes.get(&name_str)?;

        let mut guard = attr.borrow_mut();
        let data = guard.downcast_mut::<Vec<T>>()? as *mut Vec<T>;

        Some(VecMut { _guard: guard, data})
    }

    /// Get a reference to an attribute, if exists
    pub fn get_attribute<T>(&self, name: impl Into<String>) -> Option<VecRef<'_, T>> 
    where T: Serializable + Default + Debug + 'static
    {
        let name_str = name.into();
        if self.borrowed.borrow().contains(&name_str) {
            return None; // disallow borrows on attributes that are already mutably borrowed
        }
        
        let attr = self.attributes.get(&name_str)?;
        let guard = attr.borrow();
        let data = guard.downcast_ref::<Vec<T>>()? as *const Vec<T>;

        Some(VecRef { _guard: guard, data})
    }

    /// Extend an attribute from an iterator of `T` items
    pub fn extend_attribute<T>(&self, name: impl Into<String>, iter: impl IntoIterator<Item = T>)
    where T: Serializable + Default + Debug + 'static
    {
        if let Some(mut vec) = self.get_attribute_mut::<T>(name) {
            vec.extend(iter);
        }
    }
}

impl<'a> Drop for AttributeTableProxy<'a> {
    fn drop(&mut self) {
        let mut attr_len = 0;
        for attr in self.attributes.values() {
            attr_len = attr_len.max(attr.borrow().elem_count());
        }

        for attr in self.attributes.values() {
            attr.borrow_mut().resize_default(attr_len);
        }

        *self.len = attr_len;
    }
}

/// A container for homogenenous elements that have heterogeneous values, stored in an ordered table-like format.
/// 
/// All elements are normalized to have the same number of attributes.
/// 
/// Allows modification of multiple attributes through a Proxy container. (see `borrow_mut()`)
#[derive(Debug)]
pub struct AttributeTable {
    /// The cpu-side attribute data
    attributes: IndexMap<String, RefCell<Box<dyn ColumnVec>>>,
    /// The normalized length of the attributes
    len: usize,
}

impl AttributeTable {
    /// Create a new `AttributeTable`
    pub fn new() -> Self {
        Self {
            attributes: IndexMap::new(),
            len: 0,
        }
    }

    /// Add an attribute to the `AttributeTable`
    pub fn with_attribute<T>(mut self, name: impl Into<String>, data: Vec<T>) -> Self
    where
        T: Serializable + Default + Debug + 'static,
    {
        self.add_attribute(name, data);
        self
    }

    /// Add an attribute to the `AttributeTable`
    pub fn add_attribute<T>(&mut self, name: impl Into<String>, data: Vec<T>)
    where
        T: Serializable + Default + Debug + 'static,
    {
        self.len = self.len.max(data.len());
        self.attributes.insert(name.into(), RefCell::new(Box::new(data)));
    }

    /// Borrow the `AttributeTable` mutably -  returns a `AttributeTableProxy` which references the inner attribute data.
    /// 
    /// Given that any of the borrowed attribute(s) could be resized during a borrow, the returned `AttributeTableProxy` will 
    /// ensure all other attributes match the length of the largest mutated attribute.
    pub fn borrow_mut(&mut self) -> AttributeTableProxy<'_>{
        AttributeTableProxy {
            attributes: &mut self.attributes,
            len: &mut self.len,
            borrowed: RefCell::new(HashSet::new())
        }
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
        let attr = self.attributes.get(name)?;

        let guard = attr.borrow();
        let data = guard.downcast_ref::<Vec<T>>()? as *const Vec<T>;

        Some(VecRef { _guard: guard, data})
    }

    /// Resize the `AttributeTable` to have `new_len` number of elements for each attribute.
    /// 
    /// If `new_len` is greater than the current length, this appends default elements to the end of the attributes up to `new_len`.
    /// 
    /// If `new_len` is less than the current length, this truncates the attributes to match `new_len`.
    pub fn resize_default(&mut self, new_len: usize) {
        for attr in self.attributes.values_mut() {
            attr.borrow_mut().resize_default(new_len);
        }
    }

    /// Resize the `AttributeTable` to have at most `capacity` number of elements.
    /// 
    /// If the largest attribute has less elements than `capacity`, then the table is only resized up to that largest attribute.
    pub fn resize_with_capacity(&mut self, capacity: usize) {
        let new_len = self.elem_count().min(capacity);
        self.resize_default(new_len);
    }

    /// Get the number of attributes defined with this `AttributeTable`
    pub fn attr_count(&self) -> u32 {
        self.attributes.len() as u32
    }

    /// Serialize the table attributes into a single interleaved `Vec<u8>`.
    pub fn serialize_interleaved(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for i in 0..self.elem_count() {
            self.write_idx(i, &mut bytes);
        }
        bytes
    }

    /// Returns the byte stride of a single element in the table
    pub fn element_stride(&self) -> usize {
        self.attributes.values()
            .map(|cell| cell.borrow().elem_bytes())
            .sum()
    }
}

impl ElementSerializable for AttributeTable {
    fn elem_count(&self) -> usize {
        self.len
    }

    fn write_idx(&self, idx: usize, dest: &mut Vec<u8>) {
        for attr in self.attributes.values() {
            attr.borrow().write_idx(idx, dest);
        }
    }
}

impl BlockSerializable for AttributeTable {
    type Key = String;

    fn keys(&self) -> Vec<Self::Key> {
        self.attributes.keys()
            .into_iter()
            .map(|key| key.clone())
            .collect()
    }

    fn block_bytes(&self, key: &Self::Key) -> Option<Vec<u8>> {
        let cell = self.attributes.get(key)?;
        let attr = cell.borrow();

        let mut bytes = Vec::new();
        for i in 0..attr.elem_count() {
            attr.write_idx(i, &mut bytes);
        }

        Some(bytes)
    }

    fn bytes_all(&self) -> IndexMap<Self::Key, Vec<u8>> {
        let mut byte_map = IndexMap::new();
        
        for key in self.keys() {
            let bytes = self.block_bytes(&key).unwrap();
            byte_map.insert(key, bytes);
        }

        byte_map
    }
}