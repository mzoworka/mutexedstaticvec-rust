#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(generic_arg_infer)]
#[allow(clippy::missing_transmute_annotations)]
pub mod with_locks;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::{ptr, slice};
use tokio::sync::Mutex;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StaticVecError {
    CapacityExceeded,
}

#[derive(Debug)]
pub struct MutexedStaticVec<T, const N: usize> {
    len: Mutex<usize>,
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

impl<T, const N: usize> MutexedStaticVec<T, N> {
    pub fn new(len: usize) -> Result<Self, StaticVecError> {
        if len > N {
            return Err(StaticVecError::CapacityExceeded);
        }
        Ok(Self {
            data: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            len: len.into(),
        })
    }

    pub async fn len(&self) -> usize {
        *self.len.lock().await
    }

    pub async fn is_empty(&self) -> bool {
        *self.len.lock().await == 0
    }

    pub async fn as_slice(&self) -> &[T] {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<&[core::cell::UnsafeCell<core::mem::MaybeUninit<T>>], &[T]>(
                &self.data[..*self.len.lock().await],
            )
        }
    }

    pub async fn as_mut_slice(&mut self) -> &mut [T] {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<&mut [core::cell::UnsafeCell<core::mem::MaybeUninit<T>>], &mut [T]>(
                &mut self.data[..*self.len.lock().await],
            )
        }
    }

    pub async fn iter(&self) -> slice::Iter<'_, T> {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<
                core::slice::Iter<'_, core::cell::UnsafeCell<core::mem::MaybeUninit<T>>>,
                core::slice::Iter<'_, T>,
            >(self.data[..*self.len.lock().await].iter())
        }
    }

    pub async fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        //safe as we ensure that 0..len elements are initialized
        unsafe {
            core::mem::transmute::<
                core::slice::IterMut<'_, core::cell::UnsafeCell<core::mem::MaybeUninit<T>>>,
                core::slice::IterMut<'_, T>,
            >(self.data[..*self.len.lock().await].iter_mut())
        }
    }

    async fn resize_set(&mut self, new_len: usize) {
        *self.len.lock().await = new_len;
    }

    pub async fn push(&self, item: T) -> Result<&T, StaticVecError> {
        let mut len_locked = self.len.lock().await;
        let old_len = *len_locked;
        let ret = unsafe {
            let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(old_len).get();
            el.write(item)
        };
        *len_locked = old_len + 1;

        Ok(ret)
    }

    pub async fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), StaticVecError>
    where
        T: Copy,
    {
        let mut len_locked = self.len.lock().await;
        let old_len = *len_locked;
        let slice = unsafe {
            core::mem::transmute::<&mut [core::cell::UnsafeCell<core::mem::MaybeUninit<T>>], &mut [T]>(
                &mut self.data[..old_len],
            )
        };
        slice[old_len..].copy_from_slice(other);
        *len_locked = old_len + other.len();
        Ok(())
    }

    pub async fn try_extend_from_iter<I: Iterator<Item = T>>(
        &mut self,
        iter: I,
    ) -> Result<(), StaticVecError> {
        let mut len_locked = self.len.lock().await;
        let mut last_item = *len_locked;
        for it in iter {
            unsafe {
                *self.data.get_unchecked_mut(last_item) = MaybeUninit::new(it).into();
            }
            last_item += 1;
        }
        *len_locked = last_item;
        Ok(())
    }

    pub async fn try_extend_from_iter_ref<'a, I: Iterator<Item = &'a T>>(
        &mut self,
        iter: I,
    ) -> Result<(), StaticVecError>
    where
        T: 'a + Clone,
    {
        self.try_extend_from_iter(iter.cloned()).await
    }

    pub async fn from_array<const A: usize>(value: [T; A]) -> Self
    where
        T: Clone,
        [(); N - A]:,
    {
        let mut x: Self = extend_array(value).into();
        x.resize_set(A).await;
        x
    }

    pub async fn remove(&mut self, index: usize) -> T {
        let mut len_locked = self.len.lock().await;
        let len = *len_locked;

        assert!(len > 0);
        assert!(index < len);

        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.data.as_mut_ptr().add(index);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = ptr::read(ptr).into_inner().assume_init();

                // Shift everything down to fill in that spot.
                ptr::copy(ptr.add(1), ptr, len - index - 1);
            }
            *len_locked = len - 1;
            ret
        }
    }
}

impl<T, const N: usize> Default for MutexedStaticVec<T, N> {
    fn default() -> Self {
        Self {
            len: 0.into(),
            data: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
        }
    }
}

impl<'a, T: Clone, const N: usize> From<&'a [T; N]> for MutexedStaticVec<T, N> {
    fn from(value: &'a [T; N]) -> Self {
        Self {
            data: value.clone().map(|x| MaybeUninit::new(x).into()),
            len: N.into(),
        }
    }
}

impl<T, const N: usize> From<[T; N]> for MutexedStaticVec<T, N> {
    fn from(value: [T; N]) -> Self {
        Self {
            data: value.map(|x| MaybeUninit::new(x).into()),
            len: N.into(),
        }
    }
}

impl<T, const N: usize> From<[MaybeUninit<T>; N]> for MutexedStaticVec<T, N> {
    fn from(value: [MaybeUninit<T>; N]) -> Self {
        Self {
            data: value.map(|x| x.into()),
            len: N.into(),
        }
    }
}

impl<T, const N: usize> From<[UnsafeCell<MaybeUninit<T>>; N]> for MutexedStaticVec<T, N> {
    fn from(value: [UnsafeCell<MaybeUninit<T>>; N]) -> Self {
        Self {
            data: value,
            len: N.into(),
        }
    }
}
