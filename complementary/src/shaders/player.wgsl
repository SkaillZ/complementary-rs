struct PlayerUniforms {
    view_matrix: mat4x4<f32>;
    model_matrix: mat4x4<f32>;
    color: vec4<f32>;
};
[[group(0), binding(0)]] var<uniform> uniforms: PlayerUniforms;

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(input: VertexInput) -> [[builtin(position)]] vec4<f32> {
    var pos = uniforms.view_matrix * uniforms.model_matrix * vec4<f32>(input.position, 0.0, 1.0);
    return pos;
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return uniforms.color;
}
