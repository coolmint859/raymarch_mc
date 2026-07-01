@group(0) @binding(0) var input: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) << 2u) - 1.0;
    let y = f32(i32(in_vertex_index & 2u) << 1u) - 1.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let texture_size = textureDimensions(input);
    // let pixel_coords = vec2<i32>(
    //     i32(in.uv.x * f32(texture_size.x)),
    //     i32(in.uv.y * f32(texture_size.y))
    // );
    let pixel_coords = vec2<i32>(in.clip_position.xy);
    
    return textureLoad(input, pixel_coords, 0);
}