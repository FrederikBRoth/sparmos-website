struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec3<f32>, // xyz + padding
    color: vec3<f32>,  // rgb + padding
};

struct LightBlock {
    lights: array<Light, 16>,
    light_count: u32,
};

@group(1) @binding(0)
var<uniform> u_lights: LightBlock;

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
};
@group(2) @binding(0)
var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {

    let particle = particles[instance_index];

    var quad = array<vec2<f32>, 6>(
        vec2<f32>(-0.02, -0.02),
        vec2<f32>(0.02, -0.02),
        vec2<f32>(0.02, 0.02),
        vec2<f32>(-0.02, -0.02),
        vec2<f32>(0.02, 0.02),
        vec2<f32>(-0.02, 0.02),
    );

    let offset = quad[vertex_index];

    var output: VertexOutput;

    output.position = camera.view_proj * vec4<f32>(
        particle.position.xyz + vec3<f32>(offset, 0.0),
        1.0
    );

    return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.5, 0.1, 1.0);
}

