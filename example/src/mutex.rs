use core::cell::RefCell;
use core::ops::DerefMut;
use critical_section::with;

pub struct Mutex<T> {
    inner: RefCell<T>,
}

impl<T> Mutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: RefCell::new(inner),
        }
    }

    /// Exclusive mutable access to inner value
    pub fn access<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        with(|_cs| {
            f(self.inner.borrow_mut().deref_mut());
        })
    }

    /// Replaces the inner value
    pub fn replace(&self, value: T) {
        with(|_cs| {
            self.inner.replace(value);
        });
    }
}

impl<T: Clone> Mutex<T> {
    /// Returns a copy of the inner value
    pub fn clone_inner(&self) -> T {
        with(|_cs| self.inner.borrow().clone())
    }
}

unsafe impl<T> Sync for Mutex<T> {}
