// Vertex shader

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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}
struct InstanceInput {
    @location(5) pos_scale: vec4<f32>,
    @location(6) rotation: vec4<f32>,
    @location(7) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1)  world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let position = instance.pos_scale.xyz;
    let scale = instance.pos_scale.w;

    let rot = quat_to_mat3(instance.rotation);

    // apply scale
    let rot_scaled = mat3x3<f32>(
        rot[0] * scale,
        rot[1] * scale,
        rot[2] * scale,
    );

    // build full model matrix
    let model_matrix = mat4x4<f32>(
        vec4<f32>(rot_scaled[0], 0.0),
        vec4<f32>(rot_scaled[1], 0.0),
        vec4<f32>(rot_scaled[2], 0.0),
        vec4<f32>(position, 1.0),
    );

    let world_pos = model_matrix * vec4<f32>(model.position, 1.0);

    // normal matrix = rotation only
    let normal = normalize(rot * model.normal);
    var out: VertexOutput;
    out.color = instance.color;
    out.world_normal = normal;
    out.world_position = world_pos.xyz;
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

// Fragment shader
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

fn quat_to_mat3(q: vec4<f32>) -> mat3x3<f32> {
    let x = q.x;
    let y = q.y;
    let z = q.z;
    let w = q.w;

    return mat3x3<f32>(
        1.0 - 2.0 * (y*y + z*z),
        2.0 * (x*y + z*w),
        2.0 * (x*z - y*w),

        2.0 * (x*y - z*w),
        1.0 - 2.0 * (x*x + z*z),
        2.0 * (y*z + x*w),

        2.0 * (x*z + y*w),
        2.0 * (y*z - x*w),
        1.0 - 2.0 * (x*x + y*y)
    );
}
