#![allow(dead_code)]

pub mod context;
pub mod util;

use std::error::Error;
use std::sync::Arc;

use context::RenderContext;
use tracing::{debug, error};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let window_size = window.inner_size();

        debug!(size = ?window_size, "window created");

        self.render_context.replace(RenderContext::new(&window));
        self.window.replace(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                debug!("closing window and exiting event loop");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => 'event: {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                let Some(context) = self.render_context.as_ref() else {
                    break 'event;
                };

                context.draw_demo();

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => 'event: {
                let Some(context) = self.render_context.as_ref() else {
                    break 'event;
                };

                if let Err(err) = context.resize(size) {
                    error!(?err);
                    break 'event;
                }
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new().unwrap();

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default())?;

    Ok(())
}
