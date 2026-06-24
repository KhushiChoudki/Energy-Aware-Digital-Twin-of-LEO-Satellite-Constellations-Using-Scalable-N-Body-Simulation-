// src/simulation/state.rs
// Main simulation state machine with exact piece counts and Zarya destruction support

use glam::DVec3;
use crate::data::{
    ephemeris::{EphemerisPoint, interpolate_ephemeris},
    tle_parser::TleElements,
    aer_decoder::DebrisPoint,
};
use crate::simulation::{
    body::{Body, BodyType},
    collision::{check_collisions, CollisionEvent},
    integrator::rk4_step,
    gnn_predictor::GnnPredictor,
};

pub const MAX_DEBRIS: usize = 50000; 
pub const PRESENTATION_COLLISION_TIME: f64 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimPhase {
    PreCollision,
    CollisionFlash(f64),
    PostCollision,
}

pub struct OrbitPath {
    pub body_type: BodyType,
    pub points: Vec<DVec3>,
}

pub struct SimState {
    pub time: f64,
    pub time_scale: f64,
    pub paused: bool,

    pub bodies: Vec<Body>,
    pub zarya_ephem: Vec<EphemerisPoint>,
    pub debris_points: Vec<DebrisPoint>,
    pub orbital_paths: Vec<OrbitPath>,

    pub zarya_id: Option<u64>,
    pub russs_id: Option<u64>,
    pub iridium_id: Option<u64>,

    pub phase: SimPhase,
    pub collision_events: Vec<CollisionEvent>,
    pub flash_intensity: f32,

    pub russs_tle: TleElements,
    pub iridium_tle: TleElements,
    
    pub debris_tles: Vec<TleElements>,

    pub russs_offset: f64,
    pub iridium_offset: f64,
    pub jd_start: f64,
    pub predictor: GnnPredictor,
    pub sim_speed_multiplier: f64,
    pub last_gnn_update: f64,

    pub network_mode_active: bool,
    pub ground_stations: Vec<DVec3>,
    pub show_debris: bool,
    pub rl_auto_execute: bool,
}

impl SimState {
    pub fn new(
        zarya_ephem: Vec<EphemerisPoint>,
        debris_points: Vec<DebrisPoint>,
        russs_tle: TleElements,
        iridium_tle: TleElements,
        debris_tles: Vec<TleElements>,
    ) -> Self {
        let mut state = SimState {
            time: 0.0,
            time_scale: 100.0,
            paused: false,
            bodies: Vec::new(),
            zarya_ephem,
            debris_points,
            orbital_paths: Vec::new(),
            zarya_id: None,
            russs_id: None,
            iridium_id: None,
            phase: SimPhase::PreCollision,
            collision_events: Vec::new(),
            flash_intensity: 0.0,
            russs_tle,
            iridium_tle,
            debris_tles,
            russs_offset: 0.0,
            iridium_offset: 0.0,
            jd_start: 0.0, 
            predictor: GnnPredictor::new(),
            sim_speed_multiplier: 1.0,
            last_gnn_update: 0.0,
            network_mode_active: false,
            ground_stations: vec![
                // Example Indian ground stations (approximate coordinates converted to Earth-centered ECI-like vectors)
                // Just placeholders using random vectors on the sphere surface * 6371.0
                // For simplicity, we will assume a stationary Earth for the LOS check, or just place them along the equator/India lat.
                DVec3::new(1280.0, 5630.0, 2670.0), // ~Bengaluru / ISTRAC approx
                DVec3::new(1030.0, 5010.0, 3600.0), // ~New Delhi approx
            ],
            show_debris: true,
            rl_auto_execute: false,
        };
        state.force_geometric_intersection();
        state.precompute_paths();
        state.init_bodies();
        state
    }

