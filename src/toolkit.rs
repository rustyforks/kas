// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface
//!
//! In KAS, the "toolkit" is an external library handling system interfaces
//! (windowing and event translation) plus rendering. This allows KAS's core
//! to remain system-neutral.
//!
//! Note: although the choice of windowing library is left to the toolkit, for
//! convenience KAS is able to use several [winit] types.
//!
//! [winit]: https://github.com/rust-windowing/winit

use std::num::NonZeroU32;

/// Identifier for a window added to a toolkit
///
/// Identifiers should always be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WindowId(NonZeroU32);

impl WindowId {
    /// Construct a [`WindowId`]
    ///
    /// Only for toolkit use!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    pub fn new(n: NonZeroU32) -> WindowId {
        WindowId(n)
    }
}

/// Toolkit actions needed after event handling, if any.
#[must_use]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum TkAction {
    /// No action needed
    None,
    /// Whole window requires redrawing
    ///
    /// Note that [`Manager::redraw`] can instead be used for more selective
    /// redrawing, if supported by the toolkit.
    Redraw,
    /// Whole window requires reconfiguring (implies redrawing)
    ///
    /// *Configuring* widgets assigns [`WidgetId`] identifiers, updates
    /// [`event::Manager`] state and resizes all widgets.
    Reconfigure,
    /// Window should be closed
    Close,
    /// All windows should close (toolkit exit)
    CloseAll,
}

/// Toolkit-specific window management and style interface.
///
/// This is implemented by a KAS toolkit on a window handle.
pub trait TkWindow {
    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added via event
    /// processing, albeit without error handling.
    fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    fn set_clipboard(&mut self, content: String);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn action_precedence() {
        assert!(TkAction::None < TkAction::Redraw);
        assert!(TkAction::Redraw < TkAction::Reconfigure);
        assert!(TkAction::Reconfigure < TkAction::Close);
        assert!(TkAction::Close < TkAction::CloseAll);
    }
}
