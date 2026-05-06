// src/renderer/app.rs
// High-fidelity Orbital Simulator: winit 0.29 + wgpu 0.20 + egui 0.28

use std::sync::Arc;
use std::time::Instant;
use anyhow::Result;
use winit::{
    event::{Event, WindowEvent, MouseScrollDelta, ElementState},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    keyboard::{Key, NamedKey},
};
use egui_winit::State as EguiWinitState;
use egui_wgpu::Renderer as EguiRenderer;

use crate::data::{
    ephemeris::parse_zarya_ephemeris,
    tle_parser::load_tle,
    aer_decoder::parse_debris_aer,
};
use crate::simulation::{
    state::SimState,
    body::{Body, BodyType},
};
use crate::renderer::{
    camera::Camera,
    gpu_state::GpuState,
    pipeline::Pipelines,
    ui::draw_hud,
};

// ─── Static Assets ──────────────────────────────────────────────────────────
const ZARYA_EPHEM: &str   = include_str!("../../data/ISS_Zarya Ephemeris-IFT-LLA.csv");
const RUSSS_TLE: &str     = include_str!("../../data/russs.txt");
const IRIDIUM_TLE: &str   = include_str!("../../data/IRIDIUM 33.txt");
const DEBRIS_AER: &str    = include_str!("../../data/Satellite-ISS_Zarya-To-Satellite-Space_Debris AER.csv");
const EARTH_TEXTURE: &[u8] = include_bytes!("../../data/earth_hd.png");

