// src/simulation/integrator.rs
// Velocity-Verlet integrator with Earth's gravity

const MU: f64 = 398600.4418; // km^3/s^2

use glam::DVec3;

/// Compute gravitational acceleration from Earth alone (km/s^2)
pub fn earth_gravity(pos: DVec3) -> DVec3 {
    let r2 = pos.length_squared();
    let r = r2.sqrt();
    -pos * MU / (r2 * r)
}

/// Single Velocity-Verlet step for a free body (no BH)
pub fn verlet_step(pos: &mut DVec3, vel: &mut DVec3, dt: f64, extra_acc: DVec3) {
    let a0 = earth_gravity(*pos) + extra_acc;
    let new_pos = *pos + *vel * dt + a0 * (0.5 * dt * dt);
    let a1 = earth_gravity(new_pos) + extra_acc;
    *vel += (a0 + a1) * (0.5 * dt);
    *pos = new_pos;
}

/// RK4 step – more accurate for large time steps
pub fn rk4_step(pos: &mut DVec3, vel: &mut DVec3, dt: f64) {
    let accel = |p: DVec3| earth_gravity(p);

    let k1v = accel(*pos);
    let k1r = *vel;

    let k2v = accel(*pos + k1r * (dt * 0.5));
    let k2r = *vel + k1v * (dt * 0.5);

    let k3v = accel(*pos + k2r * (dt * 0.5));
    let k3r = *vel + k2v * (dt * 0.5);

    let k4v = accel(*pos + k3r * dt);
    let k4r = *vel + k3v * dt;

    *pos += (k1r + k2r * 2.0 + k3r * 2.0 + k4r) * (dt / 6.0);
    *vel += (k1v + k2v * 2.0 + k3v * 2.0 + k4v) * (dt / 6.0);
}
