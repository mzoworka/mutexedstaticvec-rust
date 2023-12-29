#![no_std]
use core::slice;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StaticVecError {
    CapacityExceeded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticVec<T, const N: usize> {
    len: usize,
    data: [T;N],
}

impl<T, const N: usize> StaticVec<T, N> {
    pub fn new(len: usize) -> Result<Self, StaticVecError>
    where T: Default + Copy,
    {
        if len > N {
            return Err(StaticVecError::CapacityExceeded);
        }
        Ok(Self {
            data: [T::default();N],
            len,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }


    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data[..self.len]
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.data[..self.len].iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.data[..self.len].iter_mut()
    }

    pub fn resize(&mut self, new_len: usize) -> Result<(), StaticVecError> {
        if new_len > N {
            return Err(StaticVecError::CapacityExceeded);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), StaticVecError>
    where T: Copy,
    {
        self.resize(self.len() + other.len())?;
        self.data[self.len..].copy_from_slice(other);
        Ok(())
    }

    pub fn try_extend_from_iter<I: Iterator<Item = T>>(&mut self, mut iter: I) -> Result<(), StaticVecError>
    {
        while let Some(it) = iter.next() {
            let last_item = self.len();
            self.resize(last_item+1)?;
            unsafe {
                *self.data.get_unchecked_mut(last_item) = it;
            }
        }
        Ok(())
    }

    pub fn try_extend_from_iter_ref<'a, I: Iterator<Item = &'a T>>(&mut self, iter: I) -> Result<(), StaticVecError>
    where T: 'a + Clone,
    {
        self.try_extend_from_iter(iter.cloned())
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a StaticVec<T, N> {
    type Item = &'a T;

    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Default + Copy, const N: usize> Default for StaticVec<T, N> {
    fn default() -> Self {
        Self { len: 0, data: [T::default();N] }
    }
}

impl<'a, T: Clone, const N: usize> From<&'a [T;N]> for StaticVec<T, N>{
    fn from(value: &'a[T;N]) -> Self {
        let this = Self {
            data: value.clone(),
            len: N,
        };

        this
    }
}

impl<T, const N: usize> From<[T;N]> for StaticVec<T, N> {
    fn from(value: [T;N]) -> Self {
        Self {
            data: value,
            len: N,
        }
    }
}

