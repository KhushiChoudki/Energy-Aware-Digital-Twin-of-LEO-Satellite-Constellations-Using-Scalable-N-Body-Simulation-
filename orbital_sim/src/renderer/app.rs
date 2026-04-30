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
use crate::simulation::state::SimState;
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
    let russs_tle = load_tle(RUSSS_TLE);
    let iridium_tle = load_tle(IRIDIUM_TLE);
    let debris_points = parse_debris_aer(DEBRIS_AER, &zarya_ephem);

    let mut sim = SimState::new(zarya_ephem, debris_points, russs_tle, iridium_tle);

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
                            ElementState::Released => camera.on_mouse_release(btn),
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

                        let size = window.inner_size();
                        if size.width == 0 || size.height == 0 { return; }

                        let aspect = size.width as f32 / size.height as f32;
                        gpu.update_uniforms(camera.view_proj(), sim.time as f32, sim.flash_intensity, aspect);
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
                            draw_hud(ctx, &mut sim, &camera, &mut selected_body, (size.width, size.height));
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
