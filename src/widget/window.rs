// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use smallvec::SmallVec;
use std::fmt::{self, Debug};

use kas::draw::ClipRegion;
use kas::event::{self, UpdateHandle};
use kas::layout;
use kas::prelude::*;
use kas::{Future, WindowId};

/// The main instantiation of the [`Window`] trait.
#[handler(send=noauto, generics = <> where W: Widget<Msg = VoidMsg>)]
#[derive(Widget)]
pub struct Window<W: Widget + 'static> {
    #[widget_core]
    core: CoreData,
    restrict_dimensions: (bool, bool),
    title: String,
    #[widget]
    w: W,
    popups: SmallVec<[(WindowId, kas::Popup); 16]>,
    drop: Option<(Box<dyn FnMut(&mut W)>, UpdateHandle)>,
}

impl<W: Widget> Debug for Window<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Window {{ core: {:?}, restrict_dimensions: {:?}, title: {:?}, w: {:?}, popups: {:?}, drop: ",
            self.core, self.restrict_dimensions, self.title, self.w, self.popups,
        )?;
        if let Some(ref d) = self.drop {
            write!(f, "Some(<closure>, {:?})", d.1)?;
        } else {
            write!(f, "None")?;
        }
        write!(f, " }}")
    }
}

impl<W: Widget + Clone> Clone for Window<W> {
    fn clone(&self) -> Self {
        Window {
            core: self.core.clone(),
            restrict_dimensions: self.restrict_dimensions.clone(),
            title: self.title.clone(),
            w: self.w.clone(),
            popups: Default::default(), // these are temporary; don't clone
            drop: None,                 // we cannot clone this!
        }
    }
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new<T: ToString>(title: T, w: W) -> Window<W> {
        Window {
            core: Default::default(),
            restrict_dimensions: (true, false),
            title: title.to_string(),
            w,
            popups: Default::default(),
            drop: None,
        }
    }

    /// Configure whether min/max dimensions are forced
    ///
    /// By default, the min size is enforced but not the max.
    pub fn set_restrict_dimensions(&mut self, min: bool, max: bool) {
        self.restrict_dimensions = (min, max);
    }

    /// Set a closure to be called on destruction, and return a future
    ///
    /// The closure `consume` is called when the window is destroyed, and yields
    /// a user-defined value. This value is returned through the returned
    /// [`Future`] object. In order to be notified when the future
    /// completes, its owner should call [`Manager::update_on_handle`] with the
    /// returned [`UpdateHandle`].
    ///
    /// Currently it is not possible for this closure to actually drop the
    /// widget, but it may alter its contents: it is the last method call on
    /// the widget. (TODO: given unsized rvalues (rfc#1909), the closure should
    /// consume self.)
    ///
    /// Panics if called more than once. In case the window is cloned, this
    /// closure is *not* inherited by the clone: in that case, `on_drop` may be
    /// called on the clone.
    pub fn on_drop<T>(
        &mut self,
        consume: Box<dyn FnMut(&mut W) -> T>,
    ) -> (Future<T>, UpdateHandle) {
        if self.drop.is_some() {
            panic!("Window::on_drop: attempt to set multiple drop closures");
        }
        let (future, finish) = Future::new_box_fnmut(consume);
        let update = UpdateHandle::new();
        self.drop = Some((finish, update));
        (future, update)
    }
}

impl<W: Widget> Layout for Window<W> {
    #[inline]
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        // Note: we do not consider popups, since they are usually temporary
        self.w.size_rules(size_handle, axis)
    }

    #[inline]
    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.w.set_rect(rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        for popup in self.popups.iter().rev() {
            if let Some(id) = self.w.find(popup.1.id).and_then(|w| w.find_id(coord)) {
                return Some(id);
            }
        }
        self.w.find_id(coord).or(Some(self.id()))
    }

    #[inline]
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        self.w.draw(draw_handle, mgr, disabled);
        for popup in &self.popups {
            let class = ClipRegion::Popup;
            draw_handle.clip_region(self.core.rect, Coord::ZERO, class, &mut |draw_handle| {
                self.find(popup.1.id)
                    .map(|w| w.draw(draw_handle, mgr, disabled));
            });
        }
    }
}

