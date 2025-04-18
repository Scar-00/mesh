#![feature(
    ptr_as_ref_unchecked,
    unsafe_cell_access,
    coerce_unsized,
    unsize,
    downcast_unchecked,
    dispatch_from_dyn
)]

use std::{
    any::Any,
    cell::UnsafeCell,
    marker::Unsize,
    ops::{CoerceUnsized, DispatchFromDyn},
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct SharedInner<T: ?Sized> {
    ref_count: AtomicUsize,
    data: UnsafeCell<T>,
}

/*impl SharedInner<dyn Any> {
    pub fn downcast_unchecked<T: Any>(self) -> SharedInner<T> {
        let SharedInner { ref_count, data } = self;
        SharedInner {
            ref_count,
            data: unsafe { data.downcast_unchecked() },
        }
    }
}*/

pub struct Shared<T: ?Sized> {
    inner: NonNull<SharedInner<T>>,
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = unsafe { self.inner.as_ref() };
        let data = unsafe { inner.data.as_ref_unchecked() };
        f.debug_struct("Shared")
            .field("ref_count", &inner.ref_count)
            .field("data", &data)
            .finish()
    }
}

impl<T> Shared<T> {
    pub fn new(data: T) -> Self {
        let inner = Box::new(SharedInner {
            ref_count: AtomicUsize::new(1),
            data: UnsafeCell::new(data),
        });
        Self {
            inner: unsafe { NonNull::new_unchecked(Box::leak(inner)) },
        }
    }
}

impl<T: ?Sized> Shared<T> {
    pub fn get(&self) -> &T {
        unsafe { self.inner.as_ref().data.as_ref_unchecked() }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { self.inner.as_ref().data.as_mut_unchecked() }
    }
}

impl Shared<dyn Any> {
    pub fn downcast_unchecked<T>(self) -> Shared<T> {
        Shared {
            inner: self.inner.cast(),
        }
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.inner.as_ref().ref_count.fetch_add(1, Ordering::AcqRel);
        }
        Self { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Shared<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_ref() };
        if inner.ref_count.load(Ordering::Acquire) > 1 {
            inner.ref_count.fetch_sub(1, Ordering::AcqRel);
            return;
        }
        unsafe {
            _ = Box::from_raw(self.inner.as_mut());
        }
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Shared<U>> for Shared<T> {}
impl<T, U> DispatchFromDyn<Shared<U>> for Shared<T>
where
    T: Unsize<U> + ?Sized,
    U: ?Sized,
{
}

unsafe impl<T: ?Sized> Send for Shared<T> {}
unsafe impl<T: ?Sized> Sync for Shared<T> {}
