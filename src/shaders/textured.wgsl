struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

struct LightBlock {
    lights: array<Light, 16>,
    light_count: u32,
}

@group(1) @binding(0)
var<uniform> u_lights: LightBlock;

@group(2) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(2) @binding(1)
var diffuse_sampler: sampler;

@group(3) @binding(0)
var<storage, read> particles: array<u32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct InstanceInput {
    @location(5) pos_scale: vec4<f32>,
    @location(6) rotation: vec4<f32>,
    @location(7) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) uv: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let position = instance.pos_scale.xyz;
    let scale = instance.pos_scale.w;

    let rot = quat_to_mat3(instance.rotation);

    // Apply scale
    let rot_scaled = mat3x3<f32>(
        rot[0] * scale,
        rot[1] * scale,
        rot[2] * scale,
    );

    // Build full model matrix
    let model_matrix = mat4x4<f32>(
        vec4<f32>(rot_scaled[0], 0.0),
        vec4<f32>(rot_scaled[1], 0.0),
        vec4<f32>(rot_scaled[2], 0.0),
        vec4<f32>(position, 1.0),
    );

    let world_pos = model_matrix * vec4<f32>(model.position, 1.0);

    // Normal matrix = rotation only
    let normal = normalize(rot * model.normal);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_pos;
    out.color = instance.color;
    out.world_normal = normal;
    out.world_position = world_pos.xyz;
    out.uv = model.texture;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(
        diffuse_texture,
        diffuse_sampler,
        in.uv,
    ).rgb;

    // Texture is tinted by instance color
    let albedo = texture_color * in.color;

    let ambient_strength = 0.05;
    let specular_strength = 0.2;
    let shininess = f32(particles[0]);

    let N = normalize(in.world_normal);
    let V = normalize(camera.view_pos.xyz - in.world_position);

    var result: vec3<f32> = vec3<f32>(0.0);

    for (var i: u32 = 0u; i < u_lights.light_count; i = i + 1u) {
        let light = u_lights.lights[i];

        let L = normalize(light.position - in.world_position);
        let H = normalize(V + L);

        // Ambient
        let ambient = ambient_strength
            * albedo
            * light.color;

        // Diffuse
        let diff = max(dot(N, L), 0.0);
        let diffuse = diff
            * albedo
            * light.color;

        // Specular
        let spec = pow(
            max(dot(N, H), 0.0),
            shininess,
        );

        let specular = specular_strength
            * spec
            * light.color;

        result += ambient + diffuse + specular;
    }

    // Tone mapping
    result = result / (result + vec3<f32>(1.0));

    return vec4<f32>(result, 1.0);
}

fn quat_to_mat3(q: vec4<f32>) -> mat3x3<f32> {
    let x = q.x;
    let y = q.y;
    let z = q.z;
    let w = q.w;

    return mat3x3<f32>(
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y + z * w),
        2.0 * (x * z - y * w),
        2.0 * (x * y - z * w),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z + x * w),
        2.0 * (x * z + y * w),
        2.0 * (y * z - x * w),
        1.0 - 2.0 * (x * x + y * y),
    );
}
