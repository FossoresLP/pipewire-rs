// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

#![warn(missing_docs)]

//! The `libspa` crate provides a high-level API to interact with
//! [libspa](https://gitlab.freedesktop.org/pipewire/pipewire/-/tree/master/doc/spa).

pub mod dict;
pub use dict::*;
pub mod result;
pub use result::*;
mod direction;
pub mod hook;
pub mod interface;
pub mod list;
pub mod pod;
pub mod utils;
pub use direction::*;
pub mod flags;

/// prelude module re-exporing all the traits providing public API.
pub mod prelude {
    pub use crate::dict::{ReadableDict, WritableDict};
}
