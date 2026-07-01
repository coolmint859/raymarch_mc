struct CameraUniform {
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

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

fn march(screen_pos: vec2f) -> vec4f {
    let near_target = camera.inv_view_proj * vec4f(screen_pos.x, screen_pos.y, 0.0, 1.0);
    let world_near = near_target.xyz / near_target.w;

    let far_target = camera.inv_view_proj * vec4f(screen_pos.x, screen_pos.y, 1.0, 1.0);
    let world_far = far_target.xyz / far_target.w;

    let ro = world_near;
    let rd = normalize(world_far - world_near);

    var t = 0.0;
    let max_t = 20.0;
    var color = vec4f(0.0, 0.0, 0.0, 1.0);

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

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let texture_size = textureDimensions(output);

    if (id.x >= texture_size.x || id.y >= texture_size.y) {
        return;
    }

    let x = (f32(id.x) / f32(texture_size.x)) * 2.0 - 1.0;
    let y = 1.0 - (f32(id.y) / f32(texture_size.y)) * 2.0;
    let uv = vec2f(x, y);

    let color = march(uv);
    
    textureStore(output, id.xy, color);
}