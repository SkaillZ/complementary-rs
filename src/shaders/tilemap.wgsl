struct TilemapUniforms {
    view_matrix: mat4x4<f32>;
    invert_colors: i32;
};
[[group(0), binding(0)]] var<uniform> uniforms: TilemapUniforms;

struct ColoredVertex {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(in: ColoredVertex) -> VertexOutput {
    var out: VertexOutput;
    out.position = uniforms.view_matrix * vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    if (uniforms.invert_colors == 1) {
        return vec4<f32>(1.0) - in.color;
    }
    return in.color;
}
