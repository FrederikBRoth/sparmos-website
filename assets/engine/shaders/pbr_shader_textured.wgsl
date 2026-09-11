const PI: f32 = 3.141592653589793;
struct CameraUniform {
    view_pos: vec4<f32>,
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

struct LightBlock {
    lights: array<Light, 16>,
    light_count: u32,
}

@group(0) @binding(1)
var<uniform> u_lights: LightBlock;

@group(1) @binding(0)
var albedo_map: texture_2d<f32>;

@group(1) @binding(1)
var normal_map: texture_2d<f32>;

@group(1) @binding(2)
var metallic_map: texture_2d<f32>;
@group(1) @binding(3)
var roughness_map: texture_2d<f32>;
@group(1) @binding(4)
var ao_map: texture_2d<f32>;

@group(1) @binding(5)
var texture_sampler: sampler;

@group(2) @binding(0)
var irradiance_map: texture_cube<f32>;

@group(2) @binding(1)
var prefiltered_environment_map: texture_cube<f32>;

@group(2) @binding(2)
var brdf_lut: texture_2d<f32>;

@group(2) @binding(3)
var ibl_sampler: sampler;

struct IblTextureParameters {
    values: vec4<f32>,
};

@group(2) @binding(4)
var<uniform> ibl_texture_parameters: IblTextureParameters;
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
    let position = instance.position;
    let scale = instance.scale;
    let scale_sign = select(
        vec3<f32>(-1.0),
        vec3<f32>(1.0),
        scale >= vec3<f32>(0.0),
    );
    let safe_scale = scale_sign * max(abs(scale), vec3<f32>(0.000001));

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

    let normal = normalize(rot * (model.normal / safe_scale));

    var out: VertexOutput;
    let view_proj = camera.proj * camera.view;
    out.clip_position = view_proj * world_pos;
    out.color = instance.color;
    out.world_normal = normal;
    out.world_position = world_pos.xyz;
    out.uv = model.texture;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let albedo = textureSample(albedo_map, texture_sampler, in.uv).rgb;
    let metallic = textureSample(
        metallic_map,
        texture_sampler,
        in.uv
    ).b;

    let roughness = clamp(textureSample(
        roughness_map,
        texture_sampler,
        in.uv
    ).g, 0.04, 1.0);

    let ao = textureSample(
        ao_map,
        texture_sampler,
        in.uv
    ).r;

    var N = get_normal_from_normal_map(
        in.uv,
        in.world_position,
        in.world_normal,
    );

    //N = normalize(in.world_normal);
    let V = normalize(camera.view_pos.xyz - in.world_position);

    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);
    var lo: vec3<f32> = vec3<f32>(0.0);

    for (var i: u32 = 0u; i < u_lights.light_count; i = i + 1u) {
        let light = u_lights.lights[i];

        let L = normalize(light.position - in.world_position);
        let H = normalize(V + L);

        let ndotl = max(dot(N, L), 0.0);
        let distance = length(light.position - in.world_position);
        let attenuation = 1.0 / (distance * distance);
        let radiance = light.color * light.intensity * attenuation;

        //full Cook-Torrance specular calculation
        //Surface reflection if looking directly at the material. This is different per material (think metals, plastic etc)
        //Should be changed based on that. we go for a constant
        let f = fresnel_schlick(max(dot(H, V), 0.0), f0);

        let ndf = distribution_ggx(N, H, roughness);
        let g = geometry_smith(N, V, L, roughness);

        let numerator = ndf * g * f;
        let denominator = 4.0 * max(dot(N, V), 0.0) * ndotl + 0.0001;
        let specular = numerator / denominator;

        let kS = f;
        var kD = vec3<f32>(1.0) - kS;
        kD = kD * (1.0 - metallic);

        lo += (kD * albedo / PI + specular) * radiance * ndotl;
    }

    let ks = fresnel_schlick_roughness(max(dot(N, V), 0.0), f0, roughness);
    let kd = (1.0 - ks) * (1.0 - metallic);
    let irradiance = textureSample(irradiance_map, ibl_sampler, N).rgb *
        ibl_texture_parameters.values.x;
    let diffuse = irradiance * albedo;
    let reflection = reflect(-V, N);
    let max_reflection_lod = f32(textureNumLevels(prefiltered_environment_map) - 1u);
    let prefiltered_color = textureSampleLevel(
        prefiltered_environment_map,
        ibl_sampler,
        reflection,
        roughness * max_reflection_lod,
    ).rgb * ibl_texture_parameters.values.x;
    let environment_brdf = textureSample(
        brdf_lut,
        ibl_sampler,
        vec2(max(dot(N, V), 0.0), roughness),
    ).rg;
    let specular_ibl = prefiltered_color * (ks * environment_brdf.x + environment_brdf.y);
    let ambient = (kd * diffuse + specular_ibl) * ao;
    // Tone mapping
    var color = ambient + lo;

    color = color / (color + vec3<f32>(1.0));
    //linear to srgb format. Not required as wgpu does that automatically
    //color = pow(color, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
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

//Cook-Torrance BRDF(bidirectional reflective distribution function) 
//functions. BRDF scales incomming radiance based on the surfaces material proper
//ties
//
//Normal distribution function, D
fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let ndoth = max(dot(n, h), 0.0);
    let ndoth2 = ndoth * ndoth;
    let nom = a2;
    var denom = (ndoth2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return nom / max(denom, 0.000001);
}

//Geometry function, G. Roughness essentially
fn geometry_schlick_ggx(ndotv: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    let nom = ndotv;
    let denom = ndotv * (1.0 - k) + k;

    return nom / max(denom, 0.000001);
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
    let ndotv = max(dot(n, v), 0.0);
    let ndotl = max(dot(n, l), 0.0);
    let ggx1 = geometry_schlick_ggx(ndotv, roughness);
    let ggx2 = geometry_schlick_ggx(ndotl, roughness);

    return ggx1 * ggx2;
}

//Fresnell, Fr. Essentially calculating the angly reflection for when you are looking 
//at material straight at it (90 degrees) or parallel

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

fn fresnel_schlick_roughness(cos_theta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}
fn get_normal_from_normal_map(
    uv: vec2<f32>,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
) -> vec3<f32> {
    let tangent_normal = textureSample(
        normal_map,
        texture_sampler,
        uv
    ).xyz * 2.0 - 1.0;

    let Q1 = dpdx(world_position);
    let Q2 = dpdy(world_position);

    let st1 = dpdx(uv);
    let st2 = dpdy(uv);

    let N = normalize(world_normal);

    let T = normalize(Q1 * st2.y - Q2 * st1.y);
    let B = -normalize(cross(N, T));

    let TBN = mat3x3<f32>(
        T,
        B,
        N
    );

    return normalize(TBN * tangent_normal);
}
