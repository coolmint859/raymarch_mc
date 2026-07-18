/// Does a simple texture read onto a large triangle that fills the screen

@group(0) @binding(0) var input: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) << 2u) - 1.0;
    let y = f32(i32(in_vertex_index & 2u) << 1u) - 1.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let pixel_coords = vec2<i32>(in.clip_position.xy);
    let raw_color = textureLoad(input, pixel_coords, 0).rgb;

    // dithering to reduce color banding
    let noise = fract(sin(dot(in.clip_position.xy, vec2f(12.9898, 78.233))) * 43758.5453);
    let dither = (noise - 0.5) / 255.0;
    let out_color = vec3f(dither) + raw_color;

    return vec4f(out_color, 1.0);
}