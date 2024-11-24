use chip8::*;

use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(include_bytes!("test_opcode.ch8"));
    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    window: Option<Window>,
    pixels: Option<Pixels>,

    program: Chip8,
}

impl App {
    fn new(program: &[u8]) -> Self {
        Self {
            window: None,
            pixels: None,

            program: Chip8::new(program, None),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(64, 32, surface_texture).unwrap()
        };

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(pixels) = &mut self.pixels else {
            return;
        };

        let Some(window) = &self.window else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                pixels.render().unwrap();
            }
            _ => (),
        }

        self.program.update(
            |d| {
                let mut display = Vec::new();

                for b in d {
                    display.extend_from_slice(&[
                        b & 128 != 0,
                        b & 64 != 0,
                        b & 32 != 0,
                        b & 16 != 0,
                        b & 8 != 0,
                        b & 4 != 0,
                        b & 2 != 0,
                        b & 1 != 0,
                    ]);
                }

                for (p, d) in pixels.frame_mut().chunks_exact_mut(4).zip(display) {
                    p.copy_from_slice(if d {
                        &[0xFF, 0xFF, 0xFF, 0xFF]
                    } else {
                        &[0x00, 0x00, 0x00, 0xFF]
                    });
                }

                window.request_redraw();
            },
            || {},
        );
    }
}
