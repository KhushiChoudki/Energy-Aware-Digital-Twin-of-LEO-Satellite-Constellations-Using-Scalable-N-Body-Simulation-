// src/simulation/collision.rs
// Grid-based collision detection with Zarya destruction logic

use glam::DVec3;
use crate::simulation::body::{Body, BodyType};
use crate::simulation::debris_gen::DebrisGen;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CollisionEvent {
    pub time: f64,
    pub body_a_name: String,
    pub body_b_name: String,
    pub pos: DVec3,
    pub new_debris_count: usize,
    pub is_primary: bool,
}

const COLLISION_DISTANCE: f64 = 50.0; // km for guaranteed cascade demonstration
const INTERACTION_DISTANCE: f64 = 0.5; // km for debris interaction
const GRID_CELL_SIZE: f64 = 160.0; // km to accommodate Zarya's 150km radius

pub fn check_collisions(
    bodies: &mut Vec<Body>,
    sim_time: f64,
    _russs_id: Option<u64>,
    _iridium_id: Option<u64>,
    main_collision_done: bool,
) -> (Vec<CollisionEvent>, Vec<Body>) {
    let mut events = Vec::new();
    let mut new_debris: Vec<Body> = Vec::new();
    let mut to_kill = std::collections::HashSet::new();

    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (idx, body) in bodies.iter().enumerate() {
        if !body.alive { continue; }
        let gx = (body.pos.x / GRID_CELL_SIZE).floor() as i32;
        let gy = (body.pos.y / GRID_CELL_SIZE).floor() as i32;
        let gz = (body.pos.z / GRID_CELL_SIZE).floor() as i32;
        grid.entry((gx, gy, gz)).or_default().push(idx);
    }

    let mut pairs_checked = std::collections::HashSet::new();
    let grid_keys: Vec<_> = grid.keys().cloned().collect();

    for &(gx, gy, gz) in &grid_keys {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(others) = grid.get(&(gx + dx, gy + dy, gz + dz)) {
                        let current = grid.get(&(gx, gy, gz)).unwrap();
                        for &i in current {
                            for &j in others {
                                if i >= j { continue; }
                                if !pairs_checked.insert((i, j)) { continue; }
                                
                                let b1 = &bodies[i];
                                let b2 = &bodies[j];
                                if !b1.alive || !b2.alive { continue; }

                                let dist = (b1.pos - b2.pos).length();

                                // 1. Satellite Fragmentation (Kessler Cascade)
                                let b1 = &bodies[i];
                                let b2 = &bodies[j];
                                
                                let is_sat_1 = matches!(b1.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                                let is_sat_2 = matches!(b2.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                                
                                // Special case for Zarya (Secondary Cascade)
                                if (b1.body_type == BodyType::Zarya || b2.body_type == BodyType::Zarya) && dist < (b1.body_type.visual_radius() as f64 + b2.body_type.visual_radius() as f64) {
                                    if main_collision_done {
                                        let pos = if b1.body_type == BodyType::Zarya { b1.pos } else { b2.pos };
                                        let vel = if b1.body_type == BodyType::Zarya { b1.vel } else { b2.vel };
                                        let id = if b1.body_type == BodyType::Zarya { b1.id } else { b2.id };

                                        let orange_debris = DebrisGen::generate_cloud(id, pos, vel, 200, sim_time, [1.0, 0.4, 0.0, 1.0]);
                                        new_debris.extend(orange_debris);
                                        events.push(CollisionEvent { time: sim_time, body_a_name: b1.name.clone(), body_b_name: b2.name.clone(), pos, new_debris_count: 200, is_primary: false });
                                        to_kill.insert(b1.id); to_kill.insert(b2.id);
                                    }
                                }
                                // General Satellite-Debris or Satellite-Satellite Fragmentation
                                else if (is_sat_1 || is_sat_2) && dist < COLLISION_DISTANCE {
                                    let pos = (b1.pos + b2.pos) * 0.5;
                                    
                                    // Fragment body 1 if it's a satellite
                                    if is_sat_1 {
                                        let name = format!("{} FRAG", b1.name);
                                        let pieces = DebrisGen::generate_cloud(b1.id, b1.pos, b1.vel, 60, sim_time, [1.0, 0.8, 0.0, 0.9]); // Golden
                                        for mut p in pieces { p.name = name.clone(); new_debris.push(p); }
                                        to_kill.insert(b1.id);
                                    }
                                    // Fragment body 2 if it's a satellite
                                    if is_sat_2 {
                                        let name = format!("{} FRAG", b2.name);
                                        let pieces = DebrisGen::generate_cloud(b2.id, b2.pos, b2.vel, 60, sim_time, [1.0, 0.8, 0.0, 0.9]); // Golden
                                        for mut p in pieces { p.name = name.clone(); new_debris.push(p); }
                                        to_kill.insert(b2.id);
                                    }

                                    events.push(CollisionEvent {
                                        time: sim_time,
                                        body_a_name: b1.name.clone(),
                                        body_b_name: b2.name.clone(),
                                        pos,
                                        new_debris_count: 120,
                                        is_primary: false,
                                    });
                                    println!("CASCADE COLLISION: {} hit {} -> Fragmented!", b1.name, b2.name);
                                }
                                // 2. Debris Interactions (Elastic-ish Bounce)
                                else if dist < INTERACTION_DISTANCE {
                                    bodies[i].highlight = 1.0;
                                    bodies[j].highlight = 1.0;
                                    let n = (bodies[i].pos - bodies[j].pos).normalize();
                                    let relative_vel = bodies[i].vel - bodies[j].vel;
                                    let v_normal = relative_vel.dot(n);
                                    if v_normal < 0.0 {
                                        let impulse = n * (-1.4 * v_normal);
                                        bodies[i].vel += impulse * 0.5;
                                        bodies[j].vel -= impulse * 0.5;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for id in &to_kill {
        if let Some(b) = bodies.iter_mut().find(|b| b.id == *id) {
            b.alive = false;
        }
    }

    (events, new_debris)
}
