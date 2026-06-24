// src/renderer/gpu_state.rs
// wgpu 0.20 compatible GPU state with optimized vertex buffers and trail logic

use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use crate::renderer::earth::EarthMesh;
use crate::simulation::body::{Body, BodyType};
use crate::simulation::state::{SimState, MAX_DEBRIS};

pub const SCALE: f32 = 1.0 / 100.0; 

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub time: f32,
    pub flash: f32,
    pub aspect: f32,
    pub camera_dist: f32, 
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BodyInstance {
    pub pos: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub kind: f32, 
    pub _pad: [f32; 3],
}

pub struct GpuState<'a> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub depth_view: wgpu::TextureView,

    pub earth_vbuf: wgpu::Buffer,
    pub earth_ibuf: wgpu::Buffer,
    pub earth_index_count: u32,
    pub earth_tex_bg: wgpu::BindGroup,

    pub body_instance_buf: wgpu::Buffer,
    pub body_instance_count: u32,

    pub trail_buf: wgpu::Buffer,
    pub trail_vertex_count: u32,

    pub static_path_buf: wgpu::Buffer,
    pub static_path_count: u32,

    pub uniform_buf: wgpu::Buffer,
    pub uniform_bg: wgpu::BindGroup,
}

impl<'a> GpuState<'a> {
    pub async fn new(window: std::sync::Arc<winit::window::Window>, earth_tex_bytes: &[u8]) -> GpuState<'static> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let surface = unsafe { instance.create_surface(&*window) }.unwrap();
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }).await.expect("No GPU adapter");

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let depth_view = make_depth_view(&device, size.width.max(1), size.height.max(1));

        let (earth_tex_bg, _) = load_texture_bind_group(&device, &queue, earth_tex_bytes, "earth_tex");

        let earth = EarthMesh::generate(64, 128, 6371.0 * SCALE);
        let earth_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("earth_v"),
            contents: bytemuck::cast_slice(&earth.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let earth_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("earth_i"),
            contents: bytemuck::cast_slice(&earth.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Pre-allocate large buffers to avoid runtime resizing
        let body_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("body_inst"),
            size: (std::mem::size_of::<BodyInstance>() * (MAX_DEBRIS + 1000)) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let trail_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trail"),
            size: (std::mem::size_of::<[f32; 7]>() * 8_000_000) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let static_path_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("static_path"),
            size: (std::mem::size_of::<[f32; 7]>() * 50_000) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniforms = Uniforms { view_proj: Mat4::IDENTITY.to_cols_array_2d(), time: 0.0, flash: 0.0, aspect: 1.0, camera_dist: 250.0 };
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &make_uniform_bgl(&device),
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() }],
            label: Some("uniform_bg"),
        });

        GpuState {
            device, queue, surface, surface_config, depth_view,
            earth_vbuf, earth_ibuf, earth_index_count: earth.indices.len() as u32,
            earth_tex_bg,
            body_instance_buf, body_instance_count: 0,
            trail_buf, trail_vertex_count: 0,
            static_path_buf, static_path_count: 0,
            uniform_buf, uniform_bg,
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 { return; }
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_view = make_depth_view(&self.device, w, h);
    }

    pub fn format(&self) -> wgpu::TextureFormat { self.surface_config.format }

    pub fn update_uniforms(&self, vp: Mat4, time: f32, flash: f32, aspect: f32, camera_dist: f32) {
        let u = Uniforms { view_proj: vp.to_cols_array_2d(), time, flash, aspect, camera_dist };
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&[u]));
    }

    pub fn update_static_paths(&mut self, sim: &SimState) {
        let mut verts = Vec::with_capacity(10000);
        for path in &sim.orbital_paths {
            let color = path.body_type.default_color();
            for i in 1..path.points.len() {
                let p0 = path.points[i-1];
                let p1 = path.points[i];
                verts.extend_from_slice(&[p0.x as f32 * SCALE, p0.y as f32 * SCALE, p0.z as f32 * SCALE, color[0], color[1], color[2], 0.1]);
                verts.extend_from_slice(&[p1.x as f32 * SCALE, p1.y as f32 * SCALE, p1.z as f32 * SCALE, color[0], color[1], color[2], 0.1]);
            }
        }
        self.static_path_count = (verts.len() / 7) as u32;
        if !verts.is_empty() {
            self.queue.write_buffer(&self.static_path_buf, 0, bytemuck::cast_slice(&verts));
        }
    }

    pub fn update_bodies(&mut self, bodies: &[Body], show_debris: bool) {
        let instances: Vec<BodyInstance> = bodies.iter()
            .filter(|b| b.alive)
            .filter(|b| show_debris || matches!(b.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya))
            .take(MAX_DEBRIS + 5) // Safety cap
            .map(|b| BodyInstance {
                pos: [b.pos.x as f32 * SCALE, b.pos.y as f32 * SCALE, b.pos.z as f32 * SCALE],
                radius: b.body_type.visual_radius() * SCALE,
                color: b.effective_color(),
                kind: b.body_type.visual_kind(),
                _pad: [0.0; 3],
            })
            .collect();
        self.body_instance_count = instances.len() as u32;
        if !instances.is_empty() {
            self.queue.write_buffer(&self.body_instance_buf, 0, bytemuck::cast_slice(&instances));
        }
    }

    pub fn update_trails(&mut self, bodies: &[Body], show_debris: bool) {
        let mut verts: Vec<f32> = Vec::with_capacity(1_000_000);
        for body in bodies.iter()
            .filter(|b| b.alive)
            .filter(|b| show_debris || matches!(b.body_type, BodyType::LiveSatellite | BodyType::Russs | BodyType::Iridium33 | BodyType::Zarya))
            .take(MAX_DEBRIS + 5) {
            if body.trail.len() < 2 { continue; }
            let c = body.effective_color();
            let n = body.trail.len();
            for i in 1..n {
                let p0 = body.trail[i-1];
                let p1 = body.trail[i];
                let alpha = (i as f32 / n as f32) * 0.5;
                verts.extend_from_slice(&[p0[0]*SCALE, p0[1]*SCALE, p0[2]*SCALE, c[0], c[1], c[2], alpha]);
                verts.extend_from_slice(&[p1[0]*SCALE, p1[1]*SCALE, p1[2]*SCALE, c[0], c[1], c[2], alpha]);
                // Safety break to avoid buffer overflow
                if verts.len() > 7_900_000 { break; }
            }
            if verts.len() > 7_900_000 { break; }
        }
        self.trail_vertex_count = (verts.len() / 7) as u32;
        if !verts.is_empty() {
            self.queue.write_buffer(&self.trail_buf, 0, bytemuck::cast_slice(&verts));
        }
    }

    pub fn uniform_bgl(&self) -> wgpu::BindGroupLayout { make_uniform_bgl(&self.device) }
    pub fn earth_tex_bgl(&self) -> wgpu::BindGroupLayout { make_texture_bgl(&self.device) }
}

fn make_uniform_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    })
}

fn make_texture_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn load_texture_bind_group(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], label: &str) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
    let img = image::load_from_memory(bytes).unwrap().to_rgba8();
    let (width, height) = img.dimensions();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &img,
        wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 }
    );
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let bgl = make_texture_bgl(device);
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
        label: Some(label),
    });
    (bg, bgl)
}

fn make_depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }).create_view(&Default::default())
}
