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

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui.button(if sim.show_debris { "🚫 HIDE DEBRIS" } else { "✅ SHOW DEBRIS" }).clicked() {
                    sim.show_debris = !sim.show_debris;
                }
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(10.0);
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
                ui.label("SIM SPEED MULTIPLIER:");
                ui.add(egui::Slider::new(&mut sim.sim_speed_multiplier, 0.0..=5.0).suffix("x"));
            });
            ui.horizontal(|ui| {
                ui.label("TIME STEP:");
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
                    let mut debris_groups: std::collections::HashMap<String, (usize, u64, glam::DVec3)> = std::collections::HashMap::new();

                    for body in &sim.bodies {
                        if !body.alive { continue; }
                        if !body.name.to_lowercase().contains(&query) { continue; }

                        let is_sat = matches!(body.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                        
                        if is_sat {
                            let color = body.effective_color();
                            let egui_color = Color32::from_rgba_unmultiplied(
                                (color[0] * 255.0) as u8,
                                (color[1] * 255.0) as u8,
                                (color[2] * 255.0) as u8,
                                255,
                            );

                            if ui.button(egui::RichText::new(format!("🛰️ SAT | {}", body.name)).color(egui_color)).clicked() {
                                *selected_body = Some(body.id);
                                camera.focus_on(body.pos);
                            }
                            found += 1;
                        } else {
                            let entry = debris_groups.entry(body.name.clone()).or_insert((0, body.id, body.pos));
                            entry.0 += 1;
                        }
                        
                        if found >= 20 { break; }
                    }

                    // Display Grouped Debris
                    for (name, (count, id, pos)) in debris_groups {
                        if ui.button(egui::RichText::new(format!("🧱 DEB | {} (x{})", name, count)).color(Color32::GOLD)).clicked() {
                            *selected_body = Some(id);
                            camera.focus_on(pos);
                        }
                        found += 1;
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
                legend_item(ui, "ACTIVE ASSETS (CYAN)", Color32::from_rgb(0, 255, 255), true);
                legend_item(ui, "ORBITAL DEBRIS", Color32::from_rgb(255, 200, 0), false);
                legend_item(ui, "PRIMARY DEBRIS (PINK)", Color32::from_rgb(255, 80, 200), false);
                legend_item(ui, "PRE-EXISTING DEBRIS", Color32::GRAY, false);
                ui.separator();
                ui.label(egui::RichText::new("• Dots: Moving Assets").size(10.0));
                ui.label(egui::RichText::new("• Lines: Orbital Paths").size(10.0));
            });
        });

    // ─── EVENT ALERT (Top Right) ──────────────────────────────────────────
    if let Some(ev) = sim.collision_events.last() {
        Window::new("⚠ EVENT ALERT")
            .anchor(Align2::RIGHT_TOP, [-20.0, 20.0])
            .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_rgba_unmultiplied(100, 0, 0, 180)))
            .show(ctx, |ui| {
                ui.heading("PRIMARY COLLISION DETECTED");
                ui.label(format!("Primary: {} + {}", ev.body_a_name, ev.body_b_name));
                ui.label(format!("Tracked Fragments: {}", ev.new_debris_count));
            });
    }

    // ─── AI COLLISION PREDICTION (Top Right) ────────────────────────────────
    if !sim.predictor.risk_map.is_empty() {
        Window::new("🤖 AI COLLISION PREDICTOR")
            .anchor(Align2::LEFT_TOP, [20.0, 300.0])
            .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(200)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("GNN PREDICTIVE RISK").strong().color(Color32::from_rgb(255, 100, 100)));
                ui.separator();
                
                let mut risks: Vec<_> = sim.predictor.risk_map.iter().collect();
                risks.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
                
                for (&(id1, id2), &risk) in risks.iter().take(5) {
                    let b1 = sim.bodies.iter().find(|b| b.id == id1);
                    let b2 = sim.bodies.iter().find(|b| b.id == id2);
                    
                    if let (Some(s1), Some(s2)) = (b1, b2) {
                        ui.horizontal(|ui| {
                            let risk_pct = (risk * 100.0) as i32;
                            ui.label(egui::RichText::new(format!("{}%", risk_pct)).strong().color(Color32::RED));
                            if ui.button(format!("{} ↔ {}", s1.name, s2.name)).clicked() {
                                *selected_body = Some(s1.id);
                                camera.focus_on(s1.pos);
                            }
                        });
                    }
                }
            });

        // ─── RL MANEUVERING AGENT (Right) ────────────────────────────────
        Window::new("🚀 RL MANEUVERING AGENT")
            .anchor(Align2::RIGHT_TOP, [-20.0, 300.0])
            .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(200)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("RECOMMENDED AVOIDANCE BURNS").strong().color(Color32::from_rgb(100, 255, 100)));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Autonomy:");
                    if ui.add(egui::widgets::SelectableLabel::new(sim.rl_auto_execute, "ENABLE AUTO-EVASION")).clicked() {
                        sim.rl_auto_execute = !sim.rl_auto_execute;
                    }
                });
                ui.separator();
                
                let mut risks: Vec<_> = sim.predictor.risk_map.iter().collect();
                risks.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
                
                let mut execute_burn_id = None;
                let mut execute_burn_dv = glam::DVec3::ZERO;
                let mut displayed = 0;

                for (&(id1, id2), &risk) in risks.iter() {
                    let b1 = sim.bodies.iter().find(|b| b.id == id1);
                    let b2 = sim.bodies.iter().find(|b| b.id == id2);
                    
                    if let (Some(s1), Some(s2)) = (b1, b2) {
                        let is_s1_sat = matches!(s1.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                        let is_s2_sat = matches!(s2.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                        
                        let (target_sat, threat_body) = if is_s1_sat {
                            (s1, s2)
                        } else if is_s2_sat {
                            (s2, s1)
                        } else {
                            continue; // Skip debris-on-debris collisions
                        };

                        let relative_vel = (target_sat.vel - threat_body.vel).length();
                        let dv_magnitude = (risk as f64 * relative_vel * 10.0).max(0.5); // Mock output in m/s
                        let direction = if target_sat.id % 2 == 0 { "Prograde" } else { "Normal" };
                        
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Target: {}", target_sat.name)).strong());
                                if ui.button("🎯 Track").clicked() {
                                    *selected_body = Some(target_sat.id);
                                    camera.focus_on(target_sat.pos);
                                }
                            });
                            ui.label(format!("Threat: {} (Risk: {:.0}%)", threat_body.name, risk * 100.0));
                            ui.horizontal(|ui| {
                                ui.label("Action:");
                                ui.colored_label(Color32::GREEN, format!("Δv {:.2} m/s {}", dv_magnitude, direction));
                            });
                            if ui.button("EXECUTE BURN").clicked() || (sim.rl_auto_execute && risk > 0.8 && execute_burn_id.is_none()) {
                                println!("RL AGENT EXECUTED BURN: {} m/s on {}", dv_magnitude, target_sat.name);
                                
                                // Auto-track and zoom on the satellite that is executing the burn
                                *selected_body = Some(target_sat.id);
                                camera.focus_on(target_sat.pos);

                                let v_hat = target_sat.vel.normalize();
                                let mut delta_v = v_hat * (dv_magnitude / 1000.0);
                                if direction == "Normal" {
                                    let h_hat = target_sat.pos.cross(target_sat.vel).normalize();
                                    delta_v = h_hat * (dv_magnitude / 1000.0);
                                }
                                execute_burn_id = Some(target_sat.id);
                                execute_burn_dv = delta_v;
                            }
                        });
                        
                        displayed += 1;
                        if displayed >= 3 { break; }
                    }
                }

                if let Some(id) = execute_burn_id {
                    if let Some(b) = sim.bodies.iter_mut().find(|b| b.id == id) {
                        b.vel += execute_burn_dv;
                        b.tle = None; // Detach from fixed TLE orbit so physics applies!
                        b.thrust_flash = 2.0; // Visual thrust feedback for 2 seconds
                    }
                }
            });
    }

    // ─── NETWORK & ENERGY MODE (Top Right) ────────────────────────────────
    Window::new("🌐 NETWORK OVERVIEW")
        .anchor(Align2::RIGHT_TOP, [-20.0, 150.0]) // Below Event Alert
        .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_black_alpha(200)))
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("COMMUNICATION & POWER").strong().color(Color32::LIGHT_GREEN));
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Display Mode:");
                if ui.add(egui::widgets::SelectableLabel::new(sim.network_mode_active, "ENABLE NETWORK/ENERGY VIEW")).clicked() {
                    sim.network_mode_active = !sim.network_mode_active;
                }
            });
            if sim.network_mode_active {
                ui.colored_label(Color32::GREEN, "🟢 Transmitting (LOS Active)");
                ui.colored_label(Color32::GRAY, "⚪ Recharging (No LOS)");
                ui.colored_label(Color32::RED, "🔴 Low Battery (<20%)");
            }
        });

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
                    
                    let type_str = match body.body_type {
                        BodyType::LiveSatellite => "🛰️ LIVE ASSET",
                        BodyType::Zarya => "🏰 STATION MODULE",
                        BodyType::Russs | BodyType::Iridium33 => "🚀 PRIMARY SATELLITE",
                        BodyType::CollisionDebris => "🧱 FRAGMENTED DEBRIS",
                        BodyType::PreExistingDebris => "⚙️ LEGACY DEBRIS",
                    };
                    ui.label(egui::RichText::new(type_str).size(12.0).italics().color(Color32::LIGHT_GRAY));

                    ui.separator();
                    ui.label(format!("Pos: [{:.1}, {:.1}, {:.1}] km", body.pos.x, body.pos.y, body.pos.z));
                    ui.label(format!("Vel: [{:.3}, {:.3}, {:.3}] km/s", body.vel.x, body.vel.y, body.vel.z));
                    ui.label(format!("Alt: {:.1} km", body.pos.length() - 6371.0));
                    
                    if matches!(body.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya) {
                        ui.separator();
                        ui.label(egui::RichText::new("ENERGY & NETWORK").strong().color(Color32::YELLOW));
                        let pct = (body.current_battery / body.battery_capacity) * 100.0;
                        ui.label(format!("Battery Level: {:.1}%", pct));
                        if body.has_los {
                            ui.colored_label(Color32::GREEN, "Network Link: ACTIVE (Transmitting)");
                        } else {
                            ui.colored_label(Color32::GRAY, "Network Link: OFFLINE (No Line-of-Sight)");
                        }
                    }
                    
                    let origin_str = match body.body_type {
                        BodyType::LiveSatellite => "India (ISRO / Indian Registry)".to_string(),
                        BodyType::Zarya => "International (ISS Zarya)".to_string(),
                        BodyType::Russs => "Russia (Cosmos)".to_string(),
                        BodyType::Iridium33 => "United States (Iridium)".to_string(),
                        BodyType::CollisionDebris | BodyType::PreExistingDebris => {
                            let upper_name = body.name.to_uppercase();
                            if upper_name.contains("COSMOS") || upper_name.contains("2251") {
                                "Fragment of Cosmos 2251 (Russia)".to_string()
                            } else if upper_name.contains("IRIDIUM") || upper_name.contains("33") {
                                "Fragment of Iridium 33 (United States)".to_string()
                            } else if upper_name.contains("ZARYA") {
                                "Fragment of ISS Zarya".to_string()
                            } else {
                                "Unknown Orbital Debris".to_string()
                            }
                        }
                    };
                    ui.label(egui::RichText::new(format!("Origin: {}", origin_str)).strong().color(Color32::LIGHT_BLUE));
                    
                    if let Some(tle) = &body.tle {
                        ui.separator();
                        ui.label(egui::RichText::new("TLE DATA").strong().color(Color32::LIGHT_GREEN));
                        ui.label(egui::RichText::new(&tle.raw_line1).monospace().size(10.0));
                        ui.label(egui::RichText::new(&tle.raw_line2).monospace().size(10.0));
                        
                        ui.label(format!("Inc: {:.4}°", tle.inclination.to_degrees()));
                        ui.label(format!("Ecc: {:.6}", tle.eccentricity));
                    } else if body.body_type == BodyType::Russs || body.body_type == BodyType::Iridium33 {
                        let tle = if body.body_type == BodyType::Russs { &sim.russs_tle } else { &sim.iridium_tle };
                        ui.separator();
                        ui.label(egui::RichText::new("TLE DATA").strong().color(Color32::LIGHT_GREEN));
                        ui.label(egui::RichText::new(&tle.raw_line1).monospace().size(10.0));
                        ui.label(egui::RichText::new(&tle.raw_line2).monospace().size(10.0));
                    }
                    
                    if ui.button("❌ DESELECT").clicked() {
                        *selected_body = None;
                    }
                });

            // ─── RED TRACKER ARROW ───
            draw_red_tracker(ctx, body.pos, camera, _screen_size);
        }
    }
}

