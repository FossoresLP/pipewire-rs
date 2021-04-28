//! Miscellaneous and utility items.

pub use spa_sys::spa_fraction as Fraction;
pub use spa_sys::spa_rectangle as Rectangle;

/// An enumerated value in a pod
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Id(pub u32);
