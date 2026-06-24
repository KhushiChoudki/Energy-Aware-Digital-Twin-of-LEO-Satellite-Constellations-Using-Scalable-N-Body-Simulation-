// src/simulation/gnn_predictor.rs
// Pure-Rust Graph Predictive Engine (GNN logic for Collision Prediction)

use std::collections::HashMap;
use ndarray::Array2;
use crate::simulation::body::{Body, BodyType};
use glam::DVec3;

pub struct GnnPredictor {
    pub risk_map: HashMap<(u64, u64), f32>,
}

impl GnnPredictor {
    pub fn new() -> Self {
        println!("🚀 GNN Predictor: Graph-based Predictive Core initialized.");
        Self {
            risk_map: HashMap::new(),
        }
    }

    pub fn update(&mut self, bodies: &[Body]) {
        self.risk_map.clear();
        
        // 1. Build the Neighborhood Graph
        // Threshold: 800km for predictive look-ahead
        let mut neighbors = Vec::new();
        for i in 0..bodies.len() {
            if !bodies[i].alive { continue; }
            for j in i+1..bodies.len() {
                if !bodies[j].alive { continue; }
                
                let dist = (bodies[i].pos - bodies[j].pos).length();
                if dist < 800.0 {
                    neighbors.push((i, j, dist));
                }
            }
        }

        // 2. GNN-style Message Passing (Predictive Layer)
        // Predicts if objects will collide based on relative trajectory convergence
        for (i, j, dist) in neighbors {
            let b1 = &bodies[i];
            let b2 = &bodies[j];
            
            let relative_vel = b1.vel - b2.vel;
            let relative_pos = b1.pos - b2.pos;
            
            // Convergence Rate: How fast they are closing the gap
            let convergence = -relative_pos.normalize().dot(relative_vel);
            
            if convergence > 0.0 {
                // Time to Intercept (seconds)
                let time_to_closest = dist / convergence;
                
                if time_to_closest < 15.0 { // 15-second predictive window
                    // Linear Risk score (Higher if closer in time)
                    let risk = (15.0 - time_to_closest) as f32 / 15.0;
                    
                    // Boost risk if they are physically very close
                    let proximity_boost = if dist < 10.0 { 0.2 } else { 0.0 };
                    let final_risk = (risk + proximity_boost).min(1.0);
                    
                    if final_risk > 0.1 {
                        self.risk_map.insert((b1.id, b2.id), final_risk);
                    }
                }
            }
        }
    }

    /// Prepares node features for future ML integration
    pub fn extract_features(&self, b1: &Body, b2: &Body, dist: f64) -> Vec<f32> {
        let rel_vel = (b1.vel - b2.vel).length() as f32;
        let convergence = (-(b1.pos - b2.pos).normalize().dot(b1.vel - b2.vel)) as f32;
        
        vec![
            dist as f32,
            rel_vel,
            convergence,
            (b1.mass / b2.mass) as f32,
            (b1.pos.length() - 6371.0) as f32,
        ]
    }
}