    fn force_geometric_intersection(&mut self) {
        let mut min_dist = f64::MAX;
        let mut best_t_ru = 0.0;
        let mut best_t_ir = 0.0;

        // Stage 1: Coarse Search (30s steps)
        for i in 0..200 {
            let t_ru = i as f64 * 30.0;
            let (p_ru, _) = self.russs_tle.propagate(t_ru);
            for j in 0..200 {
                let t_ir = j as f64 * 30.0;
                let (p_ir, _) = self.iridium_tle.propagate(t_ir);
                let d = (p_ru - p_ir).length();
                if d < min_dist {
                    min_dist = d;
                    best_t_ru = t_ru;
                    best_t_ir = t_ir;
                }
            }
        }

        // Stage 2: Fine Search (0.1s steps around coarse match)
        let coarse_ru = best_t_ru;
        let coarse_ir = best_t_ir;
        for i in -300..300 {
            let t_ru = coarse_ru + (i as f64 * 0.1);
            let (p_ru, _) = self.russs_tle.propagate(t_ru);
            for j in -300..300 {
                let t_ir = coarse_ir + (j as f64 * 0.1);
                let (p_ir, _) = self.iridium_tle.propagate(t_ir);
                let d = (p_ru - p_ir).length();
                if d < min_dist {
                    min_dist = d;
                    best_t_ru = t_ru;
                    best_t_ir = t_ir;
                }
            }
        }

        // Stage 3: Millisecond Precision (0.001s steps)
        let fine_ru = best_t_ru;
        let fine_ir = best_t_ir;
        for i in -100..100 {
            let t_ru = fine_ru + (i as f64 * 0.001);
            let (p_ru, _) = self.russs_tle.propagate(t_ru);
            for j in -100..100 {
                let t_ir = fine_ir + (j as f64 * 0.001);
                let (p_ir, _) = self.iridium_tle.propagate(t_ir);
                let d = (p_ru - p_ir).length();
                if d < min_dist {
                    min_dist = d;
                    best_t_ru = t_ru;
                    best_t_ir = t_ir;
                }
            }
        }

        self.russs_offset = best_t_ru - PRESENTATION_COLLISION_TIME;
        self.iridium_offset = best_t_ir - PRESENTATION_COLLISION_TIME;
        println!("ORBITAL INTERCEPT REFINED: Min Distance = {:.6} km ({} meters)", min_dist, (min_dist * 1000.0) as i32);
    }

    fn precompute_paths(&mut self) {
        let mut zarya_points = Vec::new();
        for p in self.zarya_ephem.iter().take(500).step_by(10) {
            zarya_points.push(p.pos_eci);
        }
        self.orbital_paths.push(OrbitPath { body_type: BodyType::Zarya, points: zarya_points });

        let mut russs_points = Vec::new();
        for i in 0..500 {
            let t = self.russs_offset + i as f64 * 30.0;
            let (pos, _) = self.russs_tle.propagate(t);
            russs_points.push(pos);
        }
        self.orbital_paths.push(OrbitPath { body_type: BodyType::Russs, points: russs_points });

        let mut iridium_points = Vec::new();
        for i in 0..500 {
            let t = self.iridium_offset + i as f64 * 30.0;
            let (pos, _) = self.iridium_tle.propagate(t);
            iridium_points.push(pos);
        }
        self.orbital_paths.push(OrbitPath { body_type: BodyType::Iridium33, points: iridium_points });
    }

    fn init_bodies(&mut self) {
        if let Some((pos, vel)) = interpolate_ephemeris(&self.zarya_ephem, 0.0) {
            let zarya = Body::new("ISS Zarya".to_string(), BodyType::Zarya, pos, vel, 420_000.0, 0.1, 0.0); // 100m radius
            self.zarya_id = Some(zarya.id);
            self.bodies.push(zarya);
        }

        for dp in self.debris_points.iter().step_by(20) {
            let d = Body::new(format!("DEB-{}", dp.time_s), BodyType::PreExistingDebris, dp.pos_eci, dp.vel_eci, 100.0, 0.005, 0.0); // 5m radius
            self.bodies.push(d);
        }

        let (ru_pos, ru_vel) = self.russs_tle.propagate(self.russs_offset);
        let russs = Body::new("Cosmos-2251".to_string(), BodyType::Russs, ru_pos, ru_vel, 900_000.0, 0.015, 0.0); // 15m radius
        self.russs_id = Some(russs.id);
        self.bodies.push(russs);

        let (ir_pos, ir_vel) = self.iridium_tle.propagate(self.iridium_offset);
        let iridium = Body::new("Iridium-33".to_string(), BodyType::Iridium33, ir_pos, ir_vel, 689_000.0, 0.015, 0.0); // 15m radius
        self.iridium_id = Some(iridium.id);
        self.bodies.push(iridium);
    }

