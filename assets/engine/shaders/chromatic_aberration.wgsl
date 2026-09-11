// Supplied from POST_PROCESS_OVERSCAN when the pipeline is created.
override post_process_overscan: f32;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VSOut {
    var positions = array<vec2<f32>, 3>(
        vec2(-1.0, -3.0),
        vec2( 3.0,  1.0),
        vec2(-1.0,  1.0),
    );

    var out: VSOut;
    let pos = positions[i];
    out.pos = vec4(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + vec2(0.5);
    return out;
}

@group(0) @binding(0)
var screen_tex: texture_2d<f32>;

@group(0) @binding(1)
var screen_sampler: sampler;

fn safe_uv(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, vec2(0.001), vec2(0.999));
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
// Flip Y
    let screen_uv = vec2(in.uv.x, 1.0 - in.uv.y);

    let crop_scale = 1.0 / post_process_overscan;
    let offset_uv = (1.0 - crop_scale) * 0.5;

    let uv = screen_uv * crop_scale + offset_uv;

    // --- Chromatic aberration ---
    let center = vec2<f32>(0.5, 0.5);
    let dir = uv - center;

    let dist = length(dir);

    // normalize safely
    let dir_n = select(vec2(0.0), dir / dist, dist > 0.0);

    // nicer falloff
    let strength = dist * dist * 0.03;

    let offset = dir_n * strength;

    let uv_r = uv + offset;
    let uv_b = uv - offset;

    let r = textureSample(screen_tex, screen_sampler, uv_r).r;
    let g = textureSample(screen_tex, screen_sampler, uv).g;
    let b = textureSample(screen_tex, screen_sampler, uv_b).b;

    return vec4(r, g, b, 1.0);
}
