use std::cell::UnsafeCell;

pub struct SyncUnsafeCell<T: ?Sized>(UnsafeCell<T>);

impl<T: ?Sized> core::ops::Deref for SyncUnsafeCell<T> {
    type Target = UnsafeCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> core::ops::DerefMut for SyncUnsafeCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<T: ?Sized> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }
}
