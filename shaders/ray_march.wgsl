/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

struct CameraUniform {
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f,
}

struct EnvironmentUniform {
    sun_dir: vec4f,
    sun_color: vec4f,
    sky_zenith: vec4f,
    sky_horizon: vec4f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> env: EnvironmentUniform;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;

struct HitInfo {
    did_hit: bool,
    normal: vec3f,
    color: vec3f,
}

fn sdfBox(p: vec3f, b: vec3f) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn getNormal(p: vec3f) -> vec3f {
    let e = vec2f(0.001, 0.0);
    return normalize(vec3f(
        sdfBox(p + e.xyy, vec3f(0.5)) - sdfBox(p - e.xyy, vec3f(0.5)),
        sdfBox(p + e.yxy, vec3f(0.5)) - sdfBox(p - e.yxy, vec3f(0.5)),
        sdfBox(p + e.yyx, vec3f(0.5)) - sdfBox(p - e.yyx, vec3f(0.5))
    ));
}

fn get_background_color(ray_dir: vec3f, sun_dir: vec3f) -> vec3<f32> {
    let sky_zenith = env.sky_zenith.xyz;
    let sky_horizon = env.sky_horizon.xyz;
    let sun_color = env.sun_color.xyz;
    let sun_intensity = env.sun_color.w;

    let ground_color = vec3<f32>(0.05, 0.05, 0.05);
    var sky_color = vec3f(0.0);

    let y = ray_dir.y;
    if (y >= 0.0) {
        let sky_blend = pow(y, 0.5); 
        sky_color = mix(sky_horizon, sky_zenith, sky_blend);
    } else {
        let horizon_glow = smoothstep(-0.05, 0.0, y);
        sky_color = mix(ground_color, sky_horizon, horizon_glow * 0.4);
    }

    let sun_alignment = dot(ray_dir, sun_dir);
    if (sun_intensity > 0.0 && sun_alignment > 0.0) {
        let sky_mask = smoothstep(-0.2, 0.0, ray_dir.y);
        let corona_glow = pow(sun_alignment, 16.0);
        let sun_disk = pow(sun_alignment, 2000.0);

        sky_color += sun_color * corona_glow * sky_mask * 0.2;
        sky_color += sun_color * sun_disk * sky_mask * 2.0; 
    }

    return clamp(sky_color, vec3<f32>(0.0), vec3<f32>(1.0));
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

    var hit_info: HitInfo;
    hit_info.did_hit = false;
    hit_info.normal = vec3f(0.0);
    hit_info.color = vec3f(0.0);

    for (var i = 0; i < 500; i++) {
        let p = ro + rd * t;
        let d = sdfBox(p, vec3f(0.5));

        if (d < 0.0001) {
            hit_info.did_hit = true;
            hit_info.normal = getNormal(p);
            hit_info.color = vec3f(0.0, 1.0, 0.0);
            break;
        }
        t += d;
        if (t > max_t) { break; }
    }

    let sun_dir = normalize(env.sun_dir.xyz);

    if hit_info.did_hit {
        let sun_intensity = env.sun_color.w;
        let diff = max(dot(hit_info.normal, sun_dir) * sun_intensity, 0.1);
        return vec4f(hit_info.color * diff, 1.0);
    } else {
        return vec4f(get_background_color(rd, sun_dir), 1.0);
    }
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