    pub fn step(&mut self, wall_dt: f64) {
        if self.paused { return; }
        let sim_dt = (wall_dt * self.time_scale * self.sim_speed_multiplier).min(100.0);
        let sub_steps = 4;
        let dt = sim_dt / sub_steps as f64;

        for _ in 0..sub_steps {
            self.integrate_step(dt);
            self.time += dt;
        }

        for body in self.bodies.iter_mut() {
            if body.alive {
                body.push_trail();
                body.age += sim_dt;
                if body.highlight > 0.0 {
                    body.highlight = (body.highlight - 0.2).max(0.0);
                }

                // Energy and Network Simulation
                if matches!(body.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya) {
                    body.has_los = false;
                    for gs in &self.ground_stations {
                        let rel = body.pos - *gs;
                        if rel.dot(*gs) > 0.0 { // Elevation > 0 approx
                            body.has_los = true;
                            break;
                        }
                    }

                    body.is_transmitting = body.has_los;

                    if body.is_transmitting {
                        // Drain battery
                        body.current_battery -= 0.05 * sim_dt; 
                    } else {
                        // Recharge battery (assume solar panels active)
                        body.current_battery += 0.02 * sim_dt;
                    }
                    body.current_battery = body.current_battery.clamp(0.0, body.battery_capacity);
                    
                    if body.thrust_flash > 0.0 {
                        body.thrust_flash = (body.thrust_flash - sim_dt).max(0.0);
                    }
                    
                    // Optional visual override
                    if self.network_mode_active {
                        if body.current_battery < 20.0 {
                            body.color_override = Some([1.0, 0.0, 0.0, 1.0]); // Critical battery
                        } else if body.is_transmitting {
                            body.color_override = Some([0.0, 1.0, 0.0, 1.0]); // Transmitting
                        } else {
                            body.color_override = Some([0.5, 0.5, 0.5, 1.0]); // Idle / Recharging
                        }
                    } else {
                        body.color_override = None;
                    }
                }
            }
        }

        if self.phase == SimPhase::PreCollision && self.time >= PRESENTATION_COLLISION_TIME {
            self.trigger_primary_collision();
        }

        if let SimPhase::CollisionFlash(start) = self.phase {
            let elapsed = self.time - start;
            self.flash_intensity = (1.0 - elapsed / 2.0).max(0.0) as f32;
            if elapsed > 2.0 {
                self.phase = SimPhase::PostCollision;
                self.flash_intensity = 0.0;
            }
        }

        let (events, new_bodies) = check_collisions(
            &mut self.bodies,
            self.time,
            self.russs_id,
            self.iridium_id,
            self.phase != SimPhase::PreCollision,
        );

        for mut nb in new_bodies {
            if self.bodies.len() < MAX_DEBRIS {
                nb.trail.push_back([nb.pos.x as f32, nb.pos.y as f32, nb.pos.z as f32]);
                self.bodies.push(nb);
            }
        }
        self.collision_events.extend(events);

        self.bodies.retain(|b| b.alive);
        // ─── GNN AI PREDICTION STEP ───
        if self.time - self.last_gnn_update > 200.0 {
            self.predictor.update(&self.bodies);
            self.last_gnn_update = self.time;
        }
    }

