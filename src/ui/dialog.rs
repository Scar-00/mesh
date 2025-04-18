use glam::{Vec2, Vec3, Vec4, Vec4Swizzles};
use mesh_macros::Builder;
use renderer::{color::Color, Renderer};
use std::fmt::{Debug, Formatter};
use tao::event::MouseButton;
use util::Shared;

use crate::context::Context;

use super::{
    button::is_in_bounds,
    bx::{Container, Orientation, Sizing},
    Component, Event as UiEvent, IntoComponent, SizingParams, UiComponent,
};

#[derive(Builder, Default)]
#[allow(dead_code)]
pub struct Dialog {
    orientation: Orientation,
    padding: Vec4,
    spacing: Vec2,
    #[skip]
    children: Vec<Component>,
    #[skip]
    size: Vec2,
    #[skip]
    pos: Vec2,
    sizing: (Sizing, Sizing),
    parent: Option<Component>,
    color: Color,
    open: bool,
    #[skip]
    controller: Option<Controller>,
}

impl Debug for Dialog {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("orientation", &self.orientation)
            .field("padding", &self.padding)
            .field("spacing", &self.spacing)
            .field("children", &self.children)
            .field("size", &self.size)
            .field("pos", &self.pos)
            .field("sizing", &self.sizing)
            .field("color", &self.color)
            .field("open", &self.open)
            .field("controller", &self.controller)
            .finish()
    }
}

impl Dialog {
    pub fn append(&mut self, child: impl IntoComponent) {
        self.children.push(child.into_component());
    }

    pub fn open(&mut self) {
        if self.open {
            return;
        }
        let pos = Context::with(|ctx| ctx.mouse_pos());
        self.pos = pos;
        self.pos.y -= self.size.y;
        self.open = true;
    }

    pub fn add_controller(&mut self, controller: Controller) {
        self.controller = Some(controller);
    }

    fn default_controller(dialog: &mut Self, parent: &Component, event: &UiEvent) -> bool {
        if let UiEvent::MouseUp { button, pos } = event {
            if is_in_bounds(parent.get_pos(), parent.get_size(), *pos)
                && matches!(button, MouseButton::Right)
            {
                Context::get_mut().consume_event(&event);
                dialog.open();
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl Container for Shared<Dialog> {
    fn append(&self, child: impl IntoComponent) {
        self.get_mut().append(child)
    }
}

impl UiComponent for Dialog {
    fn layout(&mut self, _: Vec2) {
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
        let event = {
            let Some(event) = Context::get().current_event() else {
                return false;
            };
            event.clone()
        };
        let rerender = if self.open {
            if let UiEvent::MouseUp { pos, .. } = &event {
                if !is_in_bounds(self.pos, self.size, *pos) {
                    Context::get_mut().consume_event(&event);
                    self.open = false;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            if let Some(parent) = self.parent.clone() {
                if let Some(controller) = &self.controller {
                    (controller.input_callback)(&parent, &event)
                } else {
                    Dialog::default_controller(self, &parent, &event)
                }
            } else {
                tracing::error!("dialog without parent");
                assert!(false);
                unreachable!();
            }
        };
        if !self.open {
            return rerender;
        }

        self.children
            .iter_mut()
            .map(|child| child.input())
            .any(|output| output)
            || rerender
    }

    fn render(&self, renderer: &mut Renderer, z_index: f32) {
        if !self.open {
            return;
        }
        self.children
            .iter()
            .for_each(|child| child.render(renderer, z_index + 0.2));
        renderer.add_quad_v3(
            Vec3::new(self.pos.x, self.pos.y, z_index + 0.1),
            Vec3::new(self.size.x, self.size.y, 0.0),
            &self.color,
        );
    }
}

pub struct Controller {
    input_callback: Box<dyn Fn(&Component, &UiEvent) -> bool>,
}

impl std::fmt::Debug for Controller {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Controller {
    pub fn new(callback: impl Fn(&Component, &UiEvent) -> bool + 'static) -> Self {
        Self {
            input_callback: Box::new(callback),
        }
    }
}
