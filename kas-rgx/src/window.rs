// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use std::{cell::RefCell, rc::Rc};

use rgx::core::*;
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoopWindowTarget;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::window::WindowId;

use kas::callback::Condition;
use kas::event::{Action, GuiResponse};
use kas::{Class, Coord, Widget, TkData};

use crate::widget::Widgets;
// use crate::tkd::WidgetAbstraction;


/// Per-window data
pub struct Window {
    win: Box<dyn kas::Window>,
    /// The winit window
    ww: winit::window::Window,
    /// The renderer attached to this window
    rend: Renderer,
//     /// The GTK window
//     pub gwin: gtk::Window,
    nums: (u32, u32),   // TODO: is this useful?
    widgets: Widgets,
}

// Clear TKD on all widgets to reduce pointer reference counts.
// We can't implement Drop on kas types directly since TkData is interpreted
// by this lib.
impl Drop for Window {
    fn drop(&mut self) {
        // TODO?
    }
}

// Public functions, for use by the toolkit
impl Window {
    /// Construct a window
    /// 
    /// Parameter `num0`: for the first window, use 0. For any other window,
    /// use the previous window's `nums().1` value.
    pub fn new<T: 'static>(
        event_loop: &EventLoopWindowTarget<T>,
        mut win: Box<dyn kas::Window>,
        num0: u32)
        -> Result<Window, OsError>
    {
        let ww = winit::window::Window::new(event_loop)?;
        let rend = Renderer::new(ww.raw_window_handle());
        
        let num1 = win.enumerate(num0);
        
        let mut widgets = Widgets::new();
        widgets.add(win.as_widget_mut());
        
        let mut w = Window {
            win,
            ww,
            rend,
            nums: (num0, num1),
            widgets,
        };
        
        Ok(w)
    }
    
    /// Range of widget numbers used, from first to last+1.
    pub fn nums(&self) -> (u32, u32) {
        self.nums
    }
    
    /// Identifier for this window
    pub fn id(&self) -> WindowId {
        self.ww.id()
    }
    
    /// Called by the `Toolkit` just before the event loop starts to initialise
    /// windows.
    pub fn prepare(&mut self) {
        self.do_resize(self.ww.inner_size());
        self.win.on_start(&mut self.widgets);
    }
    
    /// Handle an event
    /// 
    /// Return true to remove the window
    pub fn handle_event(&mut self, event: WindowEvent) -> bool {
        use WindowEvent::*;
        match event {
            CursorEntered {..} | KeyboardInput {..} | MouseInput {..} => {
                // TODO: handle input
            }
            RedrawRequested => {
                // TODO
            }
            Resized(size) => self.do_resize(size),
            CloseRequested => {
                return true;
            }
            _ => {
                println!("Unhandled window event: {:?}", event);
            }
        }
        false
    }
}

// Internal functions
impl Window {
    fn do_resize(&mut self, size: LogicalSize) {
        // TODO: work with logical size to allow DPI scaling
        let size: (u32, u32) = size.to_physical(self.ww.hidpi_factor()).into();
        // TODO: any reason Coord should not use u32?
        let size = (size.0 as i32, size.1 as i32);
        self.win.configure_widgets(&mut self.widgets);
        self.win.resize(&mut self.widgets, size);
    }
}

/*
/// A list of windows
/// 
/// This is a special type which has a single instance per thread.
pub(crate) struct WindowList {
    pub(crate) windows: Vec<Window>,
}

// Use thread_local because our type and GTK pointers are not Sync.
thread_local! {
    static WINDOWS: RefCell<WindowList> = RefCell::new(WindowList::new());
}

/// Call some closure on the thread-local window list.
pub(crate) fn with_list<F: FnOnce(&mut WindowList)>(f: F) {
    WINDOWS.with(|cell| f(&mut *cell.borrow_mut()) );
}

impl WindowList {
    const fn new() -> Self {
        WindowList { windows: Vec::new() }
    }
    
    // Find first window with matching `gdk::Window`, run the closure, and
    // return the result, or `None` if no match.
    pub(crate) fn for_gdk_win<T, F>(&mut self, gdk_win: gdk::Window, f: F) -> Option<T>
        where F: FnOnce(&mut dyn kas::Window, &mut gtk::Window) -> T
    {
        let gdk_win = Some(gdk_win);
        for item in self.windows.iter_mut() {
            if item.gwin.get_window() == gdk_win {
                return Some(f(&mut *item.win.borrow_mut(), &mut item.gwin))
            }
        }
        None
    }
}

fn add_widgets(gtk_widget: &gtk::Widget, widget: &mut dyn Widget) {
    widget.set_gw(gtk_widget);
    if let Some(gtk_container) = gtk_widget.downcast_ref::<gtk::Container>() {
        for i in 0..widget.len() {
            let child = widget.get_mut(i).unwrap();
            // TODO: use trait implementation for each different class?
            let gtk_child = match child.class() {
                Class::Container => {
                    // orientation is unimportant
                    gtk::Box::new(gtk::Orientation::Horizontal, 0)
                                .upcast::<gtk::Widget>()
                }
                Class::Button(iface) => {
                    let button = gtk::Button::new_with_label(iface.get_text());
                    if true /*TODO iface.has_on_click()*/ {
                        let num = child.number();
                        button.connect_clicked(move |_| {
                            let action = Action::Button;
                            with_list(|list| list.handle_action(action, num))
                        });
                    }
                    button.upcast::<gtk::Widget>()
                }
                Class::CheckBox(iface) => {
                    let button = gtk::CheckButton::new_with_label(iface.get_text());
                    button.set_active(iface.get_bool());
                    if true /*TODO iface.has_on_toggle()*/ {
                        let num = child.number();
                        button.connect_toggled(move |_| {
                            let action = Action::Toggle;
                            with_list(|list| list.handle_action(action, num))
                        });
                    }
                    button.upcast::<gtk::Widget>()
                }
                Class::Label(iface) => {
                    let label = gtk::Label::new(Some(iface.get_text()));
                    // Text naturally has a top/bottom margin, but not start/end
                    // which looks quite odd. Does this solution scale well?
                    label.set_margin_start(2);
                    label.set_margin_end(2);
                    label.upcast::<gtk::Widget>()
                }
                Class::Entry(iface) => {
                    let entry = gtk::Entry::new();
                    entry.set_editable(iface.is_editable());
                    entry.set_text(iface.get_text());
                    if true /*TODO iface.has_on_activate()*/ {
                        let num = child.number();
                        entry.connect_activate(move |_| {
                            let action = Action::Activate;
                            with_list(|list| list.handle_action(action, num))
                        });
                    }
                    entry.upcast::<gtk::Widget>()
                }
                Class::Frame => {
                    // GTK frame with no label
                    gtk::Frame::new(None)
                            .upcast::<gtk::Widget>()
                }
                Class::Window => panic!(),  // TODO embedded windows?
            };
            
            add_widgets(&gtk_child, child);
            
//             #[cfg(not(feature = "layout"))] {
//                 if let Some(grid) = gtk_container.downcast_ref::<gtk::Grid>() {
//                     let pos = widget.grid_pos(i).unwrap_or((0, 0, 1, 1));
//                     grid.attach(&gtk_child, pos.0, pos.1, pos.2, pos.3);
//                     continue;   // attach(...) instead of add(...)
//                 }
//             }
            gtk_container.add(&gtk_child);
        }
    }
}
*/