// src/data/aer_decoder.rs
// Decode AER (Azimuth, Elevation, Range) relative to Zarya → ECI debris positions

use glam::DVec3;
use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct DebrisPoint {
    pub time_s: f64,
    pub pos_eci: DVec3,
    pub vel_eci: DVec3, // estimated from finite-diff
}

/// Parse AER CSV and decode to ECI positions using Zarya as origin
pub fn parse_debris_aer(
    aer_content: &str,
    zarya_ephemeris: &[crate::data::ephemeris::EphemerisPoint],
) -> Vec<DebrisPoint> {
    let mut raw: Vec<(f64, f64, f64, f64)> = Vec::new(); // (time_s, az, el, range)

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(aer_content.as_bytes());

    let base_time = parse_time_s("23 Apr 2026 06:30:00.000");

    for result in reader.records() {
        if let Ok(record) = result {
            if record.len() < 4 { continue; }
            let time_str = record.get(0).unwrap_or("").trim_matches('"');
            let az: f64 = record.get(1).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let el: f64 = record.get(2).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let rng: f64 = record.get(3).unwrap_or("0").trim().parse().unwrap_or(0.0);
            let t = parse_time_s(time_str) - base_time;
            if t >= 0.0 {
                raw.push((t, az, el, rng));
            }
        }
    }

    // Convert each AER point to ECI
    let mut points: Vec<DebrisPoint> = Vec::new();
    for &(time_s, az_deg, el_deg, rng_km) in &raw {
        let zarya_state = crate::data::ephemeris::interpolate_ephemeris(zarya_ephemeris, time_s);
        if let Some((zarya_pos, zarya_vel)) = zarya_state {
            let pos_eci = aer_to_eci(az_deg, el_deg, rng_km, zarya_pos, zarya_vel);
            points.push(DebrisPoint {
                time_s,
                pos_eci,
                vel_eci: DVec3::ZERO, // filled in next pass
            });
        }
    }

    // Estimate velocity via finite differences
    let n = points.len();
    for i in 0..n {
        if i > 0 && i < n-1 {
            let dt = points[i+1].time_s - points[i-1].time_s;
            if dt > 1e-6 {
                let vel = (points[i+1].pos_eci - points[i-1].pos_eci) / dt;
                points[i].vel_eci = vel;
            }
        } else if i == 0 && n > 1 {
            let dt = points[1].time_s - points[0].time_s;
            if dt > 1e-6 {
                points[0].vel_eci = (points[1].pos_eci - points[0].pos_eci) / dt;
            }
        } else if i == n-1 && n > 1 {
            let dt = points[n-1].time_s - points[n-2].time_s;
            if dt > 1e-6 {
                points[n-1].vel_eci = (points[n-1].pos_eci - points[n-2].pos_eci) / dt;
            }
        }
    }

    points
}

/// Convert AER (local spacecraft frame) to absolute ECI
/// Zarya velocity used to compute local North-East-Down frame
fn aer_to_eci(az_deg: f64, el_deg: f64, rng_km: f64, zarya_pos: DVec3, zarya_vel: DVec3) -> DVec3 {
    let az = az_deg.to_radians();
    let el = el_deg.to_radians();

    // Local frame: Z = -nadir (up), X = north, Y = east (SEZ frame)
    // Range vector in SEZ
    let r_s = -rng_km * el.sin(); // south component = down when el<0
    let r_e = rng_km * el.cos() * az.sin();
    let r_z = rng_km * el.cos() * az.cos(); // "north" approximation

    // Compute NED from zarya_pos
    let pos_unit = zarya_pos.normalize();
    let up = pos_unit;
    // East = cross(up, north_approx) ≈ velocity direction projected
    let north_approx = DVec3::new(0.0, 0.0, 1.0); // toward north pole
    let east = up.cross(north_approx).normalize();
    let north = east.cross(up).normalize();

    let offset = north * r_z + east * r_e + up * r_s;
    zarya_pos + offset
}

fn parse_time_s(s: &str) -> f64 {
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
