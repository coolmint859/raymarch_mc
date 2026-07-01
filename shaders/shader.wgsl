struct CameraUniform {
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) screen_pos: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let uv = vec2f(
        f32((in_vertex_index << 1u) & 2u),
        f32(in_vertex_index & 2u)
    );
    
    // Convert UV [0, 2] to NDC Clip Space [-1, 3]
    // WebGPU NDC has Y pointing UP, so we flip the Y translation
    out.clip_position = vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    
    // Pass normalized screen position to fragment shader [-1.0, 1.0]
    out.screen_pos = vec2f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0); 
    
    return out;
}

fn sdBox(p: vec3f, b: vec3f) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn getNormal(p: vec3f) -> vec3f {
    let e = vec2f(0.001, 0.0);
    return normalize(vec3f(
        sdBox(p + e.xyy, vec3f(0.5)) - sdBox(p - e.xyy, vec3f(0.5)),
        sdBox(p + e.yxy, vec3f(0.5)) - sdBox(p - e.yxy, vec3f(0.5)),
        sdBox(p + e.yyx, vec3f(0.5)) - sdBox(p - e.yyx, vec3f(0.5))
    ));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // 1. Transform full-screen screen coords into 3D world space points using the Inv Matrix
    // Near plane point (Z = 0.0)
    let near_target = camera.inv_view_proj * vec4f(in.screen_pos.x, in.screen_pos.y, 0.0, 1.0);
    let world_near = near_target.xyz / near_target.w;

    // Far plane point (Z = 1.0)
    let far_target = camera.inv_view_proj * vec4f(in.screen_pos.x, in.screen_pos.y, 1.0, 1.0);
    let world_far = far_target.xyz / far_target.w;

    // 2. Derive Ray Origin and Ray Direction
    let ro = world_near;
    let rd = normalize(world_far - world_near);

    // 3. Marching
    var t = 0.0;
    let max_t = 20.0;
    var color = vec4f(0.01, 0.05, 0.1, 1.0); // Match clearing background color

    for (var i = 0; i < 100; i++) {
        let p = ro + rd * t;
        let d = sdBox(p, vec3f(0.5));

        if (d < 0.0001) {
            let normal = getNormal(p);
            let light_dir = normalize(vec3f(1.0, 2.0, -1.0));
            let diff = max(dot(normal, light_dir), 0.1);
            
            color = vec4f((normal * 0.5 + 0.5) * diff, 1.0);
            break;
        }
        t += d;
        if (t > max_t) { break; }
    }

    return color;
}