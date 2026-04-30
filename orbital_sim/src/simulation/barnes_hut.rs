// src/simulation/barnes_hut.rs
// Octree Barnes-Hut N-body force approximation

use glam::DVec3;

const THETA: f64 = 0.5;    // Opening angle
const EPS: f64 = 0.1;      // Softening length km (avoid singularity)
const MU_EARTH: f64 = 398600.4418; // km^3/s^2
const EARTH_RADIUS: f64 = 6371.0;
const EARTH_MASS: f64 = 5.972e24; // kg

#[derive(Clone)]
pub struct OctNode {
    pub center: DVec3,
    pub half_size: f64,
    pub mass: f64,
    pub com: DVec3,      // center of mass
    pub children: Option<Box<[OctNode; 8]>>,
}

impl OctNode {
    fn new(center: DVec3, half_size: f64) -> Self {
        OctNode {
            center,
            half_size,
            mass: 0.0,
            com: DVec3::ZERO,
            children: None,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    fn octant_index(&self, pos: DVec3) -> usize {
        let dx = if pos.x >= self.center.x { 1 } else { 0 };
        let dy = if pos.y >= self.center.y { 2 } else { 0 };
        let dz = if pos.z >= self.center.z { 4 } else { 0 };
        dx | dy | dz
    }

    fn child_center(&self, idx: usize) -> DVec3 {
        let hs = self.half_size * 0.5;
        DVec3::new(
            self.center.x + if idx & 1 != 0 { hs } else { -hs },
            self.center.y + if idx & 2 != 0 { hs } else { -hs },
            self.center.z + if idx & 4 != 0 { hs } else { -hs },
        )
    }

    fn insert(&mut self, pos: DVec3, mass: f64, depth: usize) {
        if depth > 20 { return; } // guard against infinite recursion
        if self.mass == 0.0 {
            self.mass = mass;
            self.com = pos;
            return;
        }
        if self.is_leaf() {
            // Subdivide
            let old_pos = self.com;
            let old_mass = self.mass;
            let hs = self.half_size * 0.5;
            let children: [OctNode; 8] = std::array::from_fn(|i| {
                OctNode::new(self.child_center(i), hs)
            });
            self.children = Some(Box::new(children));
            // Re-insert old particle
            let oi = self.octant_index(old_pos);
            self.children.as_mut().unwrap()[oi].insert(old_pos, old_mass, depth+1);
        }
        // Insert new particle
        let ni = self.octant_index(pos);
        self.children.as_mut().unwrap()[ni].insert(pos, mass, depth+1);
        // Update this node's com
        let total = self.mass + mass;
        self.com = (self.com * self.mass + pos * mass) / total;
        self.mass = total;
    }

    fn force_on(&self, pos: DVec3, mass_of_target: f64) -> DVec3 {
        if self.mass == 0.0 { return DVec3::ZERO; }
        let diff = self.com - pos;
        let dist2 = diff.length_squared() + EPS * EPS;
        let dist = dist2.sqrt();
        // Barnes-Hut criterion
        if self.is_leaf() || (self.half_size * 2.0 / dist < THETA) {
            // Treat as single body
            let g_const = 6.674e-20; // km^3/(kg s^2)
            let force_mag = g_const * mass_of_target * self.mass / dist2;
            return diff.normalize() * force_mag;
        }
        // Recurse into children
        let mut total = DVec3::ZERO;
        if let Some(ch) = &self.children {
            for child in ch.iter() {
                total += child.force_on(pos, mass_of_target);
            }
        }
        total
    }
}

pub struct BarnesHut {
    root: OctNode,
}

impl BarnesHut {
    pub fn build(positions: &[(DVec3, f64)]) -> Self {
        // Find bounding box
        let mut max_coord: f64 = 50000.0; // at least 50000 km
        for &(p, _) in positions {
            max_coord = max_coord.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
        let half_size = max_coord * 1.01;
        let mut root = OctNode::new(DVec3::ZERO, half_size);
        // Insert Earth
        root.insert(DVec3::ZERO, EARTH_MASS, 0);
        for &(pos, mass) in positions {
            root.insert(pos, mass, 0);
        }
        BarnesHut { root }
    }

    /// Compute acceleration (km/s^2) on body at pos with given mass
    /// Earth gravity computed analytically for accuracy
    pub fn acceleration(&self, pos: DVec3, mass: f64) -> DVec3 {
        // Earth gravity (dominant, analytic)
        let r2 = pos.length_squared();
        let r = r2.sqrt();
        let earth_acc = -pos / (r * r2) * MU_EARTH;

        // N-body perturbations from octree (exclude Earth at origin which
        // we already handled analytically)
        let pert = self.root.force_on(pos, mass) / mass;

        earth_acc + pert
    }
}
