use std::{cell::SyncUnsafeCell, collections::VecDeque};

use glam::Vec2;
use renderer::font::FontContext;
use util::Shared;

use crate::ui::Event;

static GLOBAL_CONTEXT: SyncUnsafeCell<Context> = const {
    SyncUnsafeCell::new(Context {
        events: VecDeque::new(),
        window_size: Vec2::ZERO,
        mouse_pos: Vec2::ZERO,
        font_context: None,
    })
};

pub struct Context {
    events: VecDeque<Event>,
    window_size: Vec2,
    mouse_pos: Vec2,
    font_context: Option<Shared<FontContext>>,
}

#[allow(dead_code)]
impl Context {
    pub fn get() -> &'static Self {
        unsafe { GLOBAL_CONTEXT.get().as_ref().unwrap() }
    }

    pub fn get_mut() -> &'static mut Self {
        unsafe { GLOBAL_CONTEXT.get().as_mut().unwrap() }
    }

    pub fn with<T>(f: impl FnOnce(&mut Context) -> T) -> T {
        let ctx = Self::get_mut();
        f(ctx)
    }

    pub fn set_font_context(&mut self, context: Shared<FontContext>) {
        self.font_context = Some(context);
    }

    pub fn font_context(&self) -> Shared<FontContext> {
        self.font_context.clone().unwrap()
    }

    pub fn set_mouse_pos(&mut self, pos: Vec2) {
        self.mouse_pos = pos;
    }

    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    pub fn set_window_size(&mut self, size: Vec2) {
        self.window_size = size;
    }

    pub fn window_size(&self) -> Vec2 {
        self.window_size
    }

    pub fn new_event(&mut self, event: Event) {
        self.events.push_back(event);
    }

    pub fn current_event(&self) -> Option<&Event> {
        self.events.front()
    }

    pub fn has_event(&self, event: Event) -> bool {
        self.events.iter().find(|e| **e == event).is_some()
    }

    pub fn consume_event(&mut self, event: &Event) {
        let index = self.events.iter().position(|e| e == event);
        assert!(index.is_some());
        self.events.remove(index.unwrap());
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}
