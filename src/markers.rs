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
