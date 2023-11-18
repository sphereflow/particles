use egui::*;
use instant::Instant;

pub struct Gui {
    pub winit_state: egui_winit::State,
    pub scale_factor: f32,
    pub exit_app: bool,
    last_update_inst: Instant,
    last_cursor: Option<Pos2>,
}

impl Gui {
    pub fn name(&self) -> &str {
        "Particles"
    }

    pub fn update(&mut self, ctx: &Context, winit_window: &winit::window::Window) -> FullOutput {
        let input = self.winit_state.take_egui_input(winit_window);
        ctx.begin_frame(input);

        let window = Window::new("Light Garden");
        window
            .default_size(Vec2::new(300.0, 100.0))
            .show(ctx, |ui| {
                self.last_cursor = ui.input(|i| i.pointer.interact_pos());
                if let Some(mouse_pos) = self.last_cursor {
                    ui.label(format!(
                        "Mouse Position: ({:.1},{:.1})",
                        mouse_pos.x, mouse_pos.y
                    ));
                }
                self.main(ui);

                let elapsed = self.last_update_inst.elapsed();
                ui.label(format!("Frametime: {:.2?}", elapsed));
            });

        self.last_update_inst = Instant::now();
        ctx.end_frame()
    }
}

impl Gui {
    pub fn new(
        winit_window: &winit::window::Window,
        event_loop: &winit::event_loop::EventLoop<()>,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let size = winit_window.inner_size();
        let last_update_inst = Instant::now();
        let winit_state = egui_winit::State::new(&event_loop);
        Gui {
            winit_state,
            scale_factor: winit_window.scale_factor() as f32,
            last_update_inst,
            last_cursor: None,
            exit_app: false,
        }
    }

    fn main(&mut self, ui: &mut Ui) {}

    pub fn winit_update(
        &mut self,
        event: &winit::event::WindowEvent,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        use winit::event;
        use winit::event::WindowEvent;
        type Key = event::VirtualKeyCode;
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. } => {}

            WindowEvent::CursorMoved { position, .. } => {}
            WindowEvent::MouseInput {
                state: event::ElementState::Pressed,
                button: event::MouseButton::Left,
                ..
            } => {}
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Left,
                ..
            } => {}
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Right,
                ..
            } => {}
            _ => {}
        }
    }
}
