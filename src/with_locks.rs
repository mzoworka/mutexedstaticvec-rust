use core::{future::Future, mem::MaybeUninit, ops::DerefMut};

use crate::AtomicStaticVec;

pub trait KeyTrait {
    type Key: Copy + PartialEq;
    fn get_key(&self) -> Self::Key;
}

pub trait OptionMutexTrait<'a> {
    type Item: 'a;
    type ItemMutex;
    type Guard: DerefMut<Target = Option<Self::Item>>;

    fn get_item_lock(&'a self) -> impl Future<Output = Self::ItemMutex>;
    fn lock_item(&'a self) -> impl Future<Output = Self::Guard>;
    fn set_item(&self, val: Option<Self::Item>) -> impl Future<Output = ()>;
    fn take_item(&self) -> impl Future<Output = Option<Self::Item>>;
}

pub trait RemoveWithLocksTrait {
    fn remove_with_locks(&self, index: usize) -> impl Future<Output = ()>;
}

impl<T, const N: usize> RemoveWithLocksTrait for AtomicStaticVec<T, N>
where
    T: KeyTrait + for<'a> OptionMutexTrait<'a>,
{
    async fn remove_with_locks(&self, index: usize) {
        let len = self.len.fetch_sub(1, core::sync::atomic::Ordering::Acquire);

        assert!(len > 0);
        assert!(index < len);

        let last_index = len - 1;

        let last = unsafe {
            let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(last_index).get();
            el.assume_init_mut().take_item().await
        };

        let mut selected = unsafe {
            let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(index).get();
            el.assume_init_mut().lock_item().await
        };

        *selected = last;
    }
}
