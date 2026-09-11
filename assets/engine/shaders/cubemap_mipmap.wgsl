@group(0) @binding(0)
var source_cubemap: texture_cube<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

struct FaceUniform {
    value: vec4<u32>,
};

@group(1) @binding(0)
var<uniform> face_uniform: FaceUniform;

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

fn cubemap_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let right = uv.x * 2.0 - 1.0;
    let up = 1.0 - uv.y * 2.0;

    switch face {
        case 0u: { return vec3(1.0, up, -right); }
        case 1u: { return vec3(-1.0, up, right); }
        case 2u: { return vec3(right, 1.0, -up); }
        case 3u: { return vec3(right, -1.0, up); }
        case 4u: { return vec3(right, up, 1.0); }
        default: { return vec3(-right, up, -1.0); }
    }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(
        source_cubemap,
        source_sampler,
        normalize(cubemap_direction(face_uniform.value.x, input.uv)),
        0.0,
    );
}
