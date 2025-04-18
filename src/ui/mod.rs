pub mod button;
pub mod bx;
pub mod dialog;
pub mod text;

use std::fmt::Debug;
use util::Shared;

use glam::Vec2;
use renderer::Renderer;
use tao::event::MouseButton;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ComponentState: u32 {
        const CLEAN = 1 << 0;
        const UPDATE = 1 << 1;
        const RERENDER = 1 << 2;

        const DIRTY = Self::UPDATE.bits() | Self::RERENDER.bits();
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    MouseDown { button: MouseButton, pos: Vec2 },
    MouseUp { button: MouseButton, pos: Vec2 },
    MouseMoved { pos: Vec2 },
}

pub trait UiComponent: Debug {
    fn layout(&mut self, root_pos: Vec2);
    fn size(&mut self, sizing_params: SizingParams);
    fn grow(&mut self, remaining_size: &Vec2, total_remaining: &Vec2);
    fn get_size(&self) -> Vec2;
    fn get_pos(&self) -> Vec2;
    fn input(&mut self) -> bool;
    fn render(&self, renderer: &mut Renderer, z_index: f32);
}

#[derive(Debug, Clone)]
pub struct Component {
    inner: Shared<dyn UiComponent>,
}

pub trait IntoComponent {
    fn into_component(self) -> Component;
}

impl<T: UiComponent + 'static> IntoComponent for Shared<T> {
    fn into_component(self) -> Component {
        Component::new(self)
    }
}

impl<T: UiComponent + 'static> IntoComponent for &Shared<T> {
    fn into_component(self) -> Component {
        Component::new(self.clone())
    }
}

impl IntoComponent for Component {
    fn into_component(self) -> Component {
        self
    }
}

pub struct SizingParams<'a> {
    pub(crate) parent_size: &'a mut Vec2,
}

impl Component {
    pub fn new(component: Shared<dyn UiComponent>) -> Self {
        Self { inner: component }
    }

    pub fn layout(&mut self, root_pos: Vec2) {
        self.inner.get_mut().layout(root_pos)
    }

    pub fn size(&mut self, sizing_params: SizingParams) {
        self.inner.get_mut().size(sizing_params)
    }

    pub fn grow(&mut self, remaining_size: &Vec2, total_remaining: &Vec2) {
        self.inner.get_mut().grow(remaining_size, total_remaining)
    }

    pub fn get_size(&self) -> Vec2 {
        self.inner.get().get_size()
    }

    pub fn get_pos(&self) -> Vec2 {
        self.inner.get().get_pos()
    }

    pub fn input(&mut self) -> bool {
        self.inner.get_mut().input()
    }

    pub fn render(&self, renderer: &mut Renderer, z_index: f32) {
        self.inner.get().render(renderer, z_index)
    }
}

pub mod runtime {
    use glam::{Vec2, Vec3};
    use std::cell::Cell;
    use std::collections::{HashMap, HashSet};
    use std::fmt::{Debug, Display, Formatter};
    use std::{any::Any, marker::PhantomData};
    use util::Shared;
    use uuid::Uuid;

    use crate::context::Context;

    use super::{Component, IntoComponent, SizingParams, UiComponent};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SignalId(Uuid);
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct EffectId(Uuid);

    pub struct Runtime {
        signals: Shared<HashMap<SignalId, Shared<Box<dyn Any>>>>,
        running_effect: Cell<Option<EffectId>>,
        signal_subscribers: Shared<HashMap<SignalId, HashSet<EffectId>>>,
        effects: Shared<HashMap<EffectId, Box<dyn Fn()>>>,
    }

    impl Runtime {
        pub fn new() -> &'static Self {
            Box::leak(Box::new(Self {
                signals: Shared::new(HashMap::new()),
                running_effect: Cell::new(None),
                signal_subscribers: Shared::new(HashMap::new()),
                effects: Shared::new(HashMap::new()),
            }))
        }

