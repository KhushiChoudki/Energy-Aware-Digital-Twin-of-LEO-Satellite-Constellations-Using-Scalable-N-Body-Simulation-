// src/data/ephemeris.rs
// Parse Zarya ephemeris CSV (LLA + velocity) → ECI positions

use glam::DVec3;
use std::f64::consts::PI;

const RE: f64 = 6378.137; // km WGS84
const E2: f64 = 0.00669437999014; // WGS84 eccentricity squared
const OMEGA_E: f64 = 7.2921150e-5; // Earth rotation rate rad/s
// Julian date of 23 Apr 2026 06:30:00 UTC (sim start)
const SIM_EPOCH_JD: f64 = 2461589.770833; // approximate

#[derive(Debug, Clone)]
pub struct EphemerisPoint {
    pub time_s: f64,    // seconds since sim epoch
    pub pos_eci: DVec3, // km ECI
    pub vel_eci: DVec3, // km/s ECI
}

/// Parse the Zarya LLA ephemeris CSV
pub fn parse_zarya_ephemeris(content: &str) -> Vec<EphemerisPoint> {
    let mut points = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let base_time = parse_time_str("23 Apr 2026 06:30:00.000");

    for result in reader.records() {
        if let Ok(record) = result {
            if record.len() < 7 { continue; }
            let time_str = record.get(0).unwrap_or("").trim_matches('"');
            let lat_deg: f64 = record.get(1).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let lon_deg: f64 = record.get(2).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let alt_km: f64 = record.get(3).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let vx: f64 = record.get(4).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let vy: f64 = record.get(5).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let vz: f64 = record.get(6).unwrap_or("0").trim().parse().unwrap_or(0.0);

            let t = parse_time_str(time_str);
            let time_s = t - base_time;

            // LLA → ECEF
            let lat = lat_deg.to_radians();
            let lon = lon_deg.to_radians();
            let n = RE / (1.0 - E2 * lat.sin() * lat.sin()).sqrt();
            let x_ecef = (n + alt_km) * lat.cos() * lon.cos();
            let y_ecef = (n + alt_km) * lat.cos() * lon.sin();
            let z_ecef = (n * (1.0 - E2) + alt_km) * lat.sin();

            // ECEF → ECI: rotate by GMST (Greenwich Mean Sidereal Time)
            // Approximate GMST at sim epoch
            let gmst0 = gmst_at_epoch(SIM_EPOCH_JD);
            let theta = gmst0 + OMEGA_E * time_s;
            let pos_eci = ecef_to_eci(DVec3::new(x_ecef, y_ecef, z_ecef), theta);

            // Velocity: the CSV gives ECI velocity components directly (km/s)
            let vel_eci = DVec3::new(vx, vy, vz);

            points.push(EphemerisPoint { time_s, pos_eci, vel_eci });
        }
    }
    points
}

/// Interpolate position and velocity at a given time
pub fn interpolate_ephemeris(points: &[EphemerisPoint], time_s: f64) -> Option<(DVec3, DVec3)> {
    if points.is_empty() { return None; }
    if time_s <= points[0].time_s {
        return Some((points[0].pos_eci, points[0].vel_eci));
    }
    if time_s >= points[points.len()-1].time_s {
        return None; // beyond ephemeris
    }
    // Binary search for bracket
    let idx = points.partition_point(|p| p.time_s <= time_s);
    let i = idx.saturating_sub(1).min(points.len()-2);
    let p0 = &points[i];
    let p1 = &points[i+1];
    let dt = p1.time_s - p0.time_s;
    if dt < 1e-9 {
        return Some((p0.pos_eci, p0.vel_eci));
    }
    let t = (time_s - p0.time_s) / dt;
    let pos = p0.pos_eci.lerp(p1.pos_eci, t);
    let vel = p0.vel_eci.lerp(p1.vel_eci, t);
    Some((pos, vel))
}

fn ecef_to_eci(ecef: DVec3, theta: f64) -> DVec3 {
    let (s, c) = theta.sin_cos();
    DVec3::new(
        c * ecef.x - s * ecef.y,
        s * ecef.x + c * ecef.y,
        ecef.z,
    )
}

fn gmst_at_epoch(jd: f64) -> f64 {
    // Simple GMST approximation
    let t = (jd - 2451545.0) / 36525.0;
    let gmst_deg = 280.46061837 + 360.98564736629 * (jd - 2451545.0)
        + t * t * 0.000387933
        - t * t * t / 38710000.0;
    gmst_deg.to_radians().rem_euclid(2.0 * PI)
}

/// Parse "23 Apr 2026 06:30:00.000" → seconds since J2000
fn parse_time_str(s: &str) -> f64 {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 4 { return 0.0; }
    let day: i32 = parts[0].parse().unwrap_or(1);
    let month = match parts[1] {
        "Jan" => 1, "Feb" => 2, "Mar" => 3, "Apr" => 4,
        "May" => 5, "Jun" => 6, "Jul" => 7, "Aug" => 8,
        "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12,
        _ => 1,
    };
    let year: i32 = parts[2].parse().unwrap_or(2026);
    let hms: Vec<f64> = parts[3].split(':').map(|x| x.parse().unwrap_or(0.0)).collect();
    let h = hms.get(0).copied().unwrap_or(0.0);
    let m = hms.get(1).copied().unwrap_or(0.0);
    let sec = hms.get(2).copied().unwrap_or(0.0);

    // Convert to JD then to seconds since J2000 (JD 2451545.0)
    let jd = date_to_jd(year, month, day, h, m, sec);
    (jd - 2451545.0) * 86400.0
}

fn date_to_jd(y: i32, m: i32, d: i32, h: f64, mi: f64, s: f64) -> f64 {
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y as f64 + 4716.0)).floor()
        + (30.6001 * (m as f64 + 1.0)).floor()
        + d as f64 + b - 1524.5
        + (h + mi / 60.0 + s / 3600.0) / 24.0
}
