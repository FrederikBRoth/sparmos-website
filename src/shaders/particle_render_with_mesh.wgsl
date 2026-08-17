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

struct ComputeArea {
    global_pos: vec3<f32>,
    rotation: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> compute_area: ComputeArea;

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
};
@group(3) @binding(0)
var<storage, read> particles: array<Particle>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {

    let particle = particles[instance_index];

    let local_pos = model.position + particle.position.xyz;
    let rotated_pos = quat_rotate(compute_area.rotation, local_pos);

    let world_pos = rotated_pos + vec3<f32>(compute_area.global_pos);
    // normal matrix = rotation only
    var out: VertexOutput;
    out.color = model.color;
    out.world_normal = model.normal;
    out.world_position = world_pos.xyz;
    out.clip_position = camera.view_proj * vec4<f32>(
        world_pos,
        1.0
    );
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_strength = 0.05;
    let specular_strength = 0.2;
    let shininess = 32.0;

    let N = normalize(in.world_normal);
    let V = normalize(camera.view_pos.xyz - in.world_position);

    var result: vec3<f32> = vec3<f32>(0.0);

    for (var i: u32 = 0u; i < u_lights.light_count; i = i + 1u) {
        let light = u_lights.lights[i];

        let L = normalize(light.position.xyz - in.world_position);
        let H = normalize(V + L);

        // Ambient
        let ambient = ambient_strength * in.color * light.color.xyz;

        // Diffuse
        let diff = max(dot(N, L), 0.0);
        let diffuse = diff * in.color * light.color.xyz;

        // Specular
        let spec = pow(max(dot(N, H), 0.0), shininess);
        let specular = specular_strength * spec * light.color.xyz;

        result += ambient + diffuse + specular;
    }

    // Optional tone mapping
    result = result / (result + vec3<f32>(1.0));

    return vec4<f32>(result, 1.0);
}

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let q_xyz = q.xyz;
    let t = 2.0 * cross(q_xyz, v);

    return v + q.w * t + cross(q_xyz, t);
}
