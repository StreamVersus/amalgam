use std::ops::{Deref, DerefMut};
#[derive(Debug, Default)]
pub struct NSize<T> {
    value: T,
    size: usize,
}

impl<T> NSize<T> {
    pub fn size(&self) -> usize {
        self.size
    }
}

impl<T> NSize<T> {
    pub fn new(value: T, size: usize) -> Self {
        Self { value, size }
    }

    pub fn new_calc(value: T) -> Self {
        Self { value, size: size_of::<T>() }
    }
}

impl<T> Deref for NSize<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for NSize<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> From<Vec<T>> for NSize<Vec<T>> {
    fn from(value: Vec<T>) -> Self {
        let size = size_of::<T>() * value.len();
        Self::new(value, size)
    }
}