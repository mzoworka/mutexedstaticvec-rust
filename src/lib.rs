#![no_std]
#![allow(incomplete_features)]
#![feature(maybe_uninit_uninit_array)]
#![feature(generic_const_exprs)]
#![feature(generic_arg_infer)]

use core::mem::MaybeUninit;
use core::{ptr, slice};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StaticVecError {
    CapacityExceeded,
}

#[derive(Debug)]
pub struct StaticVec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
}

fn extend_array<T, const A: usize, const N: usize>(a: [T; A]) -> [MaybeUninit<T>; N]
where
    T: Clone,
    [(); N]:,
    [(); N - A]:,
{
    let mut ary = MaybeUninit::uninit_array();
    for (idx, val) in a.into_iter().enumerate() {
        ary[idx] = MaybeUninit::new(val);
    }
    ary
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

    pub fn from_array<const A: usize>(value: [T; A]) -> Self
    where
        T: Clone,
        [(); N - A]:,
    {
        let mut x: Self = extend_array(value).into();
        x.resize(A).unwrap();
        x
    }

    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len;

        assert!(len > 0);
        assert!(index < len);

        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().add(index);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = ptr::read(ptr);

                // Shift everything down to fill in that spot.
                ptr::copy(ptr.add(1), ptr, len - index - 1);
            }
            self.len -= 1;
            ret
        }
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

impl<T, const N: usize> From<[MaybeUninit<T>; N]> for StaticVec<T, N> {
    fn from(value: [MaybeUninit<T>; N]) -> Self {
        Self {
            data: value.map(|x| x),
            len: N,
        }
    }
}

impl<T, const N: usize> core::ops::Deref for StaticVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> core::ops::DerefMut for StaticVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> core::ops::Index<usize> for StaticVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        core::ops::Index::index(&**self, index)
    }
}
