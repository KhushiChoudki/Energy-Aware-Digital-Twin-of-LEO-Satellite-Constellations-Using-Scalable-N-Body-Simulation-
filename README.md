# Energy-Aware Digital Twin of LEO Satellite Constellations Using Scalable N-Body Simulation

![Orbital Simulation](https://img.shields.io/badge/Status-Development-orange)
![Rust](https://img.shields.io/badge/Language-Rust-red)
![WGPU](https://img.shields.io/badge/Graphics-WGPU-blue)

A high-fidelity, real-time orbital simulation engine designed for modeling Low Earth Orbit (LEO) satellite constellations, collision events, and debris propagation (Kessler Syndrome). This project implements a scalable N-body simulation using the Barnes-Hut algorithm and provides a 3D visualization of the orbital environment.

## 🚀 Key Features

- **Scalable N-Body Simulation**: Uses the **Barnes-Hut algorithm** for efficient gravity calculations across thousands of bodies.
- **High-Fidelity Propagators**: Implements **Velocity Verlet** and **Runge-Kutta 4 (RK4)** integration for precise orbital mechanics.
- **Kessler Syndrome Modeling**: Real-time collision detection and fragment generation. Includes a pre-configured scenario for the **Iridium-33 and Cosmos-2251 collision**.
- **Advanced 3D Rendering**: Built with **WGPU** for high-performance GPU-accelerated visualization, featuring custom shaders and a dynamic camera system.
- **Digital Twin Integration**: Supports loading ephemeris data (CSV) and TLEs for real-world satellite tracking and "what-if" scenario analysis.
- **Interactive UI**: Built with **egui**, allowing real-time control over time scale, camera modes, and object highlighting.

## 🛠 Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **Graphics API**: [WGPU](https://wgpu.rs/) (Cross-platform GPU abstraction)
- **Math**: [glam](https://github.com/bitshifter/glam-rs) (Linear algebra for games and graphics)
- **UI**: [egui](https://github.com/emilk/egui)
- **Integration**: Velocity Verlet & RK4

## 📦 Installation

### Prerequisites

- [Rust & Cargo](https://rustup.rs/) (Latest stable version)
- A GPU supporting Vulkan, Metal, or DX12.

### Build and Run

1. Clone the repository:
   ```bash
   git clone https://github.com/KhushiChoudki/Energy-Aware-Digital-Twin-of-LEO-Satellite-Constellations-Using-Scalable-N-Body-Simulation-.git
   cd Energy-Aware-Digital-Twin-of-LEO-Satellite-Constellations-Using-Scalable-N-Body-Simulation-
   ```

2. Navigate to the simulator directory:
   ```bash
   cd orbital_sim
   ```

3. Run the simulation:
   ```bash
   cargo run --release
   ```

## 📂 Project Structure

- `src/simulation/`: Core physics engine, including Barnes-Hut, integrators, and collision logic.
- `src/renderer/`: WGPU rendering pipeline, camera systems, and UI components.
- `src/data/`: TLE and ephemeris data parsers.
- `matlab/`: Supporting scripts for breakup models and data analysis.

## 📊 Performance

The Barnes-Hut implementation allows the simulation to handle thousands of debris pieces simultaneously at high frame rates, making it suitable for long-term Kessler Syndrome projections.

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---
*Developed as part of research into Energy-Aware Digital Twins for LEO constellations.*
