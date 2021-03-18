// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::{os::unix::prelude::RawFd, ptr};

use crate::core_::Core;
use crate::error::Error;
use crate::loop_::Loop;
use crate::properties::Properties;

#[derive(Debug)]
pub struct Context<T: Loop + Clone> {
    ptr: ptr::NonNull<pw_sys::pw_context>,
    loop_: T,
}

impl<T: Loop + Clone> Context<T> {
    // TODO: properties argument
    pub fn new(loop_: &T) -> Result<Self, Error> {
        let context = unsafe { pw_sys::pw_context_new(loop_.as_ptr(), ptr::null_mut(), 0) };
        let context = ptr::NonNull::new(context).ok_or(Error::CreationFailed)?;

        Ok(Context {
            ptr: context,
            loop_: loop_.clone(),
        })
    }

    fn as_ptr(&self) -> *mut pw_sys::pw_context {
        self.ptr.as_ptr()
    }

    pub fn connect(&self, properties: Option<Properties>) -> Result<Core, Error> {
        let properties = properties.map_or(ptr::null_mut(), |p| p.into_raw());

        unsafe {
            let core = pw_sys::pw_context_connect(self.as_ptr(), properties, 0);
            let ptr = ptr::NonNull::new(core).ok_or(Error::CreationFailed)?;

            Ok(Core::from_ptr(ptr))
        }
    }

    pub fn connect_fd(&self, fd: RawFd, properties: Option<Properties>) -> Result<Core, Error> {
        let properties = properties.map_or(ptr::null_mut(), |p| p.into_raw());

        unsafe {
            let core = pw_sys::pw_context_connect_fd(self.as_ptr(), fd, properties, 0);
            let ptr = ptr::NonNull::new(core).ok_or(Error::CreationFailed)?;

            Ok(Core::from_ptr(ptr))
        }
    }
}

impl<T: Loop + Clone> Drop for Context<T> {
    fn drop(&mut self) {
        unsafe { pw_sys::pw_context_destroy(self.as_ptr()) }
    }
}
