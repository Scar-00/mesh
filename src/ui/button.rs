use std::rc::Rc;

use glam::{Vec2, Vec3};
use renderer::{color::Color, Renderer};
use tao::event::MouseButton;

use crate::context::Context;

use super::{Event, SizingParams, UiComponent};
use mesh_macros::Builder;

#[derive(Builder, Default)]
pub struct Button {
    pub text: String,
    text_size: f32,
    #[skip]
    pos: Vec2,
    #[skip]
    size: Vec2,
    #[skip]
    color: Color,
    on_click: Option<Rc<dyn Fn(&mut Button)>>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub fn is_in_bounds(e_pos: Vec2, e_size: Vec2, pos: Vec2) -> bool {
    return (pos.x >= e_pos.x && pos.x <= e_pos.x + e_size.x)
        && (pos.y >= e_pos.y && pos.y <= e_pos.y + e_size.y);
}

impl UiComponent for Button {
    fn layout(&mut self, root_pos: Vec2) {
        self.pos = root_pos;
        self.pos.y -= self.size.y;
    }

    fn size(&mut self, parent_size: SizingParams) {
        let font = Context::get().font_context().get().last();
        if self.text_size == 0.0 {
            self.text_size = 24.0;
        }
        self.size = font.calc_size(self.text_size, &self.text);
        *parent_size.parent_size += self.size;
    }

    fn get_size(&self) -> Vec2 {
        self.size
    }

    fn get_pos(&self) -> Vec2 {
        self.pos
    }

    fn grow(&mut self, _: &Vec2, _: &Vec2) {}

    fn input(&mut self) -> bool {
        let event = {
            let Some(event) = Context::get().current_event() else {
                return false;
            };
            event.clone()
        };
        match event {
            Event::MouseUp { button, pos } => {
                if is_in_bounds(self.pos, self.size, pos) && matches!(button, MouseButton::Left) {
                    Context::get_mut().consume_event(&event);
                    return if let Some(f) = self.on_click.clone() {
                        f(self);
                        true
                    } else {
                        false
                    };
                }
            }
            Event::MouseMoved { pos } => {
                let color = if is_in_bounds(self.pos, self.size, pos) {
                    Color::new([1.0, 0.0, 1.0, 1.0])
                } else {
                    Color::new([1.0, 1.0, 1.0, 1.0])
                };
                let changed = self.color != color;
                self.color = color;
                return changed;
            }
            _ => return false,
        }
        return false;
    }

    fn render(&self, renderer: &mut Renderer, z_index: f32) {
        renderer.add_text(
            Vec3::new(self.pos.x, self.pos.y, z_index),
            self.text_size,
            &self.text,
            &self.color,
        );
    }
}
