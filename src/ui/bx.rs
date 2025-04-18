use glam::{swizzles::*, Vec2, Vec3, Vec4};
use mesh_macros::Builder;
use renderer::{color::Color, Renderer};

use super::{Component, IntoComponent, SizingParams, UiComponent};
use util::Shared;

pub trait Container {
    fn append(&self, child: impl IntoComponent);
}

#[derive(Debug, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Default)]
pub struct Percentage(u8);

impl Percentage {
    pub fn new(percentage: u8) -> Self {
        assert!(percentage <= 100);
        Self(percentage)
    }

    pub fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

#[derive(Debug, Default)]
pub enum Sizing {
    Fixed(f32),
    #[default]
    Auto,
    Expand,
    Partial(Percentage),
}

#[derive(Builder, Debug)]
pub struct Bx {
    orientation: Orientation,
    padding: Vec4,
    spacing: Vec2,
    sizing: (Sizing, Sizing),
    #[skip]
    children: Vec<Component>,
    #[skip]
    size: Vec2,
    #[skip]
    pos: Vec2,
    color: Color,
}

impl Default for Bx {
    fn default() -> Self {
        Self {
            padding: Vec4::splat(4.0),
            spacing: Vec2::splat(4.0),
            orientation: Default::default(),
            sizing: (Default::default(), Default::default()),
            children: Default::default(),
            size: Default::default(),
            pos: Default::default(),
            color: Color::new(0x18181818u32),
        }
    }
}

impl Bx {
    pub fn append(&mut self, child: impl IntoComponent) {
        self.children.push(child.into_component());
    }
}

impl Container for Shared<Bx> {
    fn append(&self, child: impl IntoComponent) {
        self.get_mut().append(child)
    }
}

impl UiComponent for Bx {
    fn layout(&mut self, root_pos: Vec2) {
        self.pos = root_pos;
        self.pos.y -= self.size.y;
        let mut offset = self.padding.xy();
        for child in &mut self.children {
            match self.orientation {
                Orientation::Horizontal
                    if offset.x + child.get_size().x > (self.size.x - self.padding.z) =>
                {
                    offset.x = self.padding.x;
                    offset.y += child.get_size().y + self.spacing.y;
                }
                Orientation::Vertical
                    if offset.y + child.get_size().y > (self.size.y - self.padding.w) =>
                {
                    offset.y = self.padding.y;
                    offset.x += child.get_size().x + self.spacing.x;
                }
                _ => {}
            }
            let child_layout =
                Vec2::new(self.pos.x + offset.x, self.pos.y + self.size.y - offset.y);
            child.layout(child_layout);
            match self.orientation {
                Orientation::Horizontal => {
                    offset.x += child.get_size().x + self.spacing.x;
                }
                Orientation::Vertical => {
                    offset.y += child.get_size().y + self.spacing.y;
                }
            }
        }
    }

    fn size(&mut self, size_params: SizingParams) {
        self.size = Vec2::ZERO;
        self.pos = Vec2::ZERO;
        let chilren_count = self.children.len() as f32;
        match self.orientation {
            Orientation::Horizontal => {
                self.size.x += self.spacing.x * (chilren_count - 1.0);
            }
            Orientation::Vertical => {
                self.size.y += self.spacing.y * (chilren_count - 1.0);
            }
        }
        self.children.iter_mut().for_each(|child| {
            child.size(SizingParams {
                parent_size: &mut self.size,
            })
        });
        self.size.x += self.padding.x + self.padding.z;
        self.size.y += self.padding.y + self.padding.w;

        if let Sizing::Fixed(size) = self.sizing.0 {
            if self.size.x > size {
                tracing::error!("required size exceeeds fixed size of element");
                debug_assert!(false);
            }
            self.size.x = size;
        }
        if let Sizing::Fixed(size) = self.sizing.1 {
            if self.size.y > size {
                tracing::error!("required size exceeeds fixed size of element");
                debug_assert!(false);
            }
            self.size.y = size;
        }

        let parent_size = size_params.parent_size;
        match self.orientation {
            Orientation::Horizontal => {
                parent_size.x += self.size.x;
                parent_size.y = parent_size.y.max(self.size.y);
            }
            Orientation::Vertical => {
                parent_size.y += self.size.y;
                parent_size.x = parent_size.x.max(self.size.x);
            }
        }
    }

    fn grow(&mut self, remaining_size: &Vec2, total_remaining: &Vec2) {
        let (x, y) = &self.sizing;

        if let Sizing::Expand = x {
            self.size.x = remaining_size.x;
        }
        if let Sizing::Expand = y {
            self.size.y = remaining_size.y;
        }
        if let Sizing::Partial(p) = x {
            self.size.x = (p.as_f32() / 100.0) * total_remaining.x;
        }
        if let Sizing::Partial(p) = y {
            self.size.y = (p.as_f32() / 100.0) * total_remaining.y;
        }
        let mut size_available = self.size;
        size_available.x -= self.padding.x + self.padding.z;
        size_available.y -= self.padding.y + self.padding.w;
        size_available.x -= (self.children.len() as f32 - 1.0) * self.spacing.x;
        let total_remaining = size_available.clone();
        size_available.x -= self
            .children
            .iter()
            .map(|child| child.get_size().x)
            .sum::<f32>();
        self.children
            .iter_mut()
            .for_each(|child| child.grow(&size_available, &total_remaining));
    }

    fn get_size(&self) -> Vec2 {
        self.size
    }

    fn get_pos(&self) -> Vec2 {
        self.pos
    }

    fn input(&mut self) -> bool {
        self.children
            .iter_mut()
            .map(|child| child.input())
            .any(|output| output)
    }

    fn render(&self, renderer: &mut Renderer, z_index: f32) {
        self.children
            .iter()
            .for_each(|child| child.render(renderer, z_index + 0.1));
        renderer.add_quad_v3(
            Vec3::new(self.pos.x, self.pos.y, z_index),
            Vec3::new(self.size.x, self.size.y, 0.0),
            &self.color,
        );
    }
}
