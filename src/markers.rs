use sparmos_engine::cgmath::Vector4;

#[derive(Debug)]
pub struct Boxes;
#[derive(Debug)]
pub struct Light;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Bounds {
    pub bounds: [f32; 3],
    pub _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ComputeArea {
    pub global_pos: [f32; 3],
    pub _padding: f32,
    pub rotation: [f32; 4],
}
