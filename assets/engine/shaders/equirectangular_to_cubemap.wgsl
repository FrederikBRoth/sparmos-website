struct CaptureCamera {
    view_proj: mat4x4<f32>,
    roughness: f32,
    source_resolution: f32,
    padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> capture_camera: CaptureCamera;

@group(1) @binding(0)
var hdri: texture_2d<f32>;

@group(1) @binding(1)
var hdri_sampler: sampler;

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

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let direction = normalize(input.local_position);
    var uv = vec2(
        atan2(direction.z, direction.x),
        asin(clamp(direction.y, -1.0, 1.0)),
    );
    uv *= vec2(0.15915494, 0.31830989);
    uv += vec2(0.5);

    return textureSample(hdri, hdri_sampler, uv);
}
