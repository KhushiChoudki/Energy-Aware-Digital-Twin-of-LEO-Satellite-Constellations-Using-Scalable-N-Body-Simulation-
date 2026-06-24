// src/simulation/debris_gen.rs
// Realistic fragmentation generator based on NASA Standard Breakup Model principles

use glam::DVec3;
use rand::Rng;
use rand_distr::{Distribution, LogNormal};
use crate::simulation::body::{Body, BodyType};

pub struct DebrisGen;

impl DebrisGen {
    /// Generate a cloud of debris from a collision event
    pub fn generate_cloud(
        parent_id: u64,
        pos: DVec3,
        vel: DVec3,
        parent_mass: f64,
        count: usize,
        time: f64,
        color: [f32; 4],
    ) -> Vec<Body> {
        let mut rng = rand::thread_rng();
        let mut cloud = Vec::with_capacity(count);
        let mut impulses = Vec::with_capacity(count);
        let mut total_impulse = DVec3::ZERO;

        let dv_dist = LogNormal::new(-1.5, 0.8).unwrap(); 
        let frag_mass = parent_mass / (count as f64);

        // 1. Generate Raw Impulses
        for _ in 0..count {
            let theta = rng.gen_range(0.0..std::f64::consts::TAU);
            let phi = (rng.gen_range(-1.0..1.0) as f64).acos();
            let dir = DVec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos());
            let dv_mag: f64 = dv_dist.sample(&mut rng);
            let dv_mag = dv_mag.min(2.0);
            
            let impulse = dir * dv_mag;
            total_impulse += impulse;
            impulses.push(impulse);
        }

        // 2. Momentum Correction (Ensure sum of impulses = 0)
        let correction = total_impulse / (count as f64);

        // 3. Create Bodies with Corrected Velocities and Mass
        for i in 0..count {
            let corrected_vel = vel + (impulses[i] - correction);
            
            let mut body = Body::new(
                format!("DEB-{}-{}", parent_id, i),
                BodyType::CollisionDebris,
                pos,
                corrected_vel,
                frag_mass,
                0.005,
                time,
            );
            body.color_override = Some(color);
            cloud.push(body);
        }

        cloud
    }
}
