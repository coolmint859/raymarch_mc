use std::{any::Any, cell::{Ref, RefMut}, fmt::Debug, ops::{Deref, DerefMut}};

use crate::graphics::Serializable;

/// A type erased homogeneous `Vec`. This is best used in larger collections to allow them to be hetergeneous.
/// 
/// Note: If `T` is a type that implements `Serializable`, `Vec<T>` automatically implements this trait. 
/// This allows regular `Vec<T>`s to be used in heterogeneous collections without needing to implement `ColumnVec` directly.
/// 
/// Example Usage: A HashMap where each value is a boxed ColumnVec, allowing each concrete `Vec` type to hold different data types.
pub trait ColumnVec: Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the bytes of a single element in the DynVec
    fn bytes_of(&self, index: usize) -> Option<&[u8]>;

    /// Clear the ColumnVec, removing all values
    fn clear(&mut self);

    /// Swaps the last element with the element specified at `index`, then removes the last element.
    /// 
    /// This effectively removes the value stored at `index`, but in O(1) time, at the cost of not preserving order.
    fn swap_remove(&mut self, index: usize);

    /// Resize the `ColumnVec` to have size `new_len`, appending default values if the current length is less than `new_len`. 
    /// 
    /// If the current length is more than `new_len`, then this truncates the `ColumnVec` to have size `new_len`.
    fn resize_default(&mut self, new_len: usize);

    /// Get the length of this DynVec
    fn len(&self) -> usize;
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

    fn bytes_of(&self, index: usize) -> Option<&[u8]> {
        self.get(index).and_then(|val| Some(val.to_bytes()))
    }

    fn resize_default(&mut self, new_len: usize) {
        self.resize_with(new_len, T::default);
    }

    fn swap_remove(&mut self, index: usize) { 
        self.swap_remove(index); 
    }
    
    fn clear(&mut self) { self.clear(); }

    fn len(&self) -> usize { self.len() }
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