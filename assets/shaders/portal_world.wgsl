struct CameraUniform {
    view_pos: vec4<f32>,
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(2) normal: vec3<f32>,
}
struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) scale: vec3<f32>,
    @location(7) rotation: vec4<f32>,
    @location(8) color: vec3<f32>,
}
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}
fn rotate(q: vec4<f32>, p: vec3<f32>) -> vec3<f32> {
    return p + 2.0 * cross(q.xyz, cross(q.xyz, p) + q.w * p);
}
@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let position = instance.position + rotate(instance.rotation, vertex.position * instance.scale);
    out.position = camera.proj * camera.view * vec4<f32>(position, 1.0);
    let normal = normalize(rotate(instance.rotation, vertex.normal / instance.scale));
    let light = 0.4 + 0.6 * max(dot(normal, normalize(vec3<f32>(0.3, 0.8, -0.5))), 0.0);
    out.color = instance.color * light;
    return out;
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
