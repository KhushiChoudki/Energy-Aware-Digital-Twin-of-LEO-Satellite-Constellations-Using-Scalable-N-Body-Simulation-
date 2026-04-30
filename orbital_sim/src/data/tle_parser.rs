// src/data/tle_parser.rs
// Simplified SGP4-like TLE propagator for RUSSS and Iridium-33

use glam::DVec3;
use std::f64::consts::PI;

const MU: f64 = 398600.4418; // km^3/s^2
const RE: f64 = 6378.137;    // km
const J2: f64 = 1.08262668e-3;

pub struct TleElements {
    pub name: String,
    pub inclination: f64,    // radians
    pub raan: f64,           // radians - right ascension of ascending node
    pub eccentricity: f64,
    pub arg_perigee: f64,    // radians
    pub mean_anomaly: f64,   // radians
    pub mean_motion: f64,    // rad/s
    pub epoch_jd: f64,       // Julian Day
}

impl TleElements {
    /// Parse a 3-line TLE (name + line1 + line2)
    pub fn from_lines(name: &str, line1: &str, line2: &str) -> Self {
        let inc = parse_f64(line2, 8, 16).to_radians();
        let raan = parse_f64(line2, 17, 25).to_radians();
        let ecc_str = format!("0.{}", line2[26..33].trim());
        let ecc = ecc_str.parse::<f64>().unwrap_or(0.0);
        let argp = parse_f64(line2, 34, 42).to_radians();
        let ma = parse_f64(line2, 43, 51).to_radians();
        // Mean motion in rev/day → rad/s
        let mm_revday = parse_f64(line2, 52, 63);
        let mm_rads = mm_revday * 2.0 * PI / 86400.0;

        // Epoch from line1: YYDDD.DDDDDDDD
        let epoch_str = line1[18..32].trim();
        let epoch_jd = parse_epoch_to_jd(epoch_str);

        TleElements {
            name: name.to_string(),
            inclination: inc,
            raan,
            eccentricity: ecc,
            arg_perigee: argp,
            mean_anomaly: ma,
            mean_motion: mm_rads,
            epoch_jd,
        }
    }

    /// Semi-major axis from mean motion (km)
    pub fn semi_major_axis(&self) -> f64 {
        (MU / (self.mean_motion * self.mean_motion)).cbrt()
    }

    /// Propagate to given time offset from epoch (seconds)
    /// Returns (position_km, velocity_km_s) in ECI
    pub fn propagate(&self, dt_seconds: f64) -> (DVec3, DVec3) {
        let a = self.semi_major_axis();
        let e = self.eccentricity;
        let n = self.mean_motion;

        // Mean anomaly at time t
        let m = (self.mean_anomaly + n * dt_seconds).rem_euclid(2.0 * PI);

        // Solve Kepler's equation via Newton-Raphson
        let ea = solve_kepler(m, e);

        // True anomaly
        let sin_ta = (1.0 - e * e).sqrt() * ea.sin() / (1.0 - e * ea.cos());
        let cos_ta = (ea.cos() - e) / (1.0 - e * ea.cos());
        let ta = sin_ta.atan2(cos_ta);

        // Radial distance
        let r = a * (1.0 - e * ea.cos());

        // Position in perifocal frame (PQW)
        let p_pqw = DVec3::new(r * ta.cos(), r * ta.sin(), 0.0);

        // Velocity in perifocal frame
        let h = (MU * a * (1.0 - e * e)).sqrt();
        let v_pqw = DVec3::new(
            -MU / h * ta.sin(),
            MU / h * (e + ta.cos()),
            0.0,
        );

        // Rotation matrices: PQW → ECI
        let (pos, vel) = perifocal_to_eci(
            p_pqw, v_pqw,
            self.raan, self.inclination, self.arg_perigee
        );

        (pos, vel)
    }
}

/// Solve Kepler's equation M = E - e*sin(E) using Newton-Raphson
fn solve_kepler(m: f64, e: f64) -> f64 {
    let mut ea = m;
    for _ in 0..50 {
        let f = ea - e * ea.sin() - m;
        let fp = 1.0 - e * ea.cos();
        let delta = f / fp;
        ea -= delta;
        if delta.abs() < 1e-12 { break; }
    }
    ea
}

/// Rotate perifocal frame to ECI using RAAN, Inc, ArgP
fn perifocal_to_eci(pos: DVec3, vel: DVec3, raan: f64, inc: f64, argp: f64) -> (DVec3, DVec3) {
    let (sr, cr) = raan.sin_cos();
    let (si, ci) = inc.sin_cos();
    let (sw, cw) = argp.sin_cos();

    // DCM columns (row-major stored)
    let r11 = cr * cw - sr * sw * ci;
    let r12 = -cr * sw - sr * cw * ci;
    let r21 = sr * cw + cr * sw * ci;
    let r22 = -sr * sw + cr * cw * ci;
    let r31 = sw * si;
    let r32 = cw * si;

    let px = r11 * pos.x + r12 * pos.y;
    let py = r21 * pos.x + r22 * pos.y;
    let pz = r31 * pos.x + r32 * pos.y;

    let vx = r11 * vel.x + r12 * vel.y;
    let vy = r21 * vel.x + r22 * vel.y;
    let vz = r31 * vel.x + r32 * vel.y;

    (DVec3::new(px, py, pz), DVec3::new(vx, vy, vz))
}

fn parse_f64(s: &str, start: usize, end: usize) -> f64 {
    let end = end.min(s.len());
    if start >= end { return 0.0; }
    s[start..end].trim().parse::<f64>().unwrap_or(0.0)
}

fn parse_epoch_to_jd(epoch: &str) -> f64 {
    if epoch.len() < 5 { return 2451545.0; }
    let year_2d = epoch[..2].parse::<i32>().unwrap_or(0);
    let year = if year_2d < 57 { 2000 + year_2d } else { 1900 + year_2d };
    let day_of_year: f64 = epoch[2..].parse().unwrap_or(1.0);
    // Julian day for Jan 1.5 of year
    let jd_jan1 = 367.0 * year as f64
        - (7.0 * (year as f64 + 1.0) / 4.0).floor()
        + 275.0 * 1.0 / 9.0
        + 1721013.5;
    jd_jan1 + day_of_year - 1.0
}

/// Load TLE from a 3-line text file content
pub fn load_tle(content: &str) -> TleElements {
    let lines: Vec<&str> = content.lines().collect();
    let name = lines.get(0).copied().unwrap_or("UNKNOWN").trim();
    let line1 = lines.get(1).copied().unwrap_or("").trim();
    let line2 = lines.get(2).copied().unwrap_or("").trim();
    TleElements::from_lines(name, line1, line2)
}
