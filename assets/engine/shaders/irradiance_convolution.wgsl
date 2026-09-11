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

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.local_position);
    let helper_axis = select(
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        abs(normal.y) > 0.999,
    );
    let right = normalize(cross(helper_axis, normal));
    let local_up = normalize(cross(normal, right));

    let pi = 3.14159265359;
    let sample_delta = 0.025;
    var irradiance = vec3(0.0);
    var sample_count = 0.0;
    var phi = 0.0;

    loop {
        if phi >= 2.0 * pi {
            break;
        }

        var theta = 0.0;
        loop {
            if theta >= 0.5 * pi {
                break;
            }

            let tangent_sample = vec3(
                sin(theta) * cos(phi),
                sin(theta) * sin(phi),
                cos(theta),
            );
            let sample_direction =
                tangent_sample.x * right +
                tangent_sample.y * local_up +
                tangent_sample.z * normal;

            irradiance += textureSample(
                environment_map,
                environment_sampler,
                sample_direction,
            ).rgb * cos(theta) * sin(theta);
            sample_count += 1.0;
            theta += sample_delta;
        }

        phi += sample_delta;
    }

    irradiance = min(pi * irradiance / sample_count, vec3(65504.0));
    return vec4(irradiance, 1.0);
}
