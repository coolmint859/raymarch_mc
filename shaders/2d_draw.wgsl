struct CameraUniform {
    view_proj: mat4x4f,
    position: vec3f,
    frame: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var fire: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
};

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
}

struct InstanceInput {
    @location(2) mat_col_0: vec4f,
    @location(3) mat_col_1: vec4f,
    @location(4) mat_col_2: vec4f,
    @location(5) mat_col_3: vec4f,
}

@vertex
fn vs_main(vertex: VertexInput, model: InstanceInput) -> VertexOutput {
    var model_matrix = mat4x4f(
        model.mat_col_0,
        model.mat_col_1,
        model.mat_col_2,
        model.mat_col_3,
    );

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * vec4f(vertex.position, 1.0);
    out.tex_coords = vertex.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let color = textureSampleLevel(fire, samp, in.tex_coords, 0);
    return vec4f(color.rgb * color.a, color.a);
}