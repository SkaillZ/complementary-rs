struct TilemapUniforms {
    view_matrix: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> uniforms: TilemapUniforms;

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(input: VertexInput) -> [[builtin(position)]] vec4<f32> {
    var pos = uniforms.view_matrix * vec4<f32>(input.position, 0.0, 1.0);
    return pos;
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
