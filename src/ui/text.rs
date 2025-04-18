use glam::{Vec2, Vec3};
use renderer::Renderer;

use crate::context::Context;

use super::{SizingParams, UiComponent};
use mesh_macros::Builder;

#[derive(Builder, Default, Debug, Clone)]
#[allow(dead_code)]
pub struct Text {
    text: String,
    text_size: f32,
    #[skip]
    pos: Vec2,
    #[skip]
    size: Vec2,
}

impl UiComponent for Text {
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
        false
    }

    fn render(&self, renderer: &mut Renderer, z_index: f32) {
        renderer.add_text(
            Vec3::new(self.pos.x, self.pos.y, z_index),
            self.text_size,
            &self.text,
            &[1.0, 1.0, 1.0, 1.0],
        );
        //renderer.add_quad(self.pos, self.size, &[0.1, 0.1, 0.1, 1.0]);
    }
}
