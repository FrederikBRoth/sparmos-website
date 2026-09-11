struct CameraUniform {
    view_position: vec4<f32>,
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var skybox: texture_cube<f32>;

@group(1) @binding(1)
var skybox_sampler: sampler;

struct TextureParameters {
    values: vec4<f32>,
};

@group(1) @binding(2)
var<uniform> texture_parameters: TextureParameters;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) direction: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
) -> VertexOutput {

    var output: VertexOutput;

    // The cube is centered on the camera.
    // Remove camera translation from the view matrix.
    let view_rotation = mat4x4<f32>(
        camera.view[0],
        camera.view[1],
        camera.view[2],
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    let clip_position = camera.proj *
        view_rotation *
        vec4<f32>(position, 1.0);

    // Make the skybox always have depth = 1.0.
    output.position = vec4<f32>(
        clip_position.xy,
        clip_position.w,
        clip_position.w
    );

    // This is the cubemap lookup direction.
    output.direction = position;

    return output;
}

@fragment
fn fs_main(
    input: VertexOutput,
) -> @location(0) vec4<f32> {

    var envColor = textureSample(
        skybox,
        skybox_sampler,
        normalize(input.direction)
    ).rgb * texture_parameters.values.x;

    envColor = envColor / (envColor + vec3(1.0));
    return vec4(envColor, 1.0);
}
