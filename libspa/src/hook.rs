// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use crate::list;

pub fn remove(mut hook: spa_sys::spa_hook) {
    list::remove(&hook.link);

    if let Some(removed) = hook.removed {
        unsafe {
            removed(&mut hook as *mut _);
        }
    }
}
