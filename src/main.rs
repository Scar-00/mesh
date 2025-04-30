#![feature(sync_unsafe_cell, downcast_unchecked)]
mod context;
mod ui;

use context::Context;
use renderer::color::Color;
use renderer::font::FontContext;
use ui::dialog::Dialog;
use ui::{ComponentState, IntoComponent};
use util::Shared;

use std::sync::Arc;
use std::{error::Error, rc::Rc};

use glam::{Vec2, Vec4};
use renderer::Renderer;
use tao::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::WindowBuilder,
};
use tracing_subscriber::prelude::*;
use ui::{
    button::Button,
    bx::{Bx, Container, Sizing},
    Event as UiEvent, SizingParams,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_new(format!(
                "{}=trace,wgpu=error,renderer=trace",
                env!("CARGO_CRATE_NAME")
            ))
            .unwrap(),
        )
        .with(tracing_error::ErrorLayer::default())
        .init();

    let event_loop = EventLoop::new();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("test")
            .with_inner_size(LogicalSize::new(1280, 720))
            //.with_resizable(false)
            .build(&event_loop)?,
    );

    let font_context = Shared::new(FontContext::new()?);
    let mut renderer = Renderer::new(window.clone(), font_context.clone()).await?;
    Context::get_mut().set_font_context(font_context);

    let runtime = ui::runtime::Runtime::new();

    let value = runtime.create_signal(10);

    let test_ui = mesh_macros::view! {
        Bx(
            padding(Vec4::splat(8.0)),
            spacing(Vec2::splat(8.0)),
            sizing((Sizing::Expand, Sizing::Expand)),
        ) {
            Dialog(
                sizing((Sizing::Auto, Sizing::Auto)),
                color(Color::new([0.0, 0.0, 1.0, 1.0])),
                padding(Vec4::splat(8.0)),
                parent(Some(this.clone().into_component())),
            ) {
                Button(
                    on_click(Some(Rc::new({
                        let value = value.clone();
                        move |_| {
                            value.set(value.get() - 1);
                        }
                    }))),
                    text("-".into())
                ) {

                },
                {value},
                Button(
                    on_click(Some(Rc::new({
                        let value = value.clone();
                        move |_| {
                            value.set(value.get() + 1);
                        }
                    }))),
                    text("+".into())
                ) {

                }
            }
        }
    };

    println!("ui: {:#?}", test_ui);

    /*let test_ui = mesh_macros::view! {
        Bx(
            padding(Vec4::splat(8.0)),
            spacing(Vec2::splat(8.0)),
            sizing((Sizing::Expand, Sizing::Expand))
        ) {
            Bx(
                padding(Vec4::splat(4.0)),
                color(Color::new(0xFF00FFFFu32)),
                sizing((Sizing::Partial(Percentage::new(20)), Sizing::Expand))
            ) {

            },
            Bx(
                padding(Vec4::splat(8.0)),
                spacing(Vec2::splat(8.0)),
                sizing((Sizing::Partial(Percentage::new(80)), Sizing::Expand))
            ) {
            },
            Dialog(
                sizing((Sizing::Auto, Sizing::Auto)),
                color(Color::new([0.0, 0.0, 1.0, 1.0])),
                padding(Vec4::splat(8.0)),
                parent(Some(this.clone().into_component())),
            ) {
                Button(
                    text("Test".into()),
                    on_click(Some(Rc::new(|btn| {
                        btn.text = "test-text".into();
                        println!("test??");
                    })))
                ) {}
            }
        }
    };*/

    let mut component = test_ui.into_component();
    let mut window_size = Vec2::new(
        window.inner_size().width as f32,
        window.inner_size().height as f32,
    );
    Context::get_mut().set_window_size(window_size);

    /*let trigger_function = {
        let window = window.clone();
        move || {
            window.request_redraw();
        }
    };*/

    let mut ui_state = ComponentState::DIRTY;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: KeyCode::Escape,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: KeyCode::Tab,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    window.request_redraw();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = Vec2::new(position.x as f32, window_size.y - position.y as f32);
                    Context::with(|ctx| {
                        ctx.set_mouse_pos(pos);
                        ctx.new_event(UiEvent::MouseMoved { pos });
                    });
                }
                WindowEvent::MouseInput { state, button, .. } => match state {
                    ElementState::Pressed => Context::with(|ctx| {
                        ctx.new_event(UiEvent::MouseDown {
                            button,
                            pos: ctx.mouse_pos(),
                        });
                    }),
                    ElementState::Released => Context::with(|ctx| {
                        ctx.new_event(UiEvent::MouseUp {
                            button,
                            pos: ctx.mouse_pos(),
                        });
                    }),
                    _ => unreachable!(),
                },
                WindowEvent::Resized(new) => {
                    renderer.on_resize(new);
                    window_size = Vec2::new(
                        window.inner_size().width as f32,
                        window.inner_size().height as f32,
                    );
                    Context::get_mut().set_window_size(window_size);
                    ui_state = ComponentState::DIRTY;
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                if component.input() {
                    window.request_redraw();
                }
                Context::get_mut().clear_events();
            }
            Event::RedrawRequested(_) => {
                tracing::trace!("rerender");
                //if ui_state.contains(ComponentState::UPDATE) {
                component.size(SizingParams {
                    parent_size: &mut window_size.clone(),
                });
                component.grow(&window_size, &window_size);
                component.layout(Vec2::new(0.0, window_size.y));
                //}
                //if ui_state.contains(ComponentState::RERENDER) {
                component.render(&mut renderer, 0.0);
                _ = renderer.render();
                //}
                ui_state = ComponentState::CLEAN;
            }
            _ => {}
        }
    });
}
