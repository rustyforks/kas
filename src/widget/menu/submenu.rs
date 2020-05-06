// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::{Menu, MenuFrame};
use kas::class::HasText;
use kas::draw::TextClass;
use kas::event::{ConfigureManager, NavKey};
use kas::prelude::*;
use kas::widget::Column;
use kas::WindowId;

/// A sub-menu
#[widget(config=noauto)]
#[handler(noauto)]
#[derive(Clone, Debug, Widget)]
pub struct SubMenu<D: Directional, W: Menu> {
    #[widget_core]
    core: CoreData,
    direction: D,
    label: AccelString,
    label_off: Coord,
    #[widget]
    pub list: MenuFrame<Column<W>>,
    popup_id: Option<WindowId>,
}

impl<D: Directional + Default, W: Menu> SubMenu<D, W> {
    /// Construct a sub-menu
    #[inline]
    pub fn new<S: Into<AccelString>>(label: S, list: Vec<W>) -> Self {
        SubMenu::new_with_direction(Default::default(), label, list)
    }
}

impl<W: Menu> SubMenu<kas::Right, W> {
    /// Construct a sub-menu, opening to the right
    // NOTE: this is used since we can't infer direction of a boxed SubMenu.
    // Consider only accepting an enum of special menu widgets?
    // Then we can pass type information.
    #[inline]
    pub fn right<S: Into<AccelString>>(label: S, list: Vec<W>) -> Self {
        SubMenu::new(label, list)
    }
}

impl<W: Menu> SubMenu<kas::Down, W> {
    /// Construct a sub-menu, opening downwards
    #[inline]
    pub fn down<S: Into<AccelString>>(label: S, list: Vec<W>) -> Self {
        SubMenu::new(label, list)
    }
}

impl<D: Directional, W: Menu> SubMenu<D, W> {
    /// Construct a sub-menu
    #[inline]
    pub fn new_with_direction<S: Into<AccelString>>(direction: D, label: S, list: Vec<W>) -> Self {
        SubMenu {
            core: Default::default(),
            direction,
            label: label.into(),
            label_off: Coord::ZERO,
            list: MenuFrame::new(Column::new(list)),
            popup_id: None,
        }
    }

    fn open_menu(&mut self, mgr: &mut Manager) {
        if self.popup_id.is_none() {
            let id = mgr.add_popup(kas::Popup {
                id: self.list.id(),
                parent: self.id(),
                direction: self.direction.as_direction(),
            });
            self.popup_id = Some(id);
            mgr.next_nav_focus(self, false);
        }
    }
    fn close_menu(&mut self, mgr: &mut Manager) {
        if let Some(id) = self.popup_id {
            mgr.close_window(id);
        }
    }
}

impl<D: Directional, W: Menu> WidgetConfig for SubMenu<D, W> {
    fn configure_recurse<'a, 'b>(&mut self, mut cmgr: ConfigureManager<'a, 'b>) {
        cmgr.mgr().push_accel_layer(true);
        self.list.configure_recurse(cmgr.child());
        self.core_data_mut().id = cmgr.next_id(self.id());
        let mgr = cmgr.mgr();
        mgr.pop_accel_layer(self.id());
        mgr.add_accel_keys(self.id(), self.label.keys());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<D: Directional, W: Menu> kas::Layout for SubMenu<D, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.menu_frame();
        self.label_off = size.into();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), size + size, Margins::ZERO);
        let text_rules = size_handle.text_bound(self.label.get(false), TextClass::Label, axis);
        text_rules.surrounded_by(frame_rules, true)
    }

    fn spatial_range(&self) -> (usize, usize) {
        // We have no child within our rect; return an empty range
        (0, std::usize::MAX)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let mut state = self.input_state(mgr, disabled);
        state.depress = state.depress || self.popup_id.is_some();
        draw_handle.menu_entry(self.core.rect, state);
        let rect = Rect {
            pos: self.core.rect.pos + self.label_off,
            size: self.core.rect.size - self.label_off.into(),
        };
        let text = self.label.get(mgr.show_accel_labels());
        let align = (Align::Begin, Align::Centre);
        draw_handle.text(rect, text, TextClass::Label, align);
    }
}

