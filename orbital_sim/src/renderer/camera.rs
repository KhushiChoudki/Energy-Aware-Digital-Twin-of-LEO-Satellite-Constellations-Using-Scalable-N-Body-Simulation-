// src/renderer/camera.rs
// Arcball orbit camera with zoom and pan

use glam::{Mat4, Vec3, DVec3};

pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,   // radians
    pub pitch: f32, // radians
    pub fov: f32,   // radians
    pub aspect: f32,
    pub near: f32,
    pub far: f32,

    dragging: bool,
    last_mouse: Option<(f32, f32)>,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Camera {
            target: Vec3::ZERO,
            distance: 250.0, // GPU units (25,000 km at 0.01 scale)
            yaw: 0.8,
            pitch: 0.4,
            fov: 45_f32.to_radians(),
            aspect,
            near: 1.0,
            far: 2000.0,
            dragging: false,
            last_mouse: None,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        Vec3::new(x, y, z) + self.target
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    pub fn view_proj(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }

    pub fn on_mouse_press(&mut self, button: u32) {
        if button == 0 { self.dragging = true; }
    }

    pub fn on_mouse_release(&mut self, button: u32) {
        if button == 0 {
            self.dragging = false;
            self.last_mouse = None;
        }
    }

    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        if !self.dragging { return; }
        if let Some((lx, ly)) = self.last_mouse {
            let dx = x - lx;
            let dy = y - ly;
            self.yaw -= dx * 0.005;
            self.pitch = (self.pitch + dy * 0.005).clamp(-1.5, 1.5);
        }
        self.last_mouse = Some((x, y));
    }

    pub fn on_scroll(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 - delta * 0.1)).clamp(50.0, 1800.0);
    }

    pub fn focus_on(&mut self, pos: DVec3) {
        self.target = Vec3::new(pos.x as f32 * 0.01, pos.y as f32 * 0.01, pos.z as f32 * 0.01);
        self.distance = 50.0;
    }

    pub fn reset(&mut self) {
        self.target = Vec3::ZERO;
        self.distance = 250.0;
        self.yaw = 0.8;
        self.pitch = 0.4;
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.aspect = w as f32 / h as f32;
    }
}
