use cgmath::Vector2;
use egui::*;
use egui_plot::{Line, Plot};
use instant::Instant;

use crate::{camera::Camera, poly7::Poly7, App, SimParams};

pub struct Gui {
    pub winit_state: egui_winit::State,
    pub scale_factor: f32,
    pub exit_app: bool,
    element_text: [String; 5],
    last_update_inst: Instant,
    last_cursor: Option<Pos2>,
    poly_index: usize,
    copy_poly: Option<Poly7>,
}

impl Gui {
    pub fn update(
        &mut self,
        ctx: &Context,
        winit_window: &winit::window::Window,
        app: &mut App,
    ) -> FullOutput {
        let input = self.winit_state.take_egui_input(winit_window);
        ctx.begin_frame(input);

        let window = Window::new("Particles");
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
                self.main(ui, app);

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
    ) -> Self {
        let last_update_inst = Instant::now();
        let winit_state = egui_winit::State::new(ViewportId::ROOT, event_loop, None, None);
        let element_text = [
            String::from("Earth"),
            String::from("Water"),
            String::from("Fire"),
            String::from("Air"),
            String::from("Ether"),
        ];
        Gui {
            winit_state,
            scale_factor: winit_window.scale_factor() as f32,
            last_update_inst,
            last_cursor: None,
            exit_app: false,
            element_text,
            poly_index: 0,
            copy_poly: None,
        }
    }

    fn main(&mut self, ui: &mut Ui, app: &mut App) {
        let mut num_particles = app.psys.particles.len();
        if ui.add(Slider::new(&mut num_particles, 1..=50000)).changed() {
            app.psys.set_num_particles(num_particles);
            app.compute.upload_particles(&app.renderer.device, &app.psys.particles)
        }
        ui.vertical_centered_justified(|ui| {
            Self::edit_time_controls(ui, app);
            self.edit_cutoff(ui, &mut app.sim_params);
            Self::edit_view_distance(ui, app);
            Self::edit_camera_speed(ui, &mut app.renderer.camera);
            Self::edit_distance_exponent(ui, &mut app.sim_params);
            Self::edit_bounding_volume_radius(ui, app);
        });
        ui.horizontal(|ui| {
            ui.separator();
            self.edit_masses(ui, &mut app.sim_params);
            ui.separator();
            self.edit_polys(ui);
        });
        self.edit_poly(ui, &mut app.sim_params.attraction_force[self.poly_index]);
    }

    fn edit_time_controls(ui: &mut Ui, app: &mut App) {
        if app.speed.is_some() {
            if ui.button("pause").clicked() {
                app.speed = None;
            }
        } else if ui.button("play").clicked() {
            app.speed = Some(1.0);
        }
        if let Some(speed) = app.speed.as_mut() {
            ui.horizontal(|ui| {
                ui.label("speedup: ");
                ui.add(Slider::new(speed, 0.1..=10.).logarithmic(true));
            });
        }
    }

    fn edit_camera_speed(ui: &mut Ui, camera: &mut Camera) {
        ui.horizontal(|ui| {
            ui.label("camera speed");
            ui.add(Slider::new(&mut camera.units_per_second, 2.0..=20.0).logarithmic(true));
        });
    }

    fn edit_view_distance(ui: &mut Ui, app: &mut App) {
        if let Some((_, distance)) = app.renderer.camera.look_at_distance.as_mut() {
            ui.horizontal(|ui| {
                ui.label("view distance: ");
                ui.add(Slider::new(distance, 0.1..=20.0).logarithmic(true));
            });
        }
    }

    fn edit_distance_exponent(ui: &mut Ui, sim_params: &mut SimParams) {
        ui.horizontal(|ui| {
            ui.label("distance exponent: ");
            ui.add(Slider::new(&mut sim_params.distance_exponent, -5.0..=5.0));
        });
    }

    fn edit_bounding_volume_radius(ui: &mut Ui, app: &mut App) {
        ui.horizontal(|ui| {
            ui.label("bounding volume size :");
            let mut val = app.sim_params.bounding_volume_radius * 2.0;
            if ui.add(Slider::new(&mut val, 0.5..=10.0)).changed() {
                app.sim_params.bounding_volume_radius = val * 0.5;
                app.psys
                    .force_grid
                    .bounds
                    .set_centered(app.sim_params.bounding_volume_radius * 2.0);
            }
        });
    }

    fn edit_polys(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.colored_label(Color32::GREEN, "polynome selection matrix");
            for y in 0..5 {
                ui.horizontal(|ui| {
                    for x in 0..5 {
                        ui.radio_value(&mut self.poly_index, x + y * 5, "");
                    }
                });
            }
        });
    }

    fn edit_poly(&mut self, ui: &mut Ui, poly: &mut Poly7) {
        ui.colored_label(Color32::GREEN, "selected attraction_force polynome");
        let line = Line::new(poly.plot_points());
        Plot::new("poly plot")
            .view_aspect(2.0)
            .show(ui, |plot_ui| plot_ui.line(line));
        ui.horizontal(|ui| {
            let xs: [f32; 8] = std::array::from_fn(|i| (i as f32) / 7.0);
            let mut ys = xs.map(|x| poly.eval(x));
            let mut changed = false;
            for y in ys.iter_mut() {
                changed |= ui
                    .add(Slider::new(y, -10.0..=10.0).orientation(SliderOrientation::Vertical))
                    .changed();
            }
            if changed {
                let points = [
                    Vector2::new(xs[0], ys[0]),
                    Vector2::new(xs[1], ys[1]),
                    Vector2::new(xs[2], ys[2]),
                    Vector2::new(xs[3], ys[3]),
                    Vector2::new(xs[4], ys[4]),
                    Vector2::new(xs[5], ys[5]),
                    Vector2::new(xs[6], ys[6]),
                    Vector2::new(xs[7], ys[7]),
                ];
                if let Some(p) = Poly7::from_points(points) {
                    *poly = p;
                }
            }
        });
        ui.horizontal(|ui| {
            for (i, n) in (0..8).zip(Poly7::coeff_names()) {
                Gui::labeled_drag_value(ui, &mut poly.coeffs[i], n);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("copy").clicked() {
                self.copy_poly = Some(*poly);
            }
            if let Some(cp) = self.copy_poly {
                if ui.button("paste").clicked() {
                    *poly = cp;
                }
            }
            if ui.button("invert").clicked() {
                poly.invert();
            }
            if ui.button("zero").clicked() {
                *poly = Poly7::zero();
            }
        });
    }

    fn labeled_drag_value(ui: &mut Ui, val: &mut f32, label: &str) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(DragValue::new(val).speed(0.01));
        });
    }

    fn edit_masses(&self, ui: &mut Ui, sim_params: &mut SimParams) {
        ui.vertical(|ui| {
            ui.colored_label(Color32::GREEN, "Masses");
            for (i, mass) in sim_params.particle_type_masses.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut mass.mass)
                            .prefix(&self.element_text[i])
                            .speed(0.01)
                            .clamp_range(0.01..=10.0),
                    );
                });
            }
        });
    }

    fn edit_cutoff(&self, ui: &mut Ui, sim_params: &mut SimParams) {
        ui.horizontal(|ui| {
            ui.label("polynome cutoff distance: ");
            ui.add(Slider::new(&mut sim_params.cut_off_distance, 0.1..=5.0));
        });
    }
}
