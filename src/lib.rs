#![no_std]
#![feature(maybe_uninit_uninit_array)]
use core::mem::MaybeUninit;
use core::slice;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StaticVecError {
    CapacityExceeded,
}

#[derive(Debug)]
pub struct StaticVec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
}

impl<T, const N: usize> StaticVec<T, N> {
    pub fn new(len: usize) -> Result<Self, StaticVecError> {
        if len > N {
            return Err(StaticVecError::CapacityExceeded);
        }
        Ok(Self {
            data: MaybeUninit::uninit_array(),
            len,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[T] {
        //safe as we ensure that 0..len elements are initialized
        unsafe { core::mem::transmute::<_, &[T]>(&self.data[..self.len]) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        //safe as we ensure that 0..len elements are initialized
        unsafe { core::mem::transmute::<_, &mut [T]>(&mut self.data[..self.len]) }
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        //safe as we ensure that 0..len elements are initialized
        unsafe { core::mem::transmute::<_, core::slice::Iter<'_, T>>(self.data[..self.len].iter()) }
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<_, core::slice::IterMut<'_, T>>(self.data[..self.len].iter_mut())
        }
    }

    fn resize(&mut self, new_len: usize) -> Result<(), StaticVecError> {
        if new_len > N {
            return Err(StaticVecError::CapacityExceeded);
        }
        self.len = new_len;
        Ok(())
    }

    pub fn push(&mut self, item: T) -> Result<(), StaticVecError> {
        let old_len = self.len();
        self.resize(old_len + 1)?;
        self.as_mut_slice()[old_len] = item;
        Ok(())
    }

    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), StaticVecError>
    where
        T: Copy,
    {
        let old_len = self.len();
        self.resize(old_len + other.len())?;
        self.as_mut_slice()[old_len..].copy_from_slice(other);
        Ok(())
    }

    pub fn try_extend_from_iter<I: Iterator<Item = T>>(
        &mut self,
        iter: I,
    ) -> Result<(), StaticVecError> {
        for it in iter {
            let last_item = self.len();
            self.resize(last_item + 1)?;
            unsafe {
                *self.data.get_unchecked_mut(last_item) = MaybeUninit::new(it);
            }
        }
        Ok(())
    }

    pub fn try_extend_from_iter_ref<'a, I: Iterator<Item = &'a T>>(
        &mut self,
        iter: I,
    ) -> Result<(), StaticVecError>
    where
        T: 'a + Clone,
    {
        self.try_extend_from_iter(iter.cloned())
    }
}

impl<T, const N: usize> Clone for StaticVec<T, N>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let src = self.as_slice();
        let mut data = MaybeUninit::uninit_array();
        for i in 0..src.len() {
            data[i] = MaybeUninit::new(src[i].clone());
        }
        Self {
            len: self.len,
            data,
        }
    }
}

impl<T, const N: usize> PartialEq for StaticVec<T, N>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let a = self.as_slice();
        let b = other.as_slice();

        self.len == other.len && (*a == *b)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a StaticVec<T, N> {
    type Item = &'a T;

    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, const N: usize> Default for StaticVec<T, N> {
    fn default() -> Self {
        Self {
            len: 0,
            data: MaybeUninit::uninit_array(),
        }
    }
}

impl<'a, T: Clone, const N: usize> From<&'a [T; N]> for StaticVec<T, N> {
    fn from(value: &'a [T; N]) -> Self {
        Self {
            data: value.clone().map(|x| MaybeUninit::new(x)),
            len: N,
        }
    }
}

impl<T, const N: usize> From<[T; N]> for StaticVec<T, N> {
    fn from(value: [T; N]) -> Self {
        Self {
            data: value.map(|x| MaybeUninit::new(x)),
            len: N,
        }
    }
}
