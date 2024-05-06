// A mutex wrapper for rcore-fs to ignore poison

#![no_std]

use core::marker::{Sized, Send, Sync};
use sgx_tstd::sync::Mutex as MutexInner;
pub use sgx_tstd::sync::MutexGuard;

use sgx_tstd::sync::RwLock as RwLockInner;
pub use sgx_tstd::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct Mutex<T: ?Sized>(MutexInner<T>);

impl<T> Mutex<T> {
    pub const fn new(t: T) -> Mutex<T> {
        Mutex(MutexInner::new(t))
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock().expect("lock is poisoned")
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.0.try_lock().ok()
    }

    pub fn unlock(guard: MutexGuard<'_, T>) {
        MutexInner::unlock(guard)
    }
}

pub struct RwLock<T: ?Sized>(RwLockInner<T>);

impl<T> RwLock<T> {
    pub const fn new(t: T) -> RwLock<T> {
        RwLock(RwLockInner::new(t))
    }
}

impl<T: ?Sized> RwLock<T> {
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read().expect("lock is poisoned")
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write().expect("lock is poisoned")
    }
}