pub fn run() -> Result<()> {
    let zarya_ephem = parse_zarya_ephemeris(ZARYA_EPHEM);
    let debris_points = parse_debris_aer(DEBRIS_AER, &zarya_ephem);

    // 1. Load Consolidated TLEs
    println!("Loading consolidated TLE data...");
    let satellites_content = std::fs::read_to_string("data/satellites.txt")
        .unwrap_or_else(|_| "".to_string());
    let debris_content = std::fs::read_to_string("data/debris.txt")
        .unwrap_or_else(|_| "".to_string());

    let all_satellite_tles = crate::data::tle_parser::parse_many(&satellites_content);
    let debris_tles = crate::data::tle_parser::parse_many(&debris_content);

    // 2. Identify Primary Scenario Satellites
    let russs_tle = all_satellite_tles.iter()
        .find(|t| t.name.contains("22675") || t.name.contains("2251"))
        .cloned()
        .unwrap_or_else(|| load_tle(RUSSS_TLE));
    
    let iridium_tle = all_satellite_tles.iter()
        .find(|t| t.name.contains("24946") || t.name.contains("33"))
        .cloned()
        .unwrap_or_else(|| load_tle(IRIDIUM_TLE));

    let mut sim = SimState::new(zarya_ephem, debris_points, russs_tle, iridium_tle, debris_tles);
    sim.jd_start = 2461162.5; // May 2, 2026

    // 3. Strategic Satellite Population (Targeting ~1000 total)
    println!("Populating strategic satellite constellation...");
    let mut live_sats = Vec::new();
    for tle in all_satellite_tles.iter().take(1500) { 
        if live_sats.len() >= 997 { break; }
        
        // De-duplication: Skip primary scenario satellites
        let name = tle.name.to_uppercase();
        if name.contains("ZARYA") || name.contains("IRIDIUM 33") || name.contains("COSMOS 2251") || name.contains("22675") || name.contains("24903") {
            continue;
        }

        let a = tle.semi_major_axis();
        let alt_avg = a - 6371.0;
        
        if alt_avg > 100.0 && alt_avg < 2000.0 {
            let dt_from_epoch = (sim.jd_start - tle.epoch_jd) * 86400.0;
            let (pos, vel) = tle.propagate(dt_from_epoch);
            
            let mut body = Body::new(
                tle.name.clone(),
                BodyType::LiveSatellite,
                pos,
                vel,
                500.0,
                0.01,
                0.0,
            );
            
            body.color_override = None; // Use unified Cyan
            body.tle = Some(tle.clone());
            live_sats.push(body);
        }
    }
    println!("POC SUCCESS: {} strategic satellites added.", live_sats.len());
    sim.bodies.extend(live_sats);
    sim.export_tle_csv();

    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("🌌 AGI-Style Orbital Digital Twin — Industry Standard")
            .with_inner_size(winit::dpi::PhysicalSize::new(1600u32, 900u32))
            .build(&event_loop)?
    );

    let mut gpu = pollster::block_on(GpuState::new(window.clone(), EARTH_TEXTURE));
    let pipelines = Pipelines::new(&gpu);
    let mut camera = Camera::new(1600.0 / 900.0);

    // Initial Path Update
    gpu.update_static_paths(&sim);

    let egui_ctx = egui::Context::default();
    let mut egui_state = EguiWinitState::new(egui_ctx.clone(), egui_ctx.viewport_id(), &*window, None, None);
    let mut pixels_per_point = window.scale_factor() as f32;
    let mut egui_renderer = EguiRenderer::new(&gpu.device, gpu.format(), None, 1);

    let mut last_frame = Instant::now();
    let mut selected_body: Option<u64> = None;
    let mut search_query = String::new();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => {
                let resp = egui_state.on_window_event(&window, &event);
                if resp.consumed { return; }

                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(sz) => {
                        gpu.resize(sz.width, sz.height);
                        camera.resize(sz.width, sz.height);
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        pixels_per_point = scale_factor as f32;
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        let btn = match button {
                            winit::event::MouseButton::Left => 0,
                            winit::event::MouseButton::Right => 1,
                            winit::event::MouseButton::Middle => 2,
                            _ => 99,
                        };
                        match state {
                            ElementState::Pressed => camera.on_mouse_press(btn),
                            ElementState::Released => {
                                camera.on_mouse_release(btn);
                                if btn == 0 {
                                    // Left click released -> Try selection
                                    if let Some(pos) = camera.last_mouse {
                                        let size = window.inner_size();
                                        let mut best_id = None;
                                        let mut best_score = f32::MAX;

                                        let vp = camera.view_proj();
                                        for body in &sim.bodies {
                                            if !body.alive { continue; }
                                            
                                            // Project body to NDC
                                            let world_pos = glam::Vec3::new(body.pos.x as f32 * 0.01, body.pos.y as f32 * 0.01, body.pos.z as f32 * 0.01);
                                            let ndc = vp.project_point3(world_pos);
                                            
                                            // Depth check: ignore things far behind or far beyond
                                            if ndc.z < -1.0 || ndc.z > 1.0 { continue; }

                                            // NDC to Screen space
                                            let screen_x = (ndc.x + 1.0) * 0.5 * size.width as f32;
                                            let screen_y = (1.0 - ndc.y) * 0.5 * size.height as f32;

                                            let dist_px = ((screen_x - pos.0).powi(2) + (screen_y - pos.1).powi(2)).sqrt();
                                            
                                            // Dynamic hit radius based on body's visual size
                                            // visual_radius is roughly "pixel radius" at distance 100? No, it's world units.
                                            // Let's use a combination of fixed threshold and proximity.
                                            let hit_threshold = (body.body_type.visual_radius() * 0.5).max(30.0); 

                                            if dist_px < hit_threshold {
                                                // Score based on distance and depth (prefer closer to camera and closer to mouse)
                                                let score = dist_px + ndc.z * 10.0;
                                                if score < best_score {
                                                    best_score = score;
                                                    best_id = Some(body.id);
                                                }
                                            }
                                        }
                                        
                                        if let Some(id) = best_id {
                                            selected_body = Some(id);
                                            if let Some(b) = sim.bodies.iter_mut().find(|b| b.id == id) {
                                                println!("Selection Success: {} (ID: {}) dist={:.1}px", b.name, b.id, best_score);
                                                b.highlight = 1.0;
                                            }
                                        } else {
                                            println!("Selection Failed: Click at {:?} did not hit any body", pos);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        camera.on_mouse_move(position.x as f32, position.y as f32);
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let d = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(p) => p.y as f32 / 100.0,
                        };
                        camera.on_scroll(d);
                    }
                    WindowEvent::KeyboardInput { event: key_ev, .. } => {
                        if key_ev.state == ElementState::Pressed {
                            match key_ev.logical_key {
                                Key::Character(ref c) if c == "r" || c == "R" => camera.reset(),
                                Key::Named(NamedKey::Space) => sim.paused = !sim.paused,
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let dt = now.duration_since(last_frame).as_secs_f64();
                        last_frame = now;

                        sim.step(dt);

                        // ─── FOLLOW CAMERA LOGIC ───
                        if let Some(id) = selected_body {
                            if let Some(body) = sim.bodies.iter().find(|b| b.id == id && b.alive) {
                                camera.focus_on(body.pos);
                            } else {
                                // If body is gone (fragmented), clear selection
                                selected_body = None;
                            }
                        }

                        let size = window.inner_size();
                        if size.width == 0 || size.height == 0 { return; }

                        let aspect = size.width as f32 / size.height as f32;
                        gpu.update_uniforms(camera.view_proj(), sim.time as f32, sim.flash_intensity, aspect, camera.distance);
                        gpu.update_bodies(&sim.bodies);
                        gpu.update_trails(&sim.bodies);

                        let frame = match gpu.surface.get_current_texture() {
                            Ok(f) => f,
                            Err(_) => {
                                gpu.surface.configure(&gpu.device, &gpu.surface_config);
                                return;
                            }
                        };
                        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                        {
                            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("main"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), // Darker atmosphere
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                    view: &gpu.depth_view,
                                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                                    stencil_ops: None,
                                }),
                                ..Default::default()
                            });

                            // 1. Earth
                            rp.set_pipeline(&pipelines.earth);
                            rp.set_bind_group(0, &gpu.uniform_bg, &[]);
                            rp.set_bind_group(1, &gpu.earth_tex_bg, &[]);
                            rp.set_vertex_buffer(0, gpu.earth_vbuf.slice(..));
                            rp.set_index_buffer(gpu.earth_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                            rp.draw_indexed(0..gpu.earth_index_count, 0, 0..1);

                            // 2. Static Paths (Entire Orbits)
                            if gpu.static_path_count > 0 {
                                rp.set_pipeline(&pipelines.trails);
                                rp.set_bind_group(0, &gpu.uniform_bg, &[]);
                                rp.set_vertex_buffer(0, gpu.static_path_buf.slice(..));
                                rp.draw(0..gpu.static_path_count, 0..1);
                            }

                            // 3. Dynamic Trails (Post-Collision spread)
                            if gpu.trail_vertex_count > 0 {
                                rp.set_pipeline(&pipelines.trails);
                                rp.set_bind_group(0, &gpu.uniform_bg, &[]);
                                rp.set_vertex_buffer(0, gpu.trail_buf.slice(..));
                                rp.draw(0..gpu.trail_vertex_count, 0..1);
                            }

                            // 4. Bodies (Satellites & Rocks)
                            if gpu.body_instance_count > 0 {
                                rp.set_pipeline(&pipelines.bodies);
                                rp.set_bind_group(0, &gpu.uniform_bg, &[]);
                                rp.set_vertex_buffer(0, gpu.body_instance_buf.slice(..));
                                rp.draw(0..6, 0..gpu.body_instance_count);
                            }
                        }

                        // HUD & Key
                        let raw_input = egui_state.take_egui_input(&window);
                        let full_output = egui_ctx.run(raw_input, |ctx| {
                            let size = window.inner_size();
                            draw_hud(ctx, &mut sim, &mut camera, &mut selected_body, &mut search_query, (size.width, size.height));
                        });
                        egui_state.handle_platform_output(&window, full_output.platform_output.clone());

                        let tris = egui_ctx.tessellate(full_output.shapes, pixels_per_point);
                        let size = window.inner_size();
                        let screen_desc = egui_wgpu::ScreenDescriptor { size_in_pixels: [size.width, size.height], pixels_per_point };
                        
                        for (id, img) in &full_output.textures_delta.set {
                            egui_renderer.update_texture(&gpu.device, &gpu.queue, *id, img);
                        }
                        egui_renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &tris, &screen_desc);

                        {
                            let mut erp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("egui"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                                })],
                                depth_stencil_attachment: None,
                                ..Default::default()
                            });
                            egui_renderer.render(&mut erp, &tris, &screen_desc);
                        }
                        
                        for id in &full_output.textures_delta.free {
                            egui_renderer.free_texture(id);
                        }

                        gpu.queue.submit([encoder.finish()]);
                        frame.present();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => { window.request_redraw(); }
            _ => {}
        }
    })?;

    Ok(())
}
