// src/renderer/earth.rs
// Sphere geometry generation with UV mapping for textures

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct EarthVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct EarthMesh {
    pub vertices: Vec<EarthVertex>,
    pub indices: Vec<u32>,
}

impl EarthMesh {
    pub fn generate(lat_steps: u32, lon_steps: u32, radius: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for lat in 0..=lat_steps {
            let theta = lat as f32 * std::f32::consts::PI / lat_steps as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=lon_steps {
                let phi = lon as f32 * 2.0 * std::f32::consts::PI / lon_steps as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = cos_phi * sin_theta;
                let y = cos_theta;
                let z = sin_phi * sin_theta;

                let u = 1.0 - (lon as f32 / lon_steps as f32);
                let v = lat as f32 / lat_steps as f32;

                vertices.push(EarthVertex {
                    pos: [x * radius, y * radius, z * radius],
                    normal: [x, y, z],
                    uv: [u, v],
                });
            }
        }

        for lat in 0..lat_steps {
            for lon in 0..lon_steps {
                let first = lat * (lon_steps + 1) + lon;
                let second = first + lon_steps + 1;

                indices.push(first);
                indices.push(second);
                indices.push(first + 1);

                indices.push(second);
                indices.push(second + 1);
                indices.push(first + 1);
            }
        }

        EarthMesh { vertices, indices }
    }
}
