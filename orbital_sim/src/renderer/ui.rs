// src/renderer/ui.rs
// Professional Mission Legend and HUD (AGI/STK Style)

use egui::{Context, Window, Color32, Align2, FontId, Stroke, Vec2};
use crate::simulation::state::{SimState, SimPhase};
use crate::simulation::body::BodyType;
use crate::renderer::camera::Camera;

pub fn draw_hud(
    ctx: &Context, 
    sim: &mut SimState, 
    _camera: &Camera,
    _selected_body: &mut Option<u64>,
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
                ui.add(egui::Slider::new(&mut sim.time_scale, 0.0..=200.0).suffix("x"));
            });
            
            if ui.button("⏸ PAUSE / RESUME").clicked() {
                sim.paused = !sim.paused;
            }
        });

    // ─── MISSION KEY / LEGEND (Bottom Left) ─────────────────────────────────
    Window::new("🗺️ MISSION KEY")
        .anchor(Align2::LEFT_BOTTOM, [20.0, -20.0])
        .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(180)))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                legend_item(ui, "ISS ZARYA", Color32::from_rgb(0, 100, 255), true);
                legend_item(ui, "COSMOS-2251", Color32::RED, true);
                legend_item(ui, "IRIDIUM-33", Color32::YELLOW, true);
                legend_item(ui, "PRE-EXISTING DEBRIS", Color32::GRAY, false);
                legend_item(ui, "COLLISION FRAGMENTS", Color32::from_rgb(200, 150, 50), false);
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
