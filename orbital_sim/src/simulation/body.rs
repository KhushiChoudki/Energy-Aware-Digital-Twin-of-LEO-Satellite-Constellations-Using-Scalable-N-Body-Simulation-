// src/simulation/body.rs
// Body struct representing all simulated objects

use glam::DVec3;
use std::collections::VecDeque;

pub const SAT_TRAIL_LEN: usize = 1200; 
pub const DEB_TRAIL_LEN: usize = 200; 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType {
    Zarya,
    PreExistingDebris,
    Russs,
    Iridium33,
    CollisionDebris,
    LiveSatellite,
}

impl BodyType {
    pub fn default_color(&self) -> [f32; 4] {
        match self {
            BodyType::Zarya => [0.0, 1.0, 1.0, 0.9],          
            BodyType::PreExistingDebris => [0.4, 0.4, 0.4, 0.6], 
            BodyType::Russs => [0.0, 1.0, 1.0, 0.9],            
            BodyType::Iridium33 => [0.0, 1.0, 1.0, 0.9],         
            BodyType::CollisionDebris => [1.0, 0.3, 0.8, 0.9], 
            BodyType::LiveSatellite => [0.0, 1.0, 1.0, 0.9], 
        }
    }

    pub fn visual_radius(&self) -> f32 {
        match self {
            BodyType::Zarya => 250.0,         
            BodyType::PreExistingDebris => 10.0,
            BodyType::Russs => 200.0,         
            BodyType::Iridium33 => 200.0,     
            BodyType::CollisionDebris => 60.0, 
            BodyType::LiveSatellite => 150.0,
        }
    }

    pub fn visual_kind(&self) -> f32 {
        match self {
            BodyType::Zarya | BodyType::Russs | BodyType::Iridium33 => 0.0, 
            _ => 1.0, 
        }
    }

    pub fn max_trail_len(&self) -> usize {
        match self {
            BodyType::Zarya | BodyType::Russs | BodyType::Iridium33 | BodyType::LiveSatellite => SAT_TRAIL_LEN,
            _ => DEB_TRAIL_LEN,
        }
    }
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

use crate::data::tle_parser::TleElements;

#[derive(Debug, Clone)]
pub struct Body {
    pub id: u64,
    pub name: String,
    pub body_type: BodyType,
    pub pos: DVec3,
    pub vel: DVec3,
    pub mass: f64,
    pub radius: f64,
    pub alive: bool,
    pub age: f64,
    pub trail: VecDeque<[f32; 3]>,
    pub highlight: f32,
    pub spawned_at: f64,
    pub color_override: Option<[f32; 4]>,
    pub tle: Option<TleElements>,
}

impl Body {
    pub fn new(
        name: String,
        body_type: BodyType,
        pos: DVec3,
        vel: DVec3,
        mass: f64,
        radius: f64,
        spawned_at: f64,
    ) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            name,
            body_type,
            pos,
            vel,
            mass,
            radius,
            alive: true,
            age: 0.0,
            trail: VecDeque::with_capacity(body_type.max_trail_len() + 1),
            highlight: 0.0,
            spawned_at,
            color_override: None,
            tle: None,
        }
    }

    pub fn push_trail(&mut self) {
        let p = [self.pos.x as f32, self.pos.y as f32, self.pos.z as f32];
        self.trail.push_back(p);
        if self.trail.len() > self.body_type.max_trail_len() {
            self.trail.pop_front();
        }
    }

    pub fn effective_color(&self) -> [f32; 4] {
        let base = self.color_override.unwrap_or_else(|| self.body_type.default_color());
        let h = self.highlight;
        [
            (base[0] + (1.0 - base[0]) * h).min(1.0),
            (base[1] + (1.0 - base[1]) * h).min(1.0),
            (base[2] + (1.0 - base[2]) * h).min(1.0),
            base[3],
        ]
    }
}
