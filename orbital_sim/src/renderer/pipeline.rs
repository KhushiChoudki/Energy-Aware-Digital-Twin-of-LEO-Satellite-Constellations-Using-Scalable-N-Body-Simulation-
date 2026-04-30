// src/renderer/pipeline.rs
// Shaders and render pipelines with ultra-high visibility for Kessler Syndrome demo

use crate::renderer::gpu_state::GpuState;

pub struct Pipelines {
    pub earth: wgpu::RenderPipeline,
    pub bodies: wgpu::RenderPipeline,
    pub trails: wgpu::RenderPipeline,
}

impl Pipelines {
    pub fn new(gpu: &GpuState) -> Self {
        let device = &gpu.device;

        let earth_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("earth_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(EARTH_WGSL)),
        });

        let earth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("earth_layout"),
            bind_group_layouts: &[&gpu.uniform_bgl(), &gpu.earth_tex_bgl()],
            push_constant_ranges: &[],
        });

        let earth = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("earth_pipeline"),
            layout: Some(&earth_layout),
            vertex: wgpu::VertexState {
                module: &earth_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &earth_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.format(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let body_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("body_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(BODY_WGSL)),
        });

        let body_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("body_layout"),
            bind_group_layouts: &[&gpu.uniform_bgl()],
            push_constant_ranges: &[],
        });

        let bodies = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("body_pipeline"),
            layout: Some(&body_layout),
            vertex: wgpu::VertexState {
                module: &body_shader,
                entry_point: "vs_body",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 48,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x4, 3 => Float32],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &body_shader,
                entry_point: "fs_body",
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let trail_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trail_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(TRAIL_WGSL)),
        });

        let trails = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("trail_pipeline"),
            layout: Some(&body_layout),
            vertex: wgpu::VertexState {
                module: &trail_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 28,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &trail_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::LineList, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Pipelines { earth, bodies, trails }
    }
}

const EARTH_WGSL: &str = "
struct Uniforms { view_proj: mat4x4<f32>, time: f32, flash: f32, aspect: f32, _pad: f32 };
@group(0) @binding(0) var<uniform> uni: Uniforms;
@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

struct VIn { @location(0) pos: vec3<f32>, @location(1) norm: vec3<f32>, @location(2) uv: vec2<f32> };
struct VOut { @builtin(position) clip_pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) norm: vec3<f32> };

@vertex fn vs_main(v: VIn) -> VOut {
    let omega_e = 7.292115e-5;
    let angle = omega_e * uni.time;
    let c = cos(angle); let s = sin(angle);
    let rot_pos = vec3<f32>(c * v.pos.x - s * v.pos.z, v.pos.y, s * v.pos.x + c * v.pos.z);
    let rot_norm = vec3<f32>(c * v.norm.x - s * v.norm.z, v.norm.y, s * v.norm.x + c * v.norm.z);
    var out: VOut;
    out.clip_pos = uni.view_proj * vec4<f32>(rot_pos, 1.0);
    out.uv = v.uv; out.norm = rot_norm;
    return out;
}

@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, v.uv);
    let light_dir = normalize(vec3<f32>(1.5, 0.5, 1.0));
    let diff = max(dot(v.norm, light_dir), 0.05); 
    let is_water = select(0.0, 1.0, color.b > color.r * 1.1 && color.g < 0.7);
    let specular = pow(diff, 64.0) * is_water * 0.8;
    return vec4<f32>(color.rgb * diff + specular + vec3<f32>(uni.flash * 0.4), 1.0);
}
";

const BODY_WGSL: &str = "
struct Uniforms { view_proj: mat4x4<f32>, time: f32, flash: f32, aspect: f32, _pad: f32 };
@group(0) @binding(0) var<uniform> uni: Uniforms;

struct VIn { @builtin(vertex_index) vid: u32, @location(0) pos: vec3<f32>, @location(1) radius: f32, @location(2) color: vec4<f32>, @location(3) kind: f32 };
struct VOut { @builtin(position) clip_pos: vec4<f32>, @location(0) color: vec4<f32>, @location(1) uv: vec2<f32>, @location(2) kind: f32 };

@vertex fn vs_body(v: VIn) -> VOut {
    var uv: vec2<f32>;
    let local_idx = v.vid % 6u;
    switch (local_idx) {
        case 0u: { uv = vec2<f32>(-1.0, -1.0); }
        case 1u: { uv = vec2<f32>( 1.0, -1.0); }
        case 2u: { uv = vec2<f32>(-1.0,  1.0); }
        case 3u: { uv = vec2<f32>( 1.0, -1.0); }
        case 4u: { uv = vec2<f32>( 1.0,  1.0); }
        case 5u: { uv = vec2<f32>(-1.0,  1.0); }
        default: { uv = vec2<f32>(0.0, 0.0); }
    }
    let center_clip = uni.view_proj * vec4<f32>(v.pos, 1.0);
    // Increase size for ultra-high visibility
    let size = v.radius * 3.0; 
    let offset = uv * size;
    var out: VOut;
    out.clip_pos = center_clip + vec4<f32>(offset.x / uni.aspect, offset.y, 0.0, 0.0);
    out.color = v.color; out.uv = uv; out.kind = v.kind;
    return out;
}

fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123); }

@fragment fn fs_body(v: VOut) -> @location(0) vec4<f32> {
    let dist = length(v.uv);
    if (v.kind > 0.5) {
        // High-visibility debris (glowing rocks)
        let noise = hash(v.uv * 10.0 + floor(v.clip_pos.xy));
        if (dist > 0.9) { discard; }
        let glow = (1.0 - dist) * 1.5;
        return vec4<f32>(v.color.rgb * glow, v.color.a);
    } else {
        // High-visibility satellites (Glow + Center)
        if (dist > 1.0) { discard; }
        let glow = pow(1.0 - dist, 1.5) * 1.2;
        let ring = smoothstep(0.75, 0.85, dist) - smoothstep(0.95, 1.0, dist);
        let center = 1.0 - smoothstep(0.0, 0.3, dist);
        let alpha = max(max(glow, ring), center) * v.color.a;
        return vec4<f32>(v.color.rgb * 1.2, alpha);
    }
}
";

const TRAIL_WGSL: &str = "
struct Uniforms { view_proj: mat4x4<f32>, time: f32, flash: f32, aspect: f32, _pad: f32 };
@group(0) @binding(0) var<uniform> uni: Uniforms;
struct VIn { @location(0) pos: vec3<f32>, @location(1) color: vec4<f32> };
struct VOut { @builtin(position) clip_pos: vec4<f32>, @location(0) color: vec4<f32> };

@vertex fn vs_main(v: VIn) -> VOut {
    var out: VOut;
    out.clip_pos = uni.view_proj * vec4<f32>(v.pos, 1.0);
    out.color = v.color;
    return out;
}

@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(v.color.rgb, v.color.a * 1.5); // Brighter trails
}
";
