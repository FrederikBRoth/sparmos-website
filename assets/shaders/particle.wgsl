
struct Bounds {
    bounds: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> bounds: Bounds;

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
};

@group(1) @binding(0)
var<storage, read_write> particles: array<Particle>;

@compute
@workgroup_size(64)
fn main(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    let i = id.x;

    if i >= arrayLength(&particles) {
        return;
    }

    var p = particles[i];

    let dt = 0.016;

    // Gravity
    //p.velocity.y -= 9.81 * dt;

    // Movement
    let new_position = p.position.xyz + p.velocity.xyz * dt;

    p.position = vec4<f32>(
        new_position,
        p.position.w
    );

    // Floor
    if p.position.y < -bounds.bounds.y {
        p.position.y = -bounds.bounds.y;
        p.velocity.y = abs(p.velocity.y) * 1.1;
    }

    particles[i] = p;

    if p.position.y > bounds.bounds.y {
        p.velocity.y *= -1.0;
        p.position.y = clamp(
            p.position.y,
            -bounds.bounds.y,
            bounds.bounds.y
        );
    }
    // X boundaries
    if abs(p.position.x) > bounds.bounds.x {
        p.velocity.x *= -1.0;
        p.position.x = clamp(
            p.position.x,
            -bounds.bounds.x,
            bounds.bounds.x
        );
    }

    // Z boundaries
    if abs(p.position.z) > bounds.bounds.z {
        p.velocity.z *= -1.0;
        p.position.z = clamp(
            p.position.z,
            -bounds.bounds.z,
            bounds.bounds.z
        );
    }

    particles[i] = p;
}