        pub fn create_signal<T: 'static + Display + Copy>(&'static self, value: T) -> Signal<T> {
            let id = SignalId(Uuid::new_v4());
            self.signals
                .get_mut()
                .insert(id, Shared::new(Box::new(value)));
            Signal {
                runtime: self,
                id,
                ty: PhantomData {},
            }
        }

        pub fn create_effect(&'static self, f: impl Fn() + 'static) {
            let id = EffectId(Uuid::new_v4());
            self.effects.get_mut().insert(id, Box::new(f));
            self.run_effect(id)
        }

        fn run_effect(&'static self, id: EffectId) {
            let prev_effect = self.running_effect.get().take();
            self.running_effect.set(Some(id));

            let effect = self.effects.get_mut().get(&id).unwrap();
            effect();

            self.running_effect.set(prev_effect);
        }
    }

    #[derive(Clone, Copy)]
    pub struct Signal<T: ?Sized + Display + Copy> {
        runtime: &'static Runtime,
        id: SignalId,
        ty: PhantomData<T>,
    }

    impl<T: Display + Copy + 'static> Signal<T> {
        pub fn get(&self) -> T {
            self.get_ref().clone()
        }

        pub fn get_ref(&self) -> &T {
            let value = self.runtime.signals.get();
            let value = value.get(&self.id).unwrap();
            let value = unsafe { value.get().downcast_ref_unchecked::<T>() };

            if let Some(effect) = self.runtime.running_effect.get() {
                let subs = self.runtime.signal_subscribers.get_mut();
                let subs = subs.entry(self.id).or_default();
                subs.insert(effect);
            }

            value
        }

        pub fn set(&self, value: T) {
            let wrapper = self.runtime.signals.get_mut().get_mut(&self.id).unwrap();
            let wrapper = unsafe { wrapper.get_mut().downcast_mut_unchecked::<T>() };
            *wrapper = value;

            if let Some(subs) = self.runtime.signal_subscribers.get().get(&self.id) {
                for sub in subs {
                    self.runtime.run_effect(*sub);
                }
            }
        }
    }

    impl<T: Display + Copy> Debug for Signal<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Signal").field("id", &self.id).finish()
        }
    }

    pub struct ValueComponent<T: Display + Copy> {
        value: Signal<T>,
        updated: Shared<bool>,
        pos: Vec2,
        size: Vec2,
    }

    impl<T: Display + Copy + PartialEq + 'static> ValueComponent<T> {
        fn new(value: Signal<T>) -> Self {
            let updated = Shared::new(false);
            value.runtime.create_effect({
                let updated = updated.clone();
                move || {
                    *updated.get_mut() = true;
                }
            });
            Self {
                value,
                updated,
                pos: Vec2::ZERO,
                size: Vec2::ZERO,
            }
        }
    }

    impl<T: Display + Copy> Debug for ValueComponent<T> {
        fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
            Ok(())
        }
    }

    impl<T: Display + Copy + PartialEq + 'static> UiComponent for ValueComponent<T> {
        fn layout(&mut self, root_pos: Vec2) {
            self.pos = root_pos;
            self.pos.y -= self.size.y;
        }

        fn size(&mut self, sizing_params: SizingParams) {
            let font = Context::get().font_context().get().last();
            self.size = font.calc_size(24.0, &self.value.get_ref().to_string());
            *sizing_params.parent_size += self.size;
        }

        fn grow(&mut self, _: &Vec2, _: &Vec2) {}

        fn get_size(&self) -> Vec2 {
            self.size
        }

        fn get_pos(&self) -> Vec2 {
            self.pos
        }

        fn input(&mut self) -> bool {
            let updated = self.updated.get().clone();
            *self.updated.get_mut() = false;
            updated
        }

        fn render(&self, renderer: &mut renderer::Renderer, z_index: f32) {
            renderer.add_text(
                Vec3::new(self.pos.x, self.pos.y, z_index),
                24.0,
                self.value.get_ref().to_string(),
                &0xFFFFFFFFu32,
            );
        }
    }

    impl<T: Display + Copy + PartialEq + 'static> IntoComponent for Signal<T> {
        fn into_component(self) -> Component {
            Component {
                inner: Shared::new(ValueComponent::new(self)),
            }
        }
    }
}
