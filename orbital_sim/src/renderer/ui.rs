// src/renderer/ui.rs
// Professional Mission Legend and HUD (AGI/STK Style)

use egui::{Context, Window, Color32, Align2, FontId, Stroke, Vec2};
use crate::simulation::state::{SimState, SimPhase};
use crate::simulation::body::BodyType;
use crate::renderer::camera::Camera;

pub fn draw_hud(
    ctx: &Context, 
    sim: &mut SimState, 
    camera: &mut Camera,
    selected_body: &mut Option<u64>,
    search_query: &mut String,
    _screen_size: (u32, u32),
) {
    // ─── MISSION DATA (Top Left) ────────────────────────────────────────────
    Window::new("📊 MISSION TELEMETRY")
        .anchor(Align2::LEFT_TOP, [20.0, 20.0])
        .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(180)))
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("ORBITAL COLLISION SCENARIO").strong().color(Color32::LIGHT_BLUE));
            ui.separator();
            ui.label(format!("MET (Mission Elapsed Time): {:.2} s", sim.time));
            
            // Asset Counters
            let mut sat_count = 0;
            let mut deb_count = 0;
            for body in &sim.bodies {
                match body.body_type {
                    BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya => sat_count += 1,
                    _ => deb_count += 1,
                }
            }
            
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🛰️ ACTIVE ASSETS:").strong().color(Color32::from_rgb(0, 255, 255)));
                ui.label(format!("{}", sat_count));
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🧱 ORBITAL DEBRIS:").strong().color(Color32::from_rgb(255, 200, 0)));
                ui.label(format!("{}", deb_count));
            });

            ui.separator();
            let phase_text = match sim.phase {
                SimPhase::PreCollision => "PRE-INTERCEPT",
                SimPhase::CollisionFlash(_) => "COLLISION EVENT",
                SimPhase::PostCollision => "POST-COLLISION FRAGMENTATION",
            };
            ui.horizontal(|ui| {
                ui.label("STATUS:");
                ui.colored_label(Color32::YELLOW, phase_text);
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("SIM SPEED:");
                ui.add(egui::Slider::new(&mut sim.time_scale, 0.0..=1000.0).suffix("x"));
            });
            
            ui.horizontal(|ui| {
                if ui.button("1x").clicked() { sim.time_scale = 1.0; }
                if ui.button("100x").clicked() { sim.time_scale = 100.0; }
                if ui.button("500x").clicked() { sim.time_scale = 500.0; }
            });

            ui.separator();
            let pause_btn_text = if sim.paused { "▶ RESUME" } else { "⏸ PAUSE" };
            if ui.button(pause_btn_text).clicked() {
                sim.paused = !sim.paused;
            }

            if sim.paused {
                ui.add_space(5.0);
                ui.colored_label(Color32::RED, egui::RichText::new("⚠ SIMULATION PAUSED").strong().size(16.0));
            }
        });

    // ─── SATELLITE SEARCH (Top Center) ──────────────────────────────────────
    Window::new("🛰️ SATELLITE SEARCH")
        .anchor(Align2::CENTER_TOP, [0.0, 20.0])
        .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(200)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(search_query);
            });

            if !search_query.is_empty() {
                ui.separator();
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    let query = search_query.to_lowercase();
                    let mut found = 0;
                    for body in &sim.bodies {
                        if body.name.to_lowercase().contains(&query) {
                            let color = body.effective_color();
                            let egui_color = Color32::from_rgb(
                                (color[0] * 255.0) as u8,
                                (color[1] * 255.0) as u8,
                                (color[2] * 255.0) as u8,
                            );
                            
                            let type_str = match body.body_type {
                                BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya => "🛰️ SAT",
                                _ => "🧱 DEB",
                            };

                            if ui.button(egui::RichText::new(format!("{} | {}", type_str, body.name)).color(egui_color)).clicked() {
                                *selected_body = Some(body.id);
                                camera.focus_on(body.pos);
                            }
                            found += 1;
                            if found >= 10 { break; }
                        }
                    }
                    if found == 0 {
                        ui.label("No matches found.");
                    }
                });
            }
        });

    // ─── MISSION KEY / LEGEND (Bottom Left) ─────────────────────────────────
    Window::new("🗺️ MISSION KEY")
        .anchor(Align2::LEFT_BOTTOM, [20.0, -20.0])
        .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(180)))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                legend_item(ui, "ACTIVE ASSETS (CYAN)", Color32::from_rgb(0, 200, 255), true);
                legend_item(ui, "COSMOS / RUSSS", Color32::RED, true);
                legend_item(ui, "IRIDIUM-33", Color32::YELLOW, true);
                legend_item(ui, "PRIMARY DEBRIS (PINK)", Color32::from_rgb(255, 80, 200), false);
                legend_item(ui, "CASCADE DEBRIS (GOLD)", Color32::from_rgb(255, 200, 0), false);
                legend_item(ui, "PRE-EXISTING DEBRIS", Color32::GRAY, false);
                ui.separator();
                ui.label(egui::RichText::new("• Dots: Moving Assets").size(10.0));
                ui.label(egui::RichText::new("• Lines: Orbital Paths").size(10.0));
            });
        });

    // ─── COLLISION DATA (Top Right) ─────────────────────────────────────────
    if let Some(ev) = sim.collision_events.last() {
        Window::new("⚠ EVENT ALERT")
            .anchor(Align2::RIGHT_TOP, [-20.0, 20.0])
            .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_rgba_unmultiplied(100, 0, 0, 180)))
            .show(ctx, |ui| {
                ui.heading("PRIMARY COLLISION DETECTED");
                ui.label(format!("Time of Impact: {:.1} s", ev.time));
                ui.label(format!("Primary Objects: {} + {}", ev.body_a_name, ev.body_b_name));
                ui.label(format!("Tracked Fragments: {}", ev.new_debris_count));
            });
    }

    // ─── SELECTED OBJECT (Bottom Right) ─────────────────────────────────────
    if let Some(id) = *selected_body {
        if let Some(body) = sim.bodies.iter().find(|b| b.id == id) {
            Window::new("🛰️ SELECTED OBJECT")
                .anchor(Align2::RIGHT_BOTTOM, [-20.0, -20.0])
                .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(200)))
                .show(ctx, |ui| {
                    let color = body.effective_color();
                    let egui_color = Color32::from_rgb(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                    );
                    ui.label(egui::RichText::new(&body.name).strong().color(egui_color));
                    ui.separator();
                    ui.label(format!("Pos: [{:.1}, {:.1}, {:.1}] km", body.pos.x, body.pos.y, body.pos.z));
                    ui.label(format!("Vel: [{:.3}, {:.3}, {:.3}] km/s", body.vel.x, body.vel.y, body.vel.z));
                    ui.label(format!("Alt: {:.1} km", body.pos.length() - 6371.0));
                    
                    if let Some(tle) = &body.tle {
                        ui.separator();
                        ui.label(egui::RichText::new("TLE DATA").strong().color(Color32::LIGHT_GREEN));
                        ui.label(egui::RichText::new(&tle.raw_line1).monospace().size(10.0));
                        ui.label(egui::RichText::new(&tle.raw_line2).monospace().size(10.0));
                        
                        ui.label(format!("Inc: {:.4}°", tle.inclination.to_degrees()));
                        ui.label(format!("Ecc: {:.6}", tle.eccentricity));
                    } else if body.body_type == BodyType::Russs || body.body_type == BodyType::Iridium33 {
                        // Main satellites also have TLEs in SimState
                        let tle = if body.body_type == BodyType::Russs { &sim.russs_tle } else { &sim.iridium_tle };
                        ui.separator();
                        ui.label(egui::RichText::new("TLE DATA").strong().color(Color32::LIGHT_GREEN));
                        ui.label(egui::RichText::new(&tle.raw_line1).monospace().size(10.0));
                        ui.label(egui::RichText::new(&tle.raw_line2).monospace().size(10.0));
                    }
                    
                    if ui.button("🎯 FOCUS CAMERA").clicked() {
                        camera.focus_on(body.pos);
                    }
                    if ui.button("❌ DESELECT").clicked() {
                        *selected_body = None;
                    }
                });
        }
    }
}

fn legend_item(ui: &mut egui::Ui, name: &str, color: Color32, is_path: bool) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_at_least(Vec2::splat(12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 6.0, color);
        if is_path {
             ui.painter().line_segment(
                 [rect.center() - Vec2::new(0.0, 10.0), rect.center() + Vec2::new(0.0, 10.0)],
                 Stroke::new(1.0, color)
             );
        }
        ui.label(egui::RichText::new(name).font(FontId::proportional(14.0)));
    });
}
