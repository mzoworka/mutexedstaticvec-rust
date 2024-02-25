#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(generic_arg_infer)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::AtomicUsize;
use core::{ptr, slice};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StaticVecError {
    CapacityExceeded,
}

#[derive(Debug)]
pub struct AtomicStaticVec<T, const N: usize> {
    len: AtomicUsize,
    data: [UnsafeCell<MaybeUninit<T>>; N],
}

fn extend_array<T, const A: usize, const N: usize>(a: [T; A]) -> [UnsafeCell<MaybeUninit<T>>; N]
where
    T: Clone,
    [(); N]:,
    [(); N - A]:,
{
    let mut ary = core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit()));
    for (idx, val) in a.into_iter().enumerate() {
        ary[idx] = MaybeUninit::new(val).into();
    }
    ary
}

impl<T, const N: usize> AtomicStaticVec<T, N> {
    pub fn new(len: usize) -> Result<Self, StaticVecError> {
        if len > N {
            return Err(StaticVecError::CapacityExceeded);
        }
        Ok(Self {
            data: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            len: len.into(),
        })
    }

    pub fn len(&self) -> usize {
        self.len.load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len.load(core::sync::atomic::Ordering::Relaxed) == 0
    }

    pub fn as_slice(&self) -> &[T] {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<_, &[T]>(
                &self.data[..self.len.load(core::sync::atomic::Ordering::Relaxed)],
            )
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<_, &mut [T]>(
                &mut self.data[..self.len.load(core::sync::atomic::Ordering::Relaxed)],
            )
        }
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<_, core::slice::Iter<'_, T>>(
                self.data[..self.len.load(core::sync::atomic::Ordering::Relaxed)].iter(),
            )
        }
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<_, core::slice::IterMut<'_, T>>(
                self.data[..self.len.load(core::sync::atomic::Ordering::Relaxed)].iter_mut(),
            )
        }
    }

    fn resize_add(&self, add_len: usize) -> Result<usize, StaticVecError> {
        self.len
            .fetch_update(
                core::sync::atomic::Ordering::SeqCst,
                core::sync::atomic::Ordering::SeqCst,
                |old_val| {
                    if old_val + add_len > N {
                        None
                    } else {
                        Some(old_val + add_len)
                    }
                },
            )
            .map_err(|_| StaticVecError::CapacityExceeded)
    }

    fn resize_set(&mut self, new_len: usize) -> Result<(), StaticVecError> {
        self.len
            .store(new_len, core::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub fn push(&self, item: T) -> Result<&T, StaticVecError> {
        let old_len = self.resize_add(1)?;
        Ok(unsafe {
            let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(old_len).get();
            el.write(item)
        })
    }

    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), StaticVecError>
    where
        T: Copy,
    {
        let old_len = self.resize_add(other.len())?;
        self.as_mut_slice()[old_len..].copy_from_slice(other);
        Ok(())
    }

    pub fn try_extend_from_iter<I: Iterator<Item = T>>(
        &mut self,
        iter: I,
    ) -> Result<(), StaticVecError> {
        for it in iter {
            let last_item = self.resize_add(1)?;
            unsafe {
                *self.data.get_unchecked_mut(last_item) = MaybeUninit::new(it).into();
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
        let _ = x.resize_set(A);
        x
    }

    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len.load(core::sync::atomic::Ordering::Relaxed);

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
            let _ = self.resize_set(len - 1);
            ret
        }
    }
}

impl<T, const N: usize> Clone for AtomicStaticVec<T, N>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let src = self.as_slice();
        let mut data = core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit()));
        for i in 0..src.len() {
            data[i] = MaybeUninit::new(src[i].clone()).into();
        }
        Self {
            len: src.len().into(),
            data,
        }
    }
}

impl<T, const N: usize> PartialEq for AtomicStaticVec<T, N>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let a = self.as_slice();
        let b = other.as_slice();

        a.len() == b.len() && (*a == *b)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a AtomicStaticVec<T, N> {
    type Item = &'a T;

    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, const N: usize> Default for AtomicStaticVec<T, N> {
    fn default() -> Self {
        Self {
            len: 0.into(),
            data: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
        }
    }
}

impl<'a, T: Clone, const N: usize> From<&'a [T; N]> for AtomicStaticVec<T, N> {
    fn from(value: &'a [T; N]) -> Self {
        Self {
            data: value.clone().map(|x| MaybeUninit::new(x).into()),
            len: N.into(),
        }
    }
}

impl<T, const N: usize> From<[T; N]> for AtomicStaticVec<T, N> {
    fn from(value: [T; N]) -> Self {
        Self {
            data: value.map(|x| MaybeUninit::new(x).into()),
            len: N.into(),
        }
    }
}

impl<T, const N: usize> From<[MaybeUninit<T>; N]> for AtomicStaticVec<T, N> {
    fn from(value: [MaybeUninit<T>; N]) -> Self {
        Self {
            data: value.map(|x| x.into()),
            len: N.into(),
        }
    }
}

impl<T, const N: usize> From<[UnsafeCell<MaybeUninit<T>>; N]> for AtomicStaticVec<T, N> {
    fn from(value: [UnsafeCell<MaybeUninit<T>>; N]) -> Self {
        Self {
            data: value,
            len: N.into(),
        }
    }
}

impl<T, const N: usize> core::ops::Deref for AtomicStaticVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> core::ops::DerefMut for AtomicStaticVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> core::ops::Index<usize> for AtomicStaticVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        core::ops::Index::index(&**self, index)
    }
}
