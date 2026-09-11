const PI: f32 = 3.14159265359;
const SAMPLE_COUNT: u32 = 1024u;

struct CaptureCamera {
    view_proj: mat4x4<f32>,
    roughness: f32,
    source_resolution: f32,
    padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> capture_camera: CaptureCamera;

@group(1) @binding(0)
var environment_map: texture_cube<f32>;

@group(1) @binding(1)
var environment_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_position: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = capture_camera.view_proj * vec4(position, 1.0);
    output.local_position = position;
    return output;
}

fn radical_inverse_vdc(value: u32) -> f32 {
    var bits = value;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(index: u32) -> vec2<f32> {
    return vec2(f32(index) / f32(SAMPLE_COUNT), radical_inverse_vdc(index));
}

fn distribution_ggx(normal: vec3<f32>, half_vector: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denominator = n_dot_h2 * (a2 - 1.0) + 1.0;
    return a2 / max(PI * denominator * denominator, 0.000001);
}

fn importance_sample_ggx(
    xi: vec2<f32>,
    normal: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let tangent_half = vec3(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);

    let helper_axis = select(
        vec3(0.0, 0.0, 1.0),
        vec3(1.0, 0.0, 0.0),
        abs(normal.z) >= 0.999,
    );
    let tangent = normalize(cross(helper_axis, normal));
    let bitangent = cross(normal, tangent);
    return normalize(
        tangent * tangent_half.x +
        bitangent * tangent_half.y +
        normal * tangent_half.z
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.local_position);
    let view = normal;
    var prefiltered_color = vec3(0.0);
    var total_weight = 0.0;

    for (var i = 0u; i < SAMPLE_COUNT; i += 1u) {
        let xi = hammersley(i);
        let half_vector = importance_sample_ggx(xi, normal, capture_camera.roughness);
        let light = normalize(2.0 * dot(view, half_vector) * half_vector - view);
        let n_dot_l = max(dot(normal, light), 0.0);

        if n_dot_l > 0.0 {
            let n_dot_h = max(dot(normal, half_vector), 0.0);
            let h_dot_v = max(dot(half_vector, view), 0.0);
            let distribution = distribution_ggx(
                normal,
                half_vector,
                capture_camera.roughness,
            );
            let pdf = distribution * n_dot_h / max(4.0 * h_dot_v, 0.0001) + 0.0001;
            let texel_solid_angle = 4.0 * PI /
                (6.0 * capture_camera.source_resolution * capture_camera.source_resolution);
            let sample_solid_angle = 1.0 / (f32(SAMPLE_COUNT) * pdf + 0.0001);
            var source_mip = 0.5 * log2(sample_solid_angle / texel_solid_angle);
            if capture_camera.roughness == 0.0 {
                source_mip = 0.0;
            }

            prefiltered_color += textureSampleLevel(
                environment_map,
                environment_sampler,
                light,
                max(source_mip, 0.0),
            ).rgb * n_dot_l;
            total_weight += n_dot_l;
        }
    }

    let filtered = min(
        prefiltered_color / max(total_weight, 0.0001),
        vec3(65504.0),
    );
    return vec4(filtered, 1.0);
}
