@group(0) @binding(0)
var<storage, read> input: array<u32>;

@group(1) @binding(0)
var<storage, read_write> output: array<u32>;

@compute
@workgroup_size(64)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>
) {

    output[0] = 54u;
    output[6] = 543u;
}
