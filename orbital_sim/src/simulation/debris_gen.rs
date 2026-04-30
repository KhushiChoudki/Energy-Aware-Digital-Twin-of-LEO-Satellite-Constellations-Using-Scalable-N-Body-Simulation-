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
        count: usize,
        time: f64,
        color: [f32; 4],
    ) -> Vec<Body> {
        let mut rng = rand::thread_rng();
        let mut cloud = Vec::with_capacity(count);

        // NSBM-like velocity dispersion:
        // Spreads fragments into a characteristic elliptical shell (Gabbard diagram shape)
        let dv_dist = LogNormal::new(-1.0, 0.7).unwrap(); 

        for i in 0..count {
            // Random direction for the delta-v impulse
            let theta = rng.gen_range(0.0..std::f64::consts::TAU);
            let phi = (rng.gen_range(-1.0..1.0) as f64).acos();
            
            let dir = DVec3::new(
                phi.sin() * theta.cos(),
                phi.sin() * theta.sin(),
                phi.cos()
            );

            // Delta-V magnitude typically ranges from m/s to several hundred m/s
            // In our km/s units, 0.1 = 100 m/s
            let dv_mag: f64 = dv_dist.sample(&mut rng);
            let dv_mag = dv_mag.min(1.5); // Cap extreme outliers
            
            // Add slight position jitter to simulate explosion volume
            let pos_jitter = dir * rng.gen_range(0.0..0.1); 
            
            let debris_vel = vel + (dir * dv_mag);

            let mut body = Body::new(
                format!("DEB-{}-{}", parent_id, i),
                BodyType::CollisionDebris,
                pos + pos_jitter,
                debris_vel,
                0.01,
                0.005,
                time,
            );
            body.color_override = Some(color);
            cloud.push(body);
        }

        cloud
    }
}
