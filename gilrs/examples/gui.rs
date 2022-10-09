#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crate::egui::plot::{MarkerShape, PlotPoints, Points};
use crate::egui::RichText;
use eframe::egui;
use eframe::egui::Vec2;
use gilrs::ev::AxisOrBtn;
use gilrs::{Axis, GamepadId, Gilrs, GilrsBuilder};
use std::time::UNIX_EPOCH;
use uuid::Uuid;

struct MyEguiApp {
    gilrs: Gilrs,
    current_gamepad: Option<GamepadId>,
    log_messages: [Option<String>; 300],
}

impl Default for MyEguiApp {
    fn default() -> Self {
        const INIT: Option<String> = None;
        Self {
            gilrs: GilrsBuilder::new().set_update_state(false).build().unwrap(),
            current_gamepad: None,
            log_messages: [INIT; 300],
        }
    }
}

impl MyEguiApp {
    fn log(&mut self, message: String) {
        self.log_messages[0..].rotate_right(1);
        self.log_messages[0] = Some(message);
    }
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(event) = self.gilrs.next_event() {
            self.log(format!(
                "{} : {} : {:?}",
                event
                    .time
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
                event.id,
                event.event
            ));
            self.gilrs.update(&event);
            if self.current_gamepad.is_none() {
                self.current_gamepad = Some(event.id);
            }
        }

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Controllers");
            ui.separator();

            for (id, gamepad) in self.gilrs.gamepads() {
                if ui
                    .selectable_label(
                        self.current_gamepad == Some(id),
                        format!("{id}: {}", gamepad.name()),
                    )
                    .clicked()
                {
                    self.current_gamepad = Some(id);
                };
            }
            ui.allocate_space(ui.available_size());
        });

        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.heading("Event Log");
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        for message in self.log_messages.iter().flatten() {
                            ui.label(message);
                        }
                        ui.allocate_space(ui.available_size());
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                if let Some(gamepad_id) = self.current_gamepad {
                    let gamepad = self.gilrs.gamepad(gamepad_id);
                    let gamepad_state = gamepad.state();
                    ui.vertical(|ui| {
                        ui.heading("Info");
                        egui::Grid::new("info_grid")
                            .striped(true)
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Name");
                                ui.label(gamepad.name());
                                ui.end_row();

                                ui.label("Gilrs ID");
                                ui.label(gamepad.id().to_string());
                                ui.end_row();

                                if let Some(map_name) = gamepad.map_name() {
                                    ui.label("Map Name");
                                    ui.label(map_name);
                                    ui.end_row();
                                }

                                ui.label("Map Source");
                                ui.label(format!("{:?}", gamepad.mapping_source()));
                                ui.end_row();

                                ui.label("Uuid");
                                let uuid = Uuid::from_bytes(gamepad.uuid()).to_string();
                                ui.horizontal(|ui| {
                                    ui.label(&uuid);
                                    if ui.button("Copy").clicked() {
                                        ui.output().copied_text = uuid;
                                    }
                                });
                                ui.end_row();
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(250.0);
                            ui.heading("Buttons");

                            for (code, button_data) in gamepad_state.buttons() {
                                let name = match gamepad.axis_or_btn_name(code) {
                                    Some(AxisOrBtn::Btn(b)) => format!("{b:?}"),
                                    _ => code.to_string(),
                                };

                                ui.add(
                                    egui::widgets::ProgressBar::new(button_data.value()).text(
                                        RichText::new(format!(
                                            "{name:<13} {:<5} {:.4} {:>5}",
                                            button_data.is_pressed(),
                                            button_data.value(),
                                            code.into_u32()
                                        ))
                                        .monospace(),
                                    ),
                                );
                            }
                        });
                        ui.vertical(|ui| {
                            ui.set_width(300.0);
                            ui.heading("Axes");
                            ui.horizontal(|ui| {
                                for (name, x, y) in [
                                    ("Left Stick", Axis::LeftStickX, Axis::LeftStickY),
                                    ("Right Stick", Axis::RightStickX, Axis::RightStickY),
                                ] {
                                    ui.vertical(|ui| {
                                        ui.label(name);
                                        let y_axis = gamepad
                                            .axis_data(y)
                                            .map(|a| a.value())
                                            .unwrap_or_default()
                                            as f64;
                                        let x_axis = gamepad
                                            .axis_data(x)
                                            .map(|a| a.value())
                                            .unwrap_or_default()
                                            as f64;
                                        egui::widgets::plot::Plot::new(format!("{name}_plot"))
                                            .width(150.0)
                                            .height(150.0)
                                            .min_size(Vec2::splat(3.25))
                                            .include_x(1.25)
                                            .include_y(1.25)
                                            .include_x(-1.25)
                                            .include_y(-1.25)
                                            .allow_drag(false)
                                            .allow_zoom(false)
                                            .allow_boxed_zoom(false)
                                            .allow_scroll(false)
                                            .show(ui, |plot_ui| {
                                                plot_ui.points(
                                                    Points::new(PlotPoints::new(vec![[
                                                        x_axis, y_axis,
                                                    ]]))
                                                    .shape(MarkerShape::Circle)
                                                    .radius(4.0),
                                                );
                                            });
                                    });
                                }
                            });
                            for (code, axis_data) in gamepad_state.axes() {
                                let name = match gamepad.axis_or_btn_name(code) {
                                    None => code.to_string(),
                                    Some(AxisOrBtn::Btn(b)) => format!("{b:?}"),
                                    Some(AxisOrBtn::Axis(a)) => format!("{a:?}"),
                                };
                                ui.add(
                                    egui::widgets::ProgressBar::new(
                                        (axis_data.value() * 0.5) + 0.5,
                                    )
                                    .text(
                                        RichText::new(format!("{:+.4} {name}", axis_data.value()))
                                            .monospace(),
                                    ),
                                );
                            }
                        });
                    });
                } else {
                    ui.label("Press a button on a controller or select it from the left.");
                }
                ui.allocate_space(ui.available_size());
            });
        });

        ctx.request_repaint();
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Gilrs Input Tester",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    );
}
