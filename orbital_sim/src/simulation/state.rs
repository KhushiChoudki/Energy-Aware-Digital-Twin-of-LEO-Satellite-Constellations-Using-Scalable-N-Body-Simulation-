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
};

pub const MAX_DEBRIS: usize = 8000; 
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
    
    pub russs_offset: f64,
    pub iridium_offset: f64,
}

impl SimState {
    pub fn new(
        zarya_ephem: Vec<EphemerisPoint>,
        debris_points: Vec<DebrisPoint>,
        russs_tle: TleElements,
        iridium_tle: TleElements,
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
            russs_offset: 0.0,
            iridium_offset: 0.0,
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
        self.russs_offset = best_t_ru - PRESENTATION_COLLISION_TIME;
        self.iridium_offset = best_t_ir - PRESENTATION_COLLISION_TIME;
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
            let zarya = Body::new("ISS Zarya".to_string(), BodyType::Zarya, pos, vel, 420_000.0, 0.05, 0.0);
            self.zarya_id = Some(zarya.id);
            self.bodies.push(zarya);
        }

        for dp in self.debris_points.iter().step_by(20) {
            let d = Body::new(format!("DEB-{}", dp.time_s), BodyType::PreExistingDebris, dp.pos_eci, dp.vel_eci, 100.0, 0.01, 0.0);
            self.bodies.push(d);
        }

        let (ru_pos, ru_vel) = self.russs_tle.propagate(self.russs_offset);
        let russs = Body::new("Cosmos-2251".to_string(), BodyType::Russs, ru_pos, ru_vel, 900_000.0, 0.04, 0.0);
        self.russs_id = Some(russs.id);
        self.bodies.push(russs);

        let (ir_pos, ir_vel) = self.iridium_tle.propagate(self.iridium_offset);
        let iridium = Body::new("Iridium-33".to_string(), BodyType::Iridium33, ir_pos, ir_vel, 689_000.0, 0.04, 0.0);
        self.iridium_id = Some(iridium.id);
        self.bodies.push(iridium);
    }

    pub fn step(&mut self, wall_dt: f64) {
        if self.paused { return; }
        let sim_dt = (wall_dt * self.time_scale).min(100.0);
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

        self.bodies.retain(|b| b.alive || (b.body_type == BodyType::Zarya && self.phase == SimPhase::PreCollision));
    }

    fn integrate_step(&mut self, dt: f64) {
        for body in self.bodies.iter_mut() {
            if !body.alive { continue; }
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
        let ru_deb = DebrisGen::generate_cloud(self.russs_id.unwrap_or(0), pos, v_ru, 893, self.time, [1.0, 0.2, 0.8, 1.0]);
        let ir_deb = DebrisGen::generate_cloud(self.iridium_id.unwrap_or(0), pos, v_ir, 366, self.time, [1.0, 0.2, 0.8, 1.0]);
        
        self.bodies.extend(ru_deb);
        self.bodies.extend(ir_deb);

        self.phase = SimPhase::CollisionFlash(self.time);
        println!("PRIMARY COLLISION: Exact {} pieces generated.", 893 + 366);
    }
}
