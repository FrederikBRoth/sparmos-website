struct CameraUniform {
    view_pos: vec4<f32>,
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(1) @binding(1)
var diffuse_sampler: sampler;

//@group(2) @binding(0)
//var<storage, read> particles: array<u32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) scale: vec3<f32>,
    @location(7) rotation: vec4<f32>,
    @location(8) color: vec3<f32>,
    @location(9) uv: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let position = instance.position;
    let scale = instance.scale;
    let rot = quat_to_mat3(instance.rotation);

    // Apply scale
    let rot_scaled = mat3x3<f32>(
        rot[0] * scale.x,
        rot[1] * scale.y,
        rot[2] * scale.z,
    );

    // Build full model matrix
    let model_matrix = mat4x4<f32>(
        vec4<f32>(rot_scaled[0], 0.0),
        vec4<f32>(rot_scaled[1], 0.0),
        vec4<f32>(rot_scaled[2], 0.0),
        vec4<f32>(position, 1.0),
    );

    let world_pos = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    let view_proj = camera.proj * camera.view;

    out.clip_position = view_proj * world_pos;
    out.color = instance.color;
    out.uv = instance.uv.xy + model.texture * instance.uv.zw;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(
        diffuse_texture,
        diffuse_sampler,
        in.uv,
    );

    if texture_color.a < 0.5 {
        discard;
    }

    return vec4<f32>(texture_color.rgb * in.color, texture_color.a);
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
