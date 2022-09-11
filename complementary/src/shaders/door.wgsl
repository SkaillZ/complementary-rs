struct DoorUniforms {
    view_matrix: mat4x4<f32>;
};
[[group(0), binding(0)]] var<uniform> uniforms: DoorUniforms;

struct VertexInput {
    [[location(0)]] vert_position: vec2<f32>;
};
struct DoorInstance {
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] position: vec2<f32>;
    [[location(3)]] size: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(input: VertexInput, instance: DoorInstance) -> VertexOutput {
    var pos = uniforms.view_matrix * vec4<f32>((input.vert_position * instance.size + instance.position), 0.0, 1.0);
    var out: VertexOutput;
    out.position = pos;
    out.color = instance.color;
    return out;
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    return input.color;
}
