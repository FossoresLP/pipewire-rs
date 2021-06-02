//! Miscellaneous and utility items.

pub use spa_sys::spa_fraction as Fraction;
pub use spa_sys::spa_rectangle as Rectangle;

/// An enumerated value in a pod
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Id(pub u32);

/// A file descriptor in a pod
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct Fd(pub i64);
