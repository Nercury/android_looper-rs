//! From [android docs](http://developer.android.com/reference/android/os/Looper.html):
//!
//! > Class used to run a message loop for a thread. Threads by default do not have a message loop
//! > associated with them; to create one, call prepare() in the thread that is to run the loop, and
//! > then loop() to have it process messages until the loop is stopped.
//!
//! # Library design conventions
//!
//! This library is a minimal safe wrapper for what otherwise be unsafe ffi bindings. There are
//! few conventions used here that are good to know to make the best use of it.
//!
//! ## Safety does not include the correct destruction order
//!
//! While this library may provide the `RAII` wrappers (see bellow), it will not keep
//! track of wrapper dependencies. Doing it at this level would require `Rc` machinery that
//! may cause several problems: force users to use `Rc` or `Arc`, complicate implementation,
//! no longer be zero-cost.
//!
//! Instead, the users are encouraged to build wrappers around these different type kinds (listed
//! bellow) that manage resources as needed for each use case.
//!
//! ### Not-copyable `struct Object` kind
//!
//! This is [RAII](https://en.wikipedia.org/wiki/Resource_Acquisition_Is_Initialization) type
//! that wraps a low-level resource, provides methods to manipulate it and also destroys it when
//! it goes out of scope. It may also have convenience methods `from_handle` as constructor and method
//! `forget` to get back the handle and disable the `RAII`.
//!
//! ### Copyable `struct/enum Object` kind
//!
//! Used for tracking intermediate glue information, usually replaces or wraps cumbersome `C` unions,
//! types or enums.
//!
//! ### Copyable `struct ObjectRef` kind
//!
//! Wraps a low level handle(s) and provides methods to manipulate it/them. Used in situations
//! where the handle lifetime is controlled by another object or it is some kind of global singleton.
//! In such cases the method calls themselves may return errors when called on expired or invalid
//! handle.
//!

extern crate android_looper_sys as ffi;
extern crate libc;

use self::error::{Error, Result};
use std::ptr;
use libc::c_int;

pub use ffi::LooperPrepareOpts;

pub mod error;

pub type LooperHandle = *mut ffi::ALooper;

/**
Reference to a looper.

From [NDK docs](http://developer.android.com/ndk/reference/group___looper.html):

> A looper is the state tracking an event loop for a thread.
> Loopers do not define event structures or other such things;
> rather they are a lower-level facility to attach one or more discrete objects listening for an
> event. An "event" here is simply data available on a file descriptor: each attached object has
> an associated file descriptor, and waiting for "events" means (internally) polling on all of
> these file descriptors until one or more of them have data available.

> A thread can have only one Looper associated with it.
*/
#[derive(Copy, Clone, Debug)]
pub struct LooperRef {
    handle: LooperHandle,
}

impl LooperRef {
    /// Create `LooperRef` from native handle.
    pub fn from_handle(handle: LooperHandle) -> LooperRef {
        LooperRef { handle: handle }
    }

    /// Prepares a looper associated with the calling thread, and returns it.
    /// If the thread already has a looper, it is returned. Otherwise, a new one is created,
    /// associated with the thread, and returned.
    pub fn prepare(opts: LooperPrepareOpts) -> Result<LooperRef> {
        let looper_handle = unsafe { ffi::ALooper_prepare(opts as c_int) };
        if looper_handle.is_null() {
            return Err(Error::PrepareLooperFailed);
        }
        Ok(LooperRef { handle: looper_handle })
    }

    /// Acquire looper to prevent its deletion until `AcquiredLooper` object is dropped.
    pub fn acquire(&self) -> AcquiredLooper {
        AcquiredLooper::from_ref(*self)
    }

    /// Get native looper handle.
    pub fn handle(&self) -> LooperHandle {
        self.handle
    }

    /// Performs all pending callbacks until all data has been consumed.
    ///
    /// Calls `ALooper_pollAll(0, NULL, NULL, NULL)`.
    /// This method is unstable and may be removed in favor of better version.
    pub fn poll_all_blind(&self) {
        unsafe { ffi::ALooper_pollAll(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
    }
}

/// `RAII` acquired looper wrapper.
///
/// This prevents the object from being deleted until this wrapper is dropped.
/// This is only needed to safely hand an ALooper from one thread to another.
pub struct AcquiredLooper {
    handle: LooperHandle,
}

impl AcquiredLooper {
    /// Acquire looper to prevent its deletion until this object is dropped.
    pub fn from_ref(looper: LooperRef) -> AcquiredLooper {
        unsafe { ffi::ALooper_acquire(looper.handle()) }
        AcquiredLooper { handle: looper.handle() }
    }
}

impl Drop for AcquiredLooper {
    fn drop(&mut self) {
        unsafe { ffi::ALooper_acquire(self.handle) }
    }
}
