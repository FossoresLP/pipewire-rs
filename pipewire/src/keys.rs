// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

//! A collection of keys that are used to add extra information on objects.
//!
//! ```
//! use pipewire::properties;
//!
//! let props = properties! {
//!   *pipewire::keys::REMOTE_NAME => "pipewire-0"
//! };
//! ```

use std::ffi::CStr;

use once_cell::sync::Lazy;

// unfortunatelly we have to take two args as concat_idents! is in experimental
macro_rules! key_constant {
    ($name:ident, $pw_symbol:ident, #[doc = $doc:expr]) => {
        #[doc = $doc]
        pub static $name: Lazy<&'static str> = Lazy::new(|| unsafe {
            CStr::from_bytes_with_nul_unchecked(pw_sys::$pw_symbol)
                .to_str()
                .unwrap()
        });
    };
}

include!("auto/keys.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(*REMOTE_NAME, "remote.name");
    }
}
