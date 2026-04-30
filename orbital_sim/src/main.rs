// src/main.rs - Entry point for Orbital Collision Simulation
mod data;
mod renderer;
mod simulation;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    renderer::app::run()
}
