// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::{convert::TryInto, fmt};

use errno::Errno;

#[derive(Debug, PartialEq)]
pub struct SpaResult(i32);

#[derive(Debug, PartialEq)]
pub enum SpaSuccess {
    Sync(i32),
    Async(i32),
}

fn async_seq(res: i32) -> i32 {
    let mask: i32 = spa_sys::SPA_ASYNC_SEQ_MASK.try_into().unwrap();
    res & mask
}

impl SpaResult {
    pub fn from_c(res: i32) -> Self {
        Self(res)
    }

    /// Pending return for async operation identified with sequence number `seq`.
    pub fn new_return_async(seq: i32) -> Self {
        let bit: i32 = spa_sys::SPA_ASYNC_BIT.try_into().unwrap();
        let res = bit | async_seq(seq);
        Self::from_c(res)
    }

    fn is_async(&self) -> bool {
        let bit: i32 = spa_sys::SPA_ASYNC_BIT.try_into().unwrap();
        (self.0 & spa_sys::SPA_ASYNC_MASK as i32) == bit
    }

    pub fn into_result(self) -> Result<SpaSuccess, Error> {
        if self.0 < 0 {
            Err(Error::new(-self.0))
        } else if self.is_async() {
            Ok(SpaSuccess::Async(async_seq(self.0)))
        } else {
            Ok(SpaSuccess::Sync(self.0))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Error(Errno);

impl Error {
    fn new(e: i32) -> Self {
        assert!(e > 0);

        Self(Errno(e))
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    /* the errno crate is calling foreign function __xpg_strerror_r which is not supported by miri */
    fn spa_result() {
        assert!(!SpaResult::from_c(0).is_async());
        assert!(SpaResult::new_return_async(0).is_async());

        assert_eq!(SpaResult::from_c(0).into_result(), Ok(SpaSuccess::Sync(0)));
        assert_eq!(SpaResult::from_c(1).into_result(), Ok(SpaSuccess::Sync(1)));
        assert_eq!(
            SpaResult::new_return_async(1).into_result(),
            Ok(SpaSuccess::Async(1))
        );

        let err = SpaResult::from_c(-libc::EBUSY).into_result().unwrap_err();
        assert_eq!(format!("{}", err), "Device or resource busy",);
    }
}
