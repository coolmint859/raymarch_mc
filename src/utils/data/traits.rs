use std::{any::Any, cell::{Ref, RefMut}, fmt::Debug, hash::Hash, ops::{Deref, DerefMut}};

use indexmap::IndexMap;

use crate::graphics::Serializable;

/// Represents homogeneous collections that store elements that are `Serializable`
/// 
/// If `T` is a type that implements `Serializable`, then `Vec<T>` automatically implements this trait.
pub trait ElementSerializable {
    /// Returns the number of elements
    fn elem_count(&self) -> usize;

    /// Converts the element at `idx` into bytes and appends it to `dest`.
    fn write_idx(&self, idx: usize, dest: &mut Vec<u8>);
}

impl<T: Serializable> ElementSerializable for Vec<T> {
    fn elem_count(&self) -> usize {
        self.len()
    }

    fn write_idx(&self, idx: usize, dest: &mut Vec<u8>) {
        if let Some(elem) = self.get(idx) {
            dest.extend_from_slice(&elem.to_bytes());
        }
    }
}

/// Represents map-like collections where individual values are `Serializable`
pub trait BlockSerializable {
    /// The key type for the internal map
    type Key: Hash + Eq + PartialEq;

    /// Get the set of all keys in the map.
    fn keys(&self) -> Vec<Self::Key>;

    /// Convert the value with the provided `key` to a `Vec<u8>`, if exists.
    fn block_bytes(&self, key: &Self::Key) -> Option<Vec<u8>>;

    /// Convert all value entries to a `IndexMap` of `Vec<u8>'s
    fn bytes_all(&self) -> IndexMap<Self::Key, Vec<u8>>;
}

/// A type erased homogeneous `Vec`. This is best used in larger collections to allow them to be hetergeneous.
/// 
/// Note: If `T` is a type that implements `Serializable`, `Vec<T>` automatically implements this trait. 
/// This allows regular `Vec<T>`s to be used in heterogeneous collections without needing to implement `ColumnVec` directly.
/// 
/// Example Usage: A HashMap where each value is a boxed ColumnVec, allowing each concrete `Vec` type to hold different data types.
pub trait ColumnVec: ElementSerializable + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Clear the ColumnVec, removing all values
    fn clear(&mut self);

    /// Returns the number of bytes a single element takes up
    fn elem_bytes(&self) -> usize;

    /// Swaps the last element with the element specified at `index`, then removes the last element.
    /// 
    /// This effectively removes the value stored at `index`, but in O(1) time, at the cost of not preserving order.
    fn swap_remove(&mut self, index: usize);

    /// Resize the `ColumnVec` to have size `new_len`, appending default values if the current length is less than `new_len`. 
    /// 
    /// If the current length is more than `new_len`, then this truncates the `ColumnVec` to have size `new_len`.
    fn resize_default(&mut self, new_len: usize);
}

impl dyn ColumnVec {
    /// Downcast this `ColumnVec` into a reference of the concrete Vec type
    pub fn downcast_ref<V: ColumnVec + 'static>(&self) -> Option<&V> {
        self.as_any().downcast_ref::<V>()
    }

    /// Downcast this `ColumnVec` into a mutable reference of the concrete Vec type
    pub fn downcast_mut<V: ColumnVec + 'static>(&mut self) -> Option<&mut V> {
        self.as_any_mut().downcast_mut::<V>()
    }
}

impl<T> ColumnVec for Vec<T> 
where T: Serializable + Default + Debug + 'static
{
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn elem_bytes(&self) -> usize {
        std::mem::size_of::<T>()
    }

    fn resize_default(&mut self, new_len: usize) {
        self.resize_with(new_len, T::default);
    }

    fn swap_remove(&mut self, index: usize) { 
        self.swap_remove(index); 
    }
    
    fn clear(&mut self) { self.clear(); }
}

/// A reference guard for a mutable `ColumnVec` that is wrapped in a `RefCell`
pub struct VecMut<'a, T> {
    pub _guard: RefMut<'a, Box<dyn ColumnVec>>,

    /// Safety: The reference is valid because `_guard` keeps it alive.
    pub data: *mut Vec<T>
}

impl<'a, T> Deref for VecMut<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> DerefMut for VecMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {&mut *self.data }
    }
}

/// A reference guard for a `ColumnVec` that is wrapped in a `RefCell`.
pub struct VecRef<'a, T> {
    pub _guard: Ref<'a, Box<dyn ColumnVec>>,

    /// Safety: The reference is valid because _guard keeps it alive.
    pub data: *const Vec<T>
}

impl<'a, T> Deref for VecRef<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}