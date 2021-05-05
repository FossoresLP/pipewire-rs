// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

//! # Rust bindings for pipewire
//! `pipewire` is a crate offering a rustic bindings for `libpipewire`, the library for interacting
//! with the pipewire server.
//!
//! Programs that interact with pipewire usually react to events from the server by registering callbacks
//! and invoke methods on objects on the server by calling methods on local proxy objects.
//!
//! ## Getting started
//! Most programs that interact with pipewire will need the same few basic objects:
//! - A [`MainLoop`] that drives the program, reacting to any incoming events and dispatching method calls.
//!   Most of a time, the program/thread will sit idle in this loop, waiting on events to occur.
//! - A [`Context`] that keeps track of any pipewire resources.
//! - A [`Core`] that is a proxy for the remote pipewire instance, used to send messages to and receive events from the
//!   remote server.
//! - Optionally, a [`Registry`](`registry::Registry`) that can be used to manage and track available objects on the server.
//!
//! This is how they can be created:
// ignored because https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/issues/19
//! ```no_run
//! use pipewire::{MainLoop, Context};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mainloop = MainLoop::new()?;
//!     let context = Context::new(&mainloop)?;
//!     let core = context.connect(None)?;
//!     let registry = core.get_registry()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! Now you can start hooking up different kinds of callbacks to the objects to react to events, and call methods
//! on objects to change the state of the remote.
//! ```no_run
//! use pipewire::{MainLoop, Context};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mainloop = MainLoop::new()?;
//!     let context = Context::new(&mainloop)?;
//!     let core = context.connect(None)?;
//!     let registry = core.get_registry()?;
//!
//!     // Register a callback to the `global` event on the registry, which notifies of any new global objects
//!     // appearing on the remote.
//!     // The callback will only get called as long as we keep the returned listener alive.
//!     let _listener = registry
//!         .add_listener_local()
//!         .global(|global| println!("New global: {:?}", global))
//!         .register();
//!
//!     // Calling the `destroy_global` method on the registry will destroy the object with the specified id on the remote.
//!     // We don't have a specific object to destroy now, so this is commented out.
//!     # // FIXME: Find a better method for this example we can actually call.
//!     // registry.destroy_global(313).into_result()?;
//!
//!     mainloop.run();
//!
//!     Ok(())
//! }
//! ```
//! Note that registering any callback requires the closure to have the `'static` lifetime, so if you need to capture
//! any variables, use `move ||` closures, and use `std::rc::Rc`s to access shared variables
//! and some `std::cell` variant if you need to mutate them.
//!
//! Also note that we called `mainloop.run()` at the end.
//! This will enter the loop, and won't return until we call `mainloop.quit()` from some event.
//! If we didn't run the loop, events and method invocations would not be processed, so the program would terminate
//! without doing much.
//!
//! ## The main loop
//! Sometimes, other stuff needs to be done even though we are waiting inside the main loop. \
//! This can be done by adding sources to the loop.
//!
//! For example, we can call a function on an interval:
//!
//! ```no_run
//! // We also need to include the `Loop` trait for this.
//! use pipewire::{MainLoop, Loop};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mainloop = MainLoop::new()?;
//!
//!     let timer = mainloop.add_timer(|_| println!("Hello"));
//!     // Call the first time in half a second, and then in a one second interval.
//!     timer.update_timer(Some(Duration::from_millis(500)), Some(Duration::from_secs(1))).into_result()?;
//!
//!     mainloop.run();
//!
//!     Ok(())
//! }
//! ```
//! This program will print out "Hello" every second forever.
//!
//! You can also react to IO or Signals using similar methods on the [`Loop`] trait.
//!
//! ## Multithreading
//! The pipewire library is not really thread-safe, so pipewire objects do not implement [`Send`](`std::marker::Send`)
//! or [`Sync`](`std::marker::Sync`).
//!
//! However, you can spawn a [`MainLoop`] in another thread and do bidirectional communication using two channels.
//!
//! To send messages to the main thread, we can easily use a [`std::sync::mpsc`].
//! Because we are stuck in the main loop in the pipewire thread and can't just block on receiving a message,
//! we use a [`pipewire::channel`](`crate::channel`) instead.
//!
//! See the [`pipewire::channel`](`crate::channel`) module for details.

use std::ptr;

mod error;
pub use error::*;
pub mod loop_;
pub use loop_::*;
mod main_loop;
pub use main_loop::*;
mod context;
pub use context::*;
mod core_;
pub use core_::*;
mod properties;
pub use properties::*;
pub mod link;
pub mod node;
pub mod port;
pub mod proxy;
pub mod registry;
pub use spa;
pub mod channel;
pub mod constants;
pub mod keys;
pub mod stream;
pub mod types;
mod utils;
pub use pw_sys as sys;

// Re-export all the traits in a prelude module, so that applications
// can always "use pipewire::prelude::*" without getting conflicts
pub mod prelude {
    pub use crate::loop_::Loop;
    pub use crate::stream::ListenerBuilderT;
    pub use spa::prelude::*;
}

/// Initialize PipeWire
///
/// Initialize the PipeWire system and set up debugging
/// through the environment variable `PIPEWIRE_DEBUG`.
pub fn init() {
    use once_cell::sync::OnceCell;
    static INITIALIZED: OnceCell<()> = OnceCell::new();
    INITIALIZED.get_or_init(|| unsafe { pw_sys::pw_init(ptr::null_mut(), ptr::null_mut()) });
}

/// Deinitialize PipeWire
///
/// # Safety
/// This must only be called once during the lifetime of the process, once no PipeWire threads
/// are running anymore and all PipeWire resources are released.
pub unsafe fn deinit() {
    pw_sys::pw_deinit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
        unsafe {
            deinit();
        }
    }
}
