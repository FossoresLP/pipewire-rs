// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::{convert::TryInto, os::unix::prelude::*, ptr, time::Duration};

use libc::{c_int, c_void};
use signal::Signal;
use spa::{flags::IoFlags, result::SpaResult, spa_interface_call_method};

use crate::utils::assert_main_thread;

/// A trait for common functionality of the different pipewire loop kinds, most notably [`MainLoop`](`crate::MainLoop`).
///
/// Different kinds of events, such as receiving a signal (e.g. SIGTERM) can be attached to the loop using this trait.
pub unsafe trait Loop {
    fn as_ptr(&self) -> *mut pw_sys::pw_loop;

    #[must_use]
    fn add_io<I, F>(&self, io: I, event_mask: IoFlags, callback: F) -> IoSource<I, Self>
    where
        I: AsRawFd,
        F: Fn(&mut I) + 'static,
        Self: Sized,
    {
        unsafe extern "C" fn call_closure<I>(data: *mut c_void, _fd: RawFd, _mask: u32)
        where
            I: AsRawFd,
        {
            let (io, callback) = (data as *mut IoSourceData<I>).as_mut().unwrap();
            callback(io);
        }

        let fd = io.as_raw_fd();
        let data = Box::into_raw(Box::new((io, Box::new(callback) as Box<dyn Fn(&mut I)>)));

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
                add_io,
                fd,
                // FIXME: User provided mask instead
                event_mask.bits(),
                // Never let the loop close the fd, this should be handled via `Drop` implementations.
                false,
                Some(call_closure::<I>),
                data as *mut _
            );

            (source, Box::from_raw(data))
        };

        let ptr = ptr::NonNull::new(source).expect("source is NULL");

        IoSource {
            ptr,
            loop_: self,
            _data: data,
        }
    }

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
            loop_: self,
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
            loop_: self,
            _data: data,
        }
    }

    /// Register a timer with the loop.
    ///
    /// The timer will start out inactive, and the returned [`TimerSource`] can be used to arm the timer, or disarm it again.
    ///
    /// The callback will be provided with the number of timer expirations since the callback was last called.
    #[must_use]
    fn add_timer<F>(&self, callback: F) -> TimerSource<F, Self>
    where
        F: Fn(u64) + 'static,
        Self: Sized,
    {
        unsafe extern "C" fn call_closure<F>(data: *mut c_void, expirations: u64)
        where
            F: Fn(u64),
        {
            let callback = (data as *mut F).as_ref().unwrap();
            callback(expirations);
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
                add_timer,
                Some(call_closure::<F>),
                data as *mut _
            );
            (source, Box::from_raw(data))
        };

        let ptr = ptr::NonNull::new(source).expect("source is NULL");

        TimerSource {
            ptr,
            loop_: self,
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

type IoSourceData<I> = (I, Box<dyn Fn(&mut I) + 'static>);
pub struct IoSource<'l, I, L>
where
    I: AsRawFd,
    L: Loop,
{
    ptr: ptr::NonNull<spa_sys::spa_source>,
    loop_: &'l L,
    // Store data wrapper to prevent leak
    _data: Box<IoSourceData<I>>,
}

impl<'l, I, L> IsASource for IoSource<'l, I, L>
where
    I: AsRawFd,
    L: Loop,
{
    fn as_ptr(&self) -> *mut spa_sys::spa_source {
        self.ptr.as_ptr()
    }
}

impl<'l, I, L> Drop for IoSource<'l, I, L>
where
    I: AsRawFd,
    L: Loop,
{
    fn drop(&mut self) {
        self.loop_.destroy_source(self)
    }
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

/// A source that can be used to have a callback called on a timer.
///
/// This source can be obtained by calling [`add_timer`](`Loop::add_timer`) on a loop, registering a callback to it.
///
/// The timer starts out inactive.
/// You can arm or disarm the timer by calling [`update_timer`](`Self::update_timer`).
pub struct TimerSource<'a, F, L>
where
    F: Fn(u64) + 'static,
    L: Loop,
{
    ptr: ptr::NonNull<spa_sys::spa_source>,
    loop_: &'a L,
    // Store data wrapper to prevent leak
    _data: Box<F>,
}

impl<'a, F, L> TimerSource<'a, F, L>
where
    F: Fn(u64) + 'static,
    L: Loop,
{
    /// Arm or disarm the timer.
    ///
    /// The timer will be called the next time after the provided `value` duration.
    /// After that, the timer will be repeatedly called again at the the specified `interval`.
    ///
    /// If `interval` is `None` or zero, the timer will only be called once. \
    /// If `value` is `None` or zero, the timer will be disabled.
    ///
    /// # Panics
    /// The provided durations seconds must fit in an i64. Otherwise, this function will panic.
    pub fn update_timer(&self, value: Option<Duration>, interval: Option<Duration>) -> SpaResult {
        fn duration_to_timespec(duration: Duration) -> spa_sys::timespec {
            spa_sys::timespec {
                tv_sec: duration.as_secs().try_into().expect("Duration too long"),
                tv_nsec: duration.subsec_nanos().try_into().unwrap(),
            }
        }

        let value = duration_to_timespec(value.unwrap_or_default());
        let interval = duration_to_timespec(interval.unwrap_or_default());

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
                update_timer,
                self.as_ptr(),
                &value as *const _ as *mut _,
                &interval as *const _ as *mut _,
                false
            )
        };

        SpaResult::from_c(res)
    }
}

impl<'a, F, L> IsASource for TimerSource<'a, F, L>
where
    F: Fn(u64) + 'static,
    L: Loop,
{
    fn as_ptr(&self) -> *mut spa_sys::spa_source {
        self.ptr.as_ptr()
    }
}

impl<'a, F, L> Drop for TimerSource<'a, F, L>
where
    F: Fn(u64) + 'static,
    L: Loop,
{
    fn drop(&mut self) {
        self.loop_.destroy_source(self)
    }
}