fn draw_red_tracker(ctx: &Context, body_pos: glam::DVec3, camera: &Camera, screen_size: (u32, u32)) {
    let vp = camera.view_proj();
    let world_pos = glam::Vec3::new(body_pos.x as f32 * 0.01, body_pos.y as f32 * 0.01, body_pos.z as f32 * 0.01);
    let ndc = vp.project_point3(world_pos);
    
    if ndc.z < -1.0 || ndc.z > 1.0 { return; } 

    // Convert NDC to logical pixels (taking High-DPI scale into account)
    let ppp = ctx.pixels_per_point();
    let screen_x = (ndc.x + 1.0) * 0.5 * (screen_size.0 as f32 / ppp);
    let screen_y = (1.0 - ndc.y) * 0.5 * (screen_size.1 as f32 / ppp);
    
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("tracker")));
    let target_center = egui::pos2(screen_x, screen_y);
    
    // THE ARROW TIP: Now pointing EXACTLY at the center of the body
    let arrow_tip = target_center; 
    let arrow_top = arrow_tip - egui::vec2(0.0, 60.0);
    
    // Draw Animated Arrow
    let time = ctx.input(|i| i.time);
    let bounce = (time * 5.0).sin() as f32 * 5.0;
    let anim_tip = arrow_tip - egui::vec2(0.0, 5.0 + bounce.abs());
    let anim_top = arrow_top - egui::vec2(0.0, bounce.abs());

    painter.line_segment([anim_top, anim_tip], egui::Stroke::new(3.0, Color32::RED));
    
    // Arrow Head
    painter.line_segment([anim_tip, anim_tip + egui::vec2(-10.0, -10.0)], egui::Stroke::new(3.0, Color32::RED));
    painter.line_segment([anim_tip, anim_tip + egui::vec2(10.0, -10.0)], egui::Stroke::new(3.0, Color32::RED));
    
    // Targeting Ring (Centered exactly on body)
    painter.circle_stroke(target_center, 30.0, egui::Stroke::new(2.0, Color32::RED));
    painter.circle_stroke(target_center, 2.0, egui::Stroke::new(2.0, Color32::RED)); // Center dot
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