impl<D: Directional, M, W: Menu<Msg = M>> event::Handler for SubMenu<D, W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => {
                if self.popup_id.is_none() {
                    self.open_menu(mgr);
                }
            }
            Event::NewPopup(id) => {
                if self.popup_id.is_some() && !self.is_ancestor_of(id) {
                    self.close_menu(mgr);
                }
            }
            Event::PopupRemoved(id) => {
                debug_assert_eq!(Some(id), self.popup_id);
                self.popup_id = None;
            }
            Event::NavKey(key) => match (self.direction.as_direction(), key) {
                (Direction::Left, NavKey::Left) => self.open_menu(mgr),
                (Direction::Right, NavKey::Right) => self.open_menu(mgr),
                (Direction::Up, NavKey::Up) => self.open_menu(mgr),
                (Direction::Down, NavKey::Down) => self.open_menu(mgr),
                (_, key) => return Response::Unhandled(Event::NavKey(key)),
            },
            event => return Response::Unhandled(event),
        }
        Response::None
    }
}

impl<D: Directional, W: Menu> event::SendEvent for SubMenu<D, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.list.id() {
            let r = self.list.send(mgr, id, event);

            // The pop-up API expects us to check actions here
            // But NOTE: we don't actually use this. Should we remove from API?
            match mgr.pop_action() {
                TkAction::Close => {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id);
                    }
                }
                other => mgr.send_action(other),
            }

            match r {
                Response::Unhandled(ev) => match ev {
                    Event::NavKey(key) if self.popup_id.is_some() => {
                        if self.popup_id.is_some() {
                            let dir = self.direction.as_direction();
                            let inner_vert = self.list.inner.direction().is_vertical();
                            let next = |mgr: &mut Manager, s, clr, rev| {
                                if clr {
                                    mgr.clear_nav_focus();
                                }
                                mgr.next_nav_focus(s, rev);
                            };
                            let rev = self.list.inner.direction().is_reversed();
                            use Direction::*;
                            match key {
                                NavKey::Left if !inner_vert => next(mgr, self, false, !rev),
                                NavKey::Right if !inner_vert => next(mgr, self, false, rev),
                                NavKey::Up if inner_vert => next(mgr, self, false, !rev),
                                NavKey::Down if inner_vert => next(mgr, self, false, rev),
                                NavKey::Home => next(mgr, self, true, false),
                                NavKey::End => next(mgr, self, true, true),
                                NavKey::Left if dir == Right => self.close_menu(mgr),
                                NavKey::Right if dir == Left => self.close_menu(mgr),
                                NavKey::Up if dir == Down => self.close_menu(mgr),
                                NavKey::Down if dir == Up => self.close_menu(mgr),
                                key => return Response::Unhandled(Event::NavKey(key)),
                            }
                        }
                        Response::None
                    }
                    ev => Response::Unhandled(ev),
                },
                Response::Msg(msg) => {
                    self.close_menu(mgr);
                    Response::Msg(msg)
                }
                r => r,
            }
        } else {
            Manager::handle_generic(self, mgr, event)
        }
    }
}

impl<D: Directional, W: Menu> Menu for SubMenu<D, W> {
    fn menu_is_open(&self) -> bool {
        self.popup_id.is_some()
    }

    fn menu_path(&mut self, mgr: &mut Manager, target: Option<WidgetId>) {
        match target {
            Some(id) if self.is_ancestor_of(id) => {
                if self.popup_id.is_some() {
                    // We should close other sub-menus before opening
                    let mut child = None;
                    for i in 0..self.list.inner.len() {
                        if self.list.inner[i].is_ancestor_of(id) {
                            child = Some(i);
                        } else {
                            self.list.inner[i].menu_path(mgr, None);
                        }
                    }
                    if let Some(i) = child {
                        self.list.inner[i].menu_path(mgr, target);
                    }
                } else {
                    self.open_menu(mgr);
                    if id != self.id() {
                        for i in 0..self.list.inner.len() {
                            self.list.inner[i].menu_path(mgr, target);
                        }
                    }
                }
            }
            _ => {
                if self.popup_id.is_some() {
                    for i in 0..self.list.inner.len() {
                        self.list.inner[i].menu_path(mgr, None);
                    }
                    self.close_menu(mgr);
                }
            }
        }
    }
}

impl<D: Directional, W: Menu> HasText for SubMenu<D, W> {
    fn get_text(&self) -> &str {
        self.label.get(false)
    }

    fn set_cow_string(&mut self, text: CowString) -> TkAction {
        self.label = text.into();
        TkAction::Redraw
    }
}