impl<W: Widget<Msg = VoidMsg> + 'static> event::SendEvent for Window<W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if !self.is_disabled() && id <= self.w.id() {
            return self.w.send(mgr, id, event);
        }
        Response::Unhandled(event)
    }
}

impl<W: Widget<Msg = VoidMsg> + 'static> kas::Window for Window<W> {
    fn title(&self) -> &str {
        &self.title
    }

    fn restrict_dimensions(&self) -> (bool, bool) {
        self.restrict_dimensions
    }

    fn add_popup(&mut self, mgr: &mut Manager, id: WindowId, popup: kas::Popup) {
        let index = self.popups.len();
        self.popups.push((id, popup));
        mgr.size_handle(|size_handle| self.resize_popup(size_handle, index));
        mgr.send_action(TkAction::Redraw);
    }

    fn remove_popup(&mut self, mgr: &mut Manager, id: WindowId) {
        for i in 0..self.popups.len() {
            if id == self.popups[i].0 {
                self.popups.remove(i);
                mgr.send_action(TkAction::RegionMoved);
                return;
            }
        }
    }

    fn resize_popups(&mut self, size_handle: &mut dyn SizeHandle) {
        for i in 0..self.popups.len() {
            self.resize_popup(size_handle, i);
        }
    }

    fn handle_closure(&mut self, mgr: &mut Manager) {
        if let Some((mut consume, update)) = self.drop.take() {
            consume(&mut self.w);
            mgr.trigger_update(update, 0);
        }
    }
}

// This is like WidgetChildren::find, but returns a translated Rect.
fn find_rect(widget: &dyn WidgetConfig, id: WidgetId) -> Option<Rect> {
    if id == widget.id() {
        return Some(widget.rect());
    } else if id > widget.id() {
        return None;
    }

    for i in 0..widget.len() {
        if let Some(w) = widget.get(i) {
            if id > w.id() {
                continue;
            }
            return find_rect(w, id).map(|rect| rect - widget.translation(i));
        }
        break;
    }
    None
}

impl<W: Widget> Window<W> {
    fn resize_popup(&mut self, size_handle: &mut dyn SizeHandle, index: usize) {
        // Notation: p=point/coord, s=size, m=margin
        // r=window/root rect, c=anchor rect
        let r = self.core.rect;
        let popup = &mut self.popups[index].1;

        let c = find_rect(self.w.as_widget(), popup.parent).unwrap();
        let widget = self.w.find_mut(popup.id).unwrap();
        let mut cache = layout::SolveCache::find_constraints(widget, size_handle);
        let ideal = cache.ideal(false);
        let m = cache.margins();

        let is_reversed = popup.direction.is_reversed();
        let place_in = |rp, rs: u32, cp: i32, cs, ideal, m: (u16, u16)| -> (i32, u32) {
            let before: i32 = cp - (rp + m.1 as i32);
            let before = before.max(0) as u32;
            let after = rs.saturating_sub(cs + before + m.0 as u32);
            if after >= ideal {
                if is_reversed && before >= ideal {
                    (cp - ideal as i32 - m.1 as i32, ideal)
                } else {
                    (cp + cs as i32 + m.0 as i32, ideal)
                }
            } else if before >= ideal {
                (cp - ideal as i32 - m.1 as i32, ideal)
            } else if before > after {
                (rp, before)
            } else {
                (cp + cs as i32 + m.0 as i32, after)
            }
        };
        let place_out = |rp, rs, cp: i32, cs, ideal: u32| -> (i32, u32) {
            let pos = cp.min(rp + rs as i32 - ideal as i32).max(rp);
            let size = ideal.max(cs).min(rs);
            (pos, size)
        };
        let rect = if popup.direction.is_horizontal() {
            let (x, w) = place_in(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0, m.horiz);
            let (y, h) = place_out(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1);
            Rect::new(Coord(x, y), Size(w, h))
        } else {
            let (x, w) = place_out(r.pos.0, r.size.0, c.pos.0, c.size.0, ideal.0);
            let (y, h) = place_in(r.pos.1, r.size.1, c.pos.1, c.size.1, ideal.1, m.vert);
            Rect::new(Coord(x, y), Size(w, h))
        };

        cache.apply_rect(widget, size_handle, rect, false);
    }
}