    fn integrate_step(&mut self, dt: f64) {
        for body in self.bodies.iter_mut() {
            if !body.alive { continue; }
            
            // Priority 1: TLE Data (if explicitly attached)
            if let Some(tle) = &body.tle {
                let (pos, vel) = if body.body_type == BodyType::LiveSatellite {
                    // Sync to current JD
                    let dt_from_epoch = (self.jd_start - tle.epoch_jd) * 86400.0 + self.time;
                    tle.propagate(dt_from_epoch)
                } else {
                    // Scenario-based offset for debris cloud
                    let offset = if body.name.contains("IRIDIUM") { self.iridium_offset } else { self.russs_offset };
                    tle.propagate(self.time + offset)
                };
                body.pos = pos;
                body.vel = vel;
                continue;
            }

            match body.body_type {
                BodyType::Zarya => {
                    if let Some((pos, vel)) = interpolate_ephemeris(&self.zarya_ephem, self.time) {
                        body.pos = pos; body.vel = vel;
                    }
                }
                BodyType::Russs => {
                    let (pos, vel) = self.russs_tle.propagate(self.time + self.russs_offset);
                    body.pos = pos; body.vel = vel;
                }
                BodyType::Iridium33 => {
                    let (pos, vel) = self.iridium_tle.propagate(self.time + self.iridium_offset);
                    body.pos = pos; body.vel = vel;
                }
                _ => {
                    rk4_step(&mut body.pos, &mut body.vel, dt);
                    if body.pos.length() < 6371.0 + 80.0 {
                        body.alive = false;
                    }
                }
            }
        }
    }

    fn trigger_primary_collision(&mut self) {
        let (p_ru, v_ru) = self.russs_tle.propagate(self.time + self.russs_offset);
        let (p_ir, v_ir) = self.iridium_tle.propagate(self.time + self.iridium_offset);
        let pos = (p_ru + p_ir) * 0.5;

        for body in self.bodies.iter_mut() {
            if body.body_type == BodyType::Russs || body.body_type == BodyType::Iridium33 {
                body.alive = false;
            }
        }

        use crate::simulation::debris_gen::DebrisGen;
        // Exact Piece Counts: 893 for Russs, 366 for Iridium
        let mut ru_deb = DebrisGen::generate_cloud(self.russs_id.unwrap_or(0), pos, v_ru, 900.0, 893, self.time, [1.0, 0.2, 0.8, 1.0]);
        let mut ir_deb = DebrisGen::generate_cloud(self.iridium_id.unwrap_or(0), pos, v_ir, 560.0, 366, self.time, [1.0, 0.2, 0.8, 1.0]);
        
        // Assign TLEs to Cosmos (Russs) debris
        let cosmos_tles: Vec<_> = self.debris_tles.iter()
            .filter(|t| t.name.contains("COSMOS") || t.name.contains("2251"))
            .cloned()
            .collect();
        
        for (i, tle) in cosmos_tles.into_iter().enumerate() {
            if i < ru_deb.len() {
                ru_deb[i].name = tle.name.clone();
                ru_deb[i].tle = Some(tle);
            }
        }

        // Assign TLEs to Iridium debris
        let iridium_tles: Vec<_> = self.debris_tles.iter()
            .filter(|t| t.name.contains("IRIDIUM"))
            .cloned()
            .collect();

        for (i, tle) in iridium_tles.into_iter().enumerate() {
            if i < ir_deb.len() {
                ir_deb[i].name = tle.name.clone();
                ir_deb[i].tle = Some(tle);
            }
        }

        self.bodies.extend(ru_deb);
        self.bodies.extend(ir_deb);

        self.phase = SimPhase::CollisionFlash(self.time);
        println!("PRIMARY COLLISION: Exact {} pieces generated.", 893 + 366);
    }
    pub fn export_tle_csv(&self) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::File::create("tle_dataset.csv") {
            let _ = writeln!(file, "Name,Inclination_Deg,Eccentricity,SemiMajorAxis_km,Type");
            for body in &self.bodies {
                if let Some(tle) = &body.tle {
                    let type_str = match body.body_type {
                        BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya => "SATELLITE",
                        _ => "DEBRIS",
                    };
                    let _ = writeln!(file, "{},{:.4},{:.6},{:.2},{}", 
                        body.name, 
                        tle.inclination.to_degrees(), 
                        tle.eccentricity, 
                        tle.semi_major_axis(),
                        type_str
                    );
                }
            }
            println!("SUCCESS: Exported TLE dataset to tle_dataset.csv");
        }
    }
}
