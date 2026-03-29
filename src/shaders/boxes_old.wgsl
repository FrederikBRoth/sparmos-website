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
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) instance_color: vec3<f32>,
    @location(10) normal_matrix_0: vec3<f32>,
    @location(11) normal_matrix_1: vec3<f32>,
    @location(12) normal_matrix_2: vec3<f32>,
}

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
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.color = vec3<f32>(1.0, 1.0, 1.0);
    out.world_normal = normalize(normal_matrix * model.normal); 

    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_strength = 0.1;
    let shininess = 32.0;


    var result: vec3<f32> = vec3<f32>(0.0);

    let N = normalize(in.world_normal);
    let V = normalize(camera.view_pos.xyz - in.world_position);

    for (var i: u32 = 0u; i < u_lights.light_count; i = i + 1u) {
        let light = u_lights.lights[i];

        let L = normalize(light.position.xyz - in.world_position);
        let H = normalize(V + L);

        // Ambient
        let ambient = light.color.xyz * ambient_strength;

        // Diffuse
        let diff = max(dot(N, L), 0.0);
        let diffuse = diff * light.color.xyz;

        // Specular (Blinn–Phong)
        let spec = pow(max(dot(N, H), 0.0), shininess);
        let specular = spec * light.color.xyz;

        result += ambient + diffuse + specular;
    }

    
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
