// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::ptr;

use libc::{c_int, c_void};
use signal::Signal;
use spa::{result::SpaResult, spa_interface_call_method};

use crate::utils::assert_main_thread;

/// A trait for common functionality of the different pipewire loop kinds, most notably [`MainLoop`](`crate::MainLoop`).
///
/// Different kinds of events, such as receiving a signal (e.g. SIGTERM) can be attached to the loop using this trait.
pub unsafe trait Loop {
    fn as_ptr(&self) -> *mut pw_sys::pw_loop;

    #[must_use]
    fn add_signal_local<F>(&self, signal: Signal, callback: F) -> SignalSource<F, Self>
    where
        F: Fn() + 'static,
        Self: Sized,
    {
        assert_main_thread();

        unsafe extern "C" fn call_closure<F>(data: *mut c_void, _signal: c_int)
        where
            F: Fn(),
        {
            let callback = (data as *mut F).as_ref().unwrap();
            callback();
        }

        let data = Box::into_raw(Box::new(callback));

        let (source, data) = unsafe {
            let mut iface = self
                .as_ptr()
                .as_ref()
                .unwrap()
                .utils
                .as_ref()
                .unwrap()
                .iface;

            let source = spa_interface_call_method!(
                &mut iface as *mut spa_sys::spa_interface,
                spa_sys::spa_loop_utils_methods,
                add_signal,
                signal as c_int,
                Some(call_closure::<F>),
                data as *mut _
            );

            (source, Box::from_raw(data))
        };

        let ptr = ptr::NonNull::new(source).expect("source is NULL");

        SignalSource {
            ptr,
            loop_: &self,
            _data: data,
        }
    }

    /// Register a new event with a callback to be called when the event happens.
    ///
    /// The returned [`EventSource`] can be used to trigger the event.
    #[must_use]
    fn add_event<F>(&self, callback: F) -> EventSource<F, Self>
    where
        F: Fn() + 'static,
        Self: Sized,
    {
        unsafe extern "C" fn call_closure<F>(data: *mut c_void, _count: u64)
        where
            F: Fn(),
        {
            let callback = (data as *mut F).as_ref().unwrap();
            callback();
        }

        let data = Box::into_raw(Box::new(callback));

        let (source, data) = unsafe {
            let mut iface = self
                .as_ptr()
                .as_ref()
                .unwrap()
                .utils
                .as_ref()
                .unwrap()
                .iface;

            let source = spa_interface_call_method!(
                &mut iface as *mut spa_sys::spa_interface,
                spa_sys::spa_loop_utils_methods,
                add_event,
                Some(call_closure::<F>),
                data as *mut _
            );
            (source, Box::from_raw(data))
        };

        let ptr = ptr::NonNull::new(source).expect("source is NULL");

        EventSource {
            ptr,
            loop_: &self,
            _data: data,
        }
    }

    fn destroy_source<S>(&self, source: &S)
    where
        S: IsASource,
        Self: Sized,
    {
        unsafe {
            let mut iface = self
                .as_ptr()
                .as_ref()
                .unwrap()
                .utils
                .as_ref()
                .unwrap()
                .iface;

            spa_interface_call_method!(
                &mut iface as *mut spa_sys::spa_interface,
                spa_sys::spa_loop_utils_methods,
                destroy_source,
                source.as_ptr()
            )
        }
    }
}

pub trait IsASource {
    /// Return a valid pointer to a raw `spa_source`.
    fn as_ptr(&self) -> *mut spa_sys::spa_source;
}

pub struct SignalSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    ptr: ptr::NonNull<spa_sys::spa_source>,
    loop_: &'a L,
    // Store data wrapper to prevent leak
    _data: Box<F>,
}

impl<'a, F, L> IsASource for SignalSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    fn as_ptr(&self) -> *mut spa_sys::spa_source {
        self.ptr.as_ptr()
    }
}

impl<'a, F, L> Drop for SignalSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    fn drop(&mut self) {
        self.loop_.destroy_source(self)
    }
}

/// A source that can be used to signal to a loop that an event has occurred.
///
/// This source can be obtained by calling [`add_event`](`Loop::add_event`) on a loop, registering a callback to it.
/// By calling [`signal`](`EventSource::signal`) on the `EventSource`, the loop is signaled that the event has occurred.
/// It will then call the callback at the next possible occasion.
pub struct EventSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    ptr: ptr::NonNull<spa_sys::spa_source>,
    loop_: &'a L,
    // Store data wrapper to prevent leak
    _data: Box<F>,
}

impl<'a, F, L> IsASource for EventSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    fn as_ptr(&self) -> *mut spa_sys::spa_source {
        self.ptr.as_ptr()
    }
}

impl<'a, F, L> EventSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    /// Signal the loop associated with this source that the event has occurred,
    /// to make the loop call the callback at the next possible occasion.
    pub fn signal(&self) -> SpaResult {
        let res = unsafe {
            let mut iface = self
                .loop_
                .as_ptr()
                .as_ref()
                .unwrap()
                .utils
                .as_ref()
                .unwrap()
                .iface;

            spa_interface_call_method!(
                &mut iface as *mut spa_sys::spa_interface,
                spa_sys::spa_loop_utils_methods,
                signal_event,
                self.as_ptr()
            )
        };

        SpaResult::from_c(res)
    }
}

impl<'a, F, L> Drop for EventSource<'a, F, L>
where
    F: Fn() + 'static,
    L: Loop,
{
    fn drop(&mut self) {
        self.loop_.destroy_source(self)
    }
}
