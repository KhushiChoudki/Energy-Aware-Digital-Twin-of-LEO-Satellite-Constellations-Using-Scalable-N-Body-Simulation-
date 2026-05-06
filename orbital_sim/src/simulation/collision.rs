// src/simulation/collision.rs
// Grid-based collision detection with High-Fidelity Radii-Sum logic

use glam::DVec3;
use crate::simulation::body::{Body, BodyType};
use crate::simulation::debris_gen::DebrisGen;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CollisionEvent {
    pub time: f64,
    pub body_a_name: String,
    pub body_b_name: String,
    pub pos: DVec3,
    pub new_debris_count: usize,
    pub is_primary: bool,
}

const CONJUNCTION_THRESHOLD: f64 = 5.0; // km for visual "near-miss" highlight
const INTERACTION_DISTANCE: f64 = 0.5; // km for debris interaction (elastic bounce)
const GRID_CELL_SIZE: f64 = 80.0; // km for efficient spatial partitioning

pub fn check_collisions(
    bodies: &mut Vec<Body>,
    sim_time: f64,
    _russs_id: Option<u64>,
    _iridium_id: Option<u64>,
    main_collision_done: bool,
) -> (Vec<CollisionEvent>, Vec<Body>) {
    let mut events = Vec::new();
    let mut new_debris: Vec<Body> = Vec::new();
    let mut to_kill = HashSet::new();
    let mut to_highlight = HashSet::new();

    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (idx, body) in bodies.iter().enumerate() {
        if !body.alive { continue; }
        let gx = (body.pos.x / GRID_CELL_SIZE).floor() as i32;
        let gy = (body.pos.y / GRID_CELL_SIZE).floor() as i32;
        let gz = (body.pos.z / GRID_CELL_SIZE).floor() as i32;
        grid.entry((gx, gy, gz)).or_default().push(idx);
    }

    let mut pairs_checked = HashSet::new();
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
                                
                                // Check distance using indices first to avoid borrowing bodies
                                let dist = (bodies[i].pos - bodies[j].pos).length();
                                let collision_threshold = bodies[i].radius + bodies[j].radius;

                                // 1. Conjunction Highlight
                                if dist < CONJUNCTION_THRESHOLD {
                                    to_highlight.insert(i);
                                    to_highlight.insert(j);
                                }

                                // 2. Physical Collision
                                let is_sat_1 = matches!(bodies[i].body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                                let is_sat_2 = matches!(bodies[j].body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya);
                                
                                if (bodies[i].body_type == BodyType::Zarya || bodies[j].body_type == BodyType::Zarya) && dist < collision_threshold {
                                    if main_collision_done {
                                        let (b_idx, o_idx) = if bodies[i].body_type == BodyType::Zarya { (i, j) } else { (j, i) };
                                        let pos = bodies[b_idx].pos;
                                        let vel = bodies[b_idx].vel;
                                        let id = bodies[b_idx].id;

                                        let orange_debris = DebrisGen::generate_cloud(id, pos, vel, bodies[b_idx].mass, 200, sim_time, [1.0, 0.4, 0.0, 1.0]);
                                        new_debris.extend(orange_debris);
                                        events.push(CollisionEvent { 
                                            time: sim_time, 
                                            body_a_name: bodies[b_idx].name.clone(), 
                                            body_b_name: bodies[o_idx].name.clone(), 
                                            pos, 
                                            new_debris_count: 200, 
                                            is_primary: false 
                                        });
                                        to_kill.insert(bodies[i].id); to_kill.insert(bodies[j].id);
                                    }
                                }
                                else if (is_sat_1 || is_sat_2) && dist < collision_threshold {
                                    let pos = (bodies[i].pos + bodies[j].pos) * 0.5;
                                    
                                    if is_sat_1 {
                                        let name = format!("{} FRAG", bodies[i].name);
                                        let pieces = DebrisGen::generate_cloud(bodies[i].id, bodies[i].pos, bodies[i].vel, bodies[i].mass, 60, sim_time, [1.0, 0.8, 0.0, 0.9]);
                                        for mut p in pieces { p.name = name.clone(); new_debris.push(p); }
                                        to_kill.insert(bodies[i].id);
                                    }
                                    if is_sat_2 {
                                        let name = format!("{} FRAG", bodies[j].name);
                                        let pieces = DebrisGen::generate_cloud(bodies[j].id, bodies[j].pos, bodies[j].vel, bodies[j].mass, 60, sim_time, [1.0, 0.8, 0.0, 0.9]);
                                        for mut p in pieces { p.name = name.clone(); new_debris.push(p); }
                                        to_kill.insert(bodies[j].id);
                                    }

                                    events.push(CollisionEvent {
                                        time: sim_time,
                                        body_a_name: bodies[i].name.clone(),
                                        body_b_name: bodies[j].name.clone(),
                                        pos,
                                        new_debris_count: 120,
                                        is_primary: false,
                                     });
                                     println!("PHYSICAL COLLISION: {} hit {} at {:.3}km distance!", bodies[i].name, bodies[j].name, dist);
                                }
                                // 3. Debris Interactions
                                 else if dist < INTERACTION_DISTANCE {
                                    let n = (bodies[i].pos - bodies[j].pos).normalize();
                                    let relative_vel = bodies[i].vel - bodies[j].vel;
                                    let v_normal = relative_vel.dot(n);
                                    if v_normal < 0.0 {
                                         // Realistic 1D inelastic collision with mass weighting
                                         let m1 = bodies[i].mass;
                                         let m2 = bodies[j].mass;
                                         let restitution = 0.6; 
                                         let j_impulse = -(1.0 + restitution) * v_normal / (1.0/m1 + 1.0/m2);
                                         
                                         let impulse_vec = n * j_impulse;
                                         bodies[i].vel += impulse_vec / m1;
                                         bodies[j].vel -= impulse_vec / m2;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Apply highlights
    for idx in to_highlight {
        bodies[idx].highlight = 1.0;
    }

    for id in &to_kill {
        if let Some(b) = bodies.iter_mut().find(|b| b.id == *id) {
            b.alive = false;
        }
    }

    (events, new_debris)
}
