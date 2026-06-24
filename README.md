# Energy-Aware Digital Twin of LEO Satellite Constellations

![Orbital Simulation](https://img.shields.io/badge/Status-Development-orange)
![Rust](https://img.shields.io/badge/Language-Rust-red)
![WGPU](https://img.shields.io/badge/Graphics-WGPU-blue)

A high-performance, industry-standard digital twin for Low Earth Orbit (LEO) satellite constellations. This project couples scalable N-body physics simulation (Barnes-Hut algorithm), real-time Graph Neural Network (GNN) collision prediction, and Reinforcement Learning (RL) autonomous evasion maneuvers with dynamic energy and network uptime tracking.

<img width="100%" alt="System Overview" src="https://github.com/user-attachments/assets/37deed48-3781-4401-afb0-5f5feaea4fa1" />

## Table of Contents
- [Overview](#overview)
- [Key Features](#key-features)
- [System Architecture](#system-architecture)
- [Technology Stack](#technology-stack)
- [Simulation Mechanics & Physics](#simulation-mechanics--physics)
- [Reinforcement Learning Auto-Evasion](#reinforcement-learning-auto-evasion)
- [Energy & Network Analysis](#energy--network-analysis)
- [Getting Started](#getting-started)
- [Authors](#authors)

## Overview

As orbital zones become increasingly congested, manual monitoring of satellite constellations is no longer viable. This project provides a **Digital Twin** that seamlessly integrates:
1. **Kinetic Realism**: Simulating thousands of debris objects using GPU-accelerated scalable N-Body physics.
2. **Predictive AI**: A GNN that continuously maps collision risks based on spatial proximity and velocity, proactively identifying high-risk conjunctions.
3. **Prescriptive Autonomy**: An RL Agent capable of performing autonomous Delta-V maneuvers to evade threats while minimizing battery consumption and maintaining service availability.
4. **Energy Dynamics**: Tracking energy usage, solar generation, and data transmission capabilities in a unified model to ensure robust mission lifecycles.

<img width="100%" alt="Collision Avoidance View" src="https://github.com/user-attachments/assets/b1c34296-cf78-4f59-8a3b-9dae13041a7e" />

## Key Features

- **Scalable N-Body Barnes-Hut Gravity Engine**: Simulates gravitational interactions between thousands of objects without O(N^2) bottlenecks, enabling massive-scale scenarios.
- **Real-Time WGPU Rendering**: High-performance Rust-based rendering capable of drawing tens of thousands of debris pieces, complete with orbital trails and cinematic engine flares during maneuvers.
- **Dynamic Movable Telemetry HUD**: Clean, professional `egui`-based interface where all windows can be dragged and rearranged for optimal presentation flow. Includes real-time graphs and state tracking.
- **Autonomous Evasion AI**: Configurable RL agent that calculates risk and can automatically trigger life-saving thrusts.
- **Power & Data Coupling**: Realistic Line-of-Sight (LOS) calculations to ground stations that dictate data transmission windows, draining battery reserves that are replenished by solar energy based on sun-exposure phases.

## Technology Stack

- **Backend & Physics Engine**: Rust, leveraging zero-cost abstractions for compute-heavy N-Body simulations.
- **Graphics API**: WGPU (WebGPU implementation for Rust), providing cross-platform GPU access.
- **UI Toolkit**: `egui` for immediate-mode GUI, enabling highly responsive and customizable HUD elements.
- **Machine Learning**: Custom integrations for Graph Neural Networks and Reinforcement Learning algorithms.

## System Architecture

The architecture relies on a highly concurrent Rust backend that feeds physics state data into the rendering pipeline while synchronously updating the GNN feature graphs. The pipeline is designed for minimal latency, ensuring smooth frame rates even with tens of thousands of tracked objects.

<div align="center">
  <img height="400" alt="System Architecture Diagram" src="https://github.com/user-attachments/assets/6089fd75-6723-49b9-9857-2063d4a4fb1a" />
</div>

## Simulation Mechanics & Physics

- **TLE Parsing & Propagation**: Primary satellites and known debris clouds are initialized via SGP4-compatible TLE parsing from live or historical datasets.
- **Kinetic Detachment**: Once a maneuver is initiated, the satellite dynamically detaches from static TLE propagation and transitions into a 4th-order Runge-Kutta (RK4) physics integrator to accurately map its new orbit.
- **Collision Generation**: The system dynamically models historical events (e.g., Cosmos-2251 & Iridium-33 collision), spawning thousands of kinetic fragments that physically interact with the environment.

## Reinforcement Learning Auto-Evasion

The AI dashboard allows users to toggle between **Manual Mode** (where the system calculates required Delta-V and waits for human confirmation) and **Auto-Evasion Mode** (where the RL agent takes direct control over satellite thrusters).

| Agent Parameter | Value / Formula |
|-----------------|-----------------|
| Collision Threshold | > 80% Risk |
| Observation Space | `[ΔPos, ΔVel, Battery, Mass]` |
| Reward Function | `+100` (Survival), `-10 * Δv` (Fuel penalty) |

*The agent's visual output highlights the selected satellite in red (selection) and transitions to a massive bright yellow flare during the 5-second active burn.*

## Energy & Network Analysis

Maneuvers are not free. Firing thrusters or transmitting data while in LOS of ground stations heavily depletes battery reserves. If a satellite triggers an evasion maneuver while at low power, it risks complete system failure. The digital twin visualizes these critical trade-offs in real-time.

<img width="100%" alt="Energy and Uptime Graph" src="https://github.com/user-attachments/assets/c575df93-6450-48eb-b567-7be0db50bd1e" />

**Analytical Insights:**
- **Green Bars**: Successful network transmission windows (LOS established, battery sufficient).
- **Red Bars**: Outages caused by battery depletion or lost LOS due to emergency orbital changes.
- **Blue Line**: Dynamic battery capacity (100% to 0%).

## Getting Started

### Prerequisites
- **Rust**: Ensure you have the latest stable Rust toolchain installed (1.70+ recommended).
- **Vulkan/DirectX12/Metal**: A compatible GPU for WGPU rendering.

### Build and Run
1. Clone the repository.
2. Ensure `tle_dataset.csv` is in the root directory.
3. Run the optimized build:
```bash
cargo run --release
```

### Controls
- **Left Click & Drag**: Rotate camera.
- **Right Click & Drag / Scroll**: Zoom in/out.
- **Left Click on Satellite**: Select target (enables Red Pointer and locks tracking camera).
- **Click Empty Space**: Deselect target.

## Authors

- **Khushi Choudki**, Dept of ISE, RV College of Engineering (khushichoudki.is23@rvce.edu.in)
- **Keerthi M**, Dept of ISE, RV College of Engineering (keerthim.is23@rvce.edu.in)
- **Tejas L**, Dept of ASE, RV College of Engineering (tejasl.ae23@rvce.edu.in)
- **Akula Uday Kiran**, Dept of ASE, RV College of Engineering (akulauday.ae23@rvce.edu.in)
- **Dr. Rachana S Akki**, Dept of EEE, RV College of Engineering (rachana.akki@rvce.edu.in)
