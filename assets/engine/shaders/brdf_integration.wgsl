const PI: f32 = 3.14159265359;
const SAMPLE_COUNT: u32 = 1024u;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array(
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4(position, 0.0, 1.0);
    output.uv = position * vec2(0.5, -0.5) + vec2(0.5);
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

fn importance_sample_ggx(xi: vec2<f32>, normal: vec3<f32>, roughness: f32) -> vec3<f32> {
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

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let k = roughness * roughness * 0.5;
    return n_dot_v / max(n_dot_v * (1.0 - k) + k, 0.0001);
}

fn geometry_smith(
    normal: vec3<f32>,
    view: vec3<f32>,
    light: vec3<f32>,
    roughness: f32,
) -> f32 {
    let n_dot_v = max(dot(normal, view), 0.0);
    let n_dot_l = max(dot(normal, light), 0.0);
    return geometry_schlick_ggx(n_dot_v, roughness) *
        geometry_schlick_ggx(n_dot_l, roughness);
}

fn integrate_brdf(n_dot_v: f32, roughness: f32) -> vec2<f32> {
    let view = vec3(sqrt(max(1.0 - n_dot_v * n_dot_v, 0.0)), 0.0, n_dot_v);
    let normal = vec3(0.0, 0.0, 1.0);
    var scale = 0.0;
    var bias = 0.0;

    for (var i = 0u; i < SAMPLE_COUNT; i += 1u) {
        let half_vector = importance_sample_ggx(hammersley(i), normal, roughness);
        let light = normalize(2.0 * dot(view, half_vector) * half_vector - view);
        let n_dot_l = max(light.z, 0.0);
        let n_dot_h = max(half_vector.z, 0.0);
        let v_dot_h = max(dot(view, half_vector), 0.0);

        if n_dot_l > 0.0 {
            let geometry = geometry_smith(normal, view, light, roughness);
            let geometry_visibility = geometry * v_dot_h /
                max(n_dot_h * n_dot_v, 0.0001);
            let fresnel = pow(1.0 - v_dot_h, 5.0);
            scale += (1.0 - fresnel) * geometry_visibility;
            bias += fresnel * geometry_visibility;
        }
    }

    return vec2(scale, bias) / f32(SAMPLE_COUNT);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec2<f32> {
    return integrate_brdf(clamp(input.uv.x, 0.0, 1.0), clamp(input.uv.y, 0.0, 1.0));
}
