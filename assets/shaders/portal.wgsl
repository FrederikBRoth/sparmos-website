struct CameraUniform {
    view_pos: vec4<f32>,
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
    screen: vec4<f32>,
    viewport: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var portal_texture: texture_2d<f32>;
@group(1) @binding(1) var portal_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
}
struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) scale: vec3<f32>,
    @location(7) rotation: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let p = vertex.position * instance.scale;
    let q = instance.rotation;
    let rotated = p + 2.0 * cross(q.xyz, cross(q.xyz, p) + q.w * p);
    return camera.proj * camera.view * vec4<f32>(instance.position + rotated, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // Fragment coordinates and render textures both start at the top left in WGPU.
    // Use the actual scene viewport, including post-process overscan when active.
    let uv = position.xy / camera.viewport.xy;
    return vec4<f32>(textureSample(portal_texture, portal_sampler, uv).rgb, 1.0);
}
