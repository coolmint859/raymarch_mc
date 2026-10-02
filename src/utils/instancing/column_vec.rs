use std::{any::Any, cell::{Ref, RefMut}, ops::{Deref, DerefMut}};

use crate::graphics::Serializable;

/// A type erased homogeneous Vector. This is best used in larger collections to allow them to be hetergeneous.
/// 
/// Note: If `T` is a struct that implements `Serializable`, `Vec<T>` automatically implements this trait.
/// 
/// Example Usage: A HashMap where each value is a boxed ColumnVec, allowing each concrete Vec to hold different data types.
pub trait ColumnVec {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the bytes of a single element in the DynVec
    fn bytes_of(&self, index: usize) -> Option<&[u8]>; 

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

impl<T: Serializable + 'static> ColumnVec for Vec<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn bytes_of(&self, index: usize) -> Option<&[u8]> {
        self.get(index).and_then(|val| Some(val.to_bytes()))
    }

    fn len(&self) -> usize { self.len() }
}

/// A mutable reference guard for a `ColumnVec` that is wrapped in a `RefCell`. 
/// 
/// This is needed since `RefMut::try_map()` is not yet stable.
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
/// 
/// This is needed since `RefMut::try_map()` is not yet stable.
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