use core::{future::Future, mem::MaybeUninit, ops::DerefMut};

use crate::MutexedStaticVec;

pub trait KeyTrait {
    type Key: Copy + PartialEq;
    fn get_key(&self) -> Self::Key;
    fn set_key(&mut self, key: Self::Key);
}

pub trait OptionMutexTrait<'a> {
    type Item: 'a;
    type ItemMutex;
    type Guard: DerefMut<Target = Option<Self::Item>>;

    fn get_item_lock(&'a self) -> &'a Self::ItemMutex;
    fn lock_item(&'a self) -> impl Future<Output = Self::Guard>;
    fn set_item(&self, val: Option<Self::Item>) -> impl Future<Output = ()>;
    fn take_item(&self) -> impl Future<Output = Option<Self::Item>>;
}

pub trait RemoveWithLocksTrait<'a, T: KeyTrait + OptionMutexTrait<'a>> {
    fn remove_with_locks<
        KP: Fn(&T::Key) -> bool,
        IP: Fn(&<T as OptionMutexTrait<'_>>::Item) -> bool,
    >(
        &self,
        key_pred: KP,
        item_pred: IP,
    ) -> impl Future<Output = bool>;
}

impl<'a, T, const N: usize> RemoveWithLocksTrait<'a, T> for MutexedStaticVec<T, N>
where
    T: KeyTrait + OptionMutexTrait<'a> + 'a,
{
    async fn remove_with_locks<
        KP: Fn(&T::Key) -> bool,
        IP: Fn(&<T as OptionMutexTrait<'_>>::Item) -> bool,
    >(
        &self,
        key_pred: KP,
        item_pred: IP,
    ) -> bool {
        let mut len_locked = self.len.lock().await;
        let len = *len_locked;

        assert!(len > 0);

        for i in 0..len {
            let last_index = len - 1;
            let item = unsafe { (*self.data.get_unchecked(i).get()).assume_init_ref() };
            if !key_pred(&item.get_key()) {
                continue;
            }

            if i != last_index {
                let last_key = {
                    let mut selected = unsafe {
                        let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(i).get();
                        el.assume_init_mut().lock_item().await
                    };

                    match selected.as_ref() {
                        Some(item) => {
                            if !item_pred(item) {
                                continue;
                            }
                        }
                        None => continue,
                    }

                    let (last, last_key) = unsafe {
                        let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(last_index).get();
                        (el.assume_init_mut().take_item().await, el.assume_init_mut().get_key())
                    };

                    *selected = last;
                    last_key
                };
                let selected_parent = unsafe {
                    let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(i).get();
                    el.assume_init_mut()
                };
                selected_parent.set_key(last_key);
            } else {
                let mut selected = unsafe {
                    let el: &mut MaybeUninit<T> = &mut *self.data.get_unchecked(i).get();
                    el.assume_init_mut().lock_item().await
                };

                match selected.as_ref() {
                    Some(item) => {
                        if !item_pred(item) {
                            continue;
                        }
                    }
                    None => continue,
                }

                *selected = None;
            }

            *len_locked = len - 1;
        }

        false
    }
}
