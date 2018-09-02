//! Utilities

/// A rectangular region.
#[derive(Clone, Default)]
pub struct Rect {
    pub pos: (i32, i32),
    pub size: (i32, i32),
}