/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

struct CameraUniform {
    inv_view_proj: mat4x4f,
    position: vec3f,
}

struct EnvironmentUniform {
    sun_dir: vec4f,
    sun_color: vec4f,
    sky_zenith: vec4f,
    sky_horizon: vec4f,
    ground_color: vec4f,
}

struct PaletteUniform {
    colors: array<vec4<f32>, 5>,
}

struct Region {
    coord: vec4<i32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> env: EnvironmentUniform;
@group(0) @binding(2) var<uniform> palette: PaletteUniform;
@group(0) @binding(3) var<storage, read> voxels: array<u32>;
@group(0) @binding(4) var<storage, read> regions: array<Region, 9>;
@group(0) @binding(5) var output: texture_storage_2d<rgba16float, write>;

struct Material {
    normal: vec3f,
    color: vec3f,
}

struct HitInfo {
    did_hit: bool,
    t: f32,
    material: Material
}

struct Ray {
    org: vec3f,
    dir: vec3f,
}

struct DDA {
    step_pos: vec3<i32>,
    delta_dist: vec3<f32>,
    step_dir: vec3<i32>,
    side_dist: vec3<f32>
}

fn calc_lighting(material: Material, sun_dir: vec3f, view_dir: vec3f) -> vec3f {
    let amb_strength = clamp(sun_dir.y * 0.5 + 0.5, 0.05, 1.0);
    let ambient = env.sky_zenith.xyz * amb_strength + 0.1;

    let diff_strength = max(dot(material.normal, sun_dir), 0.0);
    let diffuse = env.sun_color.xyz * diff_strength;

    let half = normalize(sun_dir + view_dir);
    let spec_strength = pow(max(dot(material.normal, half), 0.0), 256.0);
    let specular = env.sun_color.xyz * spec_strength;

    return (ambient + diffuse + specular) * material.color;
}

fn get_background_color(ray_dir: vec3f, sun_dir: vec3f) -> vec3<f32> {
    let y = ray_dir.y;
    var color: vec3f;

    if (y >= 0.0) {
        let sky_blend = pow(y, 0.5); 
        color = mix(env.sky_horizon.xyz, env.sky_zenith.xyz, sqrt(y));
    } else {
        let horizon_glow = smoothstep(-0.05, 0.0, y);
        color = mix(env.ground_color.xyz, env.sky_horizon.xyz, horizon_glow * 0.4);
    }

    let align = max(dot(ray_dir, sun_dir), 0.0);
    let mask = smoothstep(-0.2, 0.0, y);
    let corona = pow(align, 16.0) * 0.2;
    let disk = pow(align, 2000.0) * 2.0;

    let sun_factor = (corona + disk) * mask * env.sun_color.w;
    color += env.sun_color.xyz * sun_factor;

    return saturate(color);
}

fn march(ray: Ray) -> HitInfo {
    var dda = init_dda(ray);

    var hit_info: HitInfo;
    hit_info.did_hit = false;
    hit_info.material.normal = vec3f(0.0);
    hit_info.material.color = vec3f(0.0);

    var last_side_hit = 0; 
    let CHUNK_SIZE = 32;
    let CHUNK_VOL = u32(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE);
    var enter_t: f32 = 0.0;

    let cam_int = vec3i(floor(camera.position)); // integer part of camera position

    for (var i = 0; i < 300; i++) {
        let exit_t = min(dda.side_dist.x, min(dda.side_dist.y, dda.side_dist.z));

        // map step pos to region space, then add 1 to get between 0 and 2, use to find index
        // if region is step pos is (16, 20, -2), region is (0, 0, -1), which maps to index 3
        let world_pos = dda.step_pos + cam_int;
        let region_x = world_pos.x >> 5u;
        let region_y = world_pos.y >> 5u;
        let region_z = world_pos.z >> 5u;

        if (region_x >= -2 && region_x <= 2 && 
            region_y == 0 && 
            // region_y >= -1 && region_y <= 1 &&
            region_z >= -2 && region_z <= 2) 
        {
            let r_x = region_x + 2;
            let r_z = region_z + 2;

            let region_idx = (r_x * 3) + r_z;
            let region_start = u32(region_idx) * CHUNK_VOL;

            let block_pos = vec3i(
                world_pos.x & 31,
                world_pos.y & 31,
                world_pos.z & 31,
            );

            let voxel_index = u32(block_pos.x + (block_pos.y * CHUNK_SIZE) + (block_pos.z * CHUNK_SIZE * CHUNK_SIZE));
            let block_id = voxels[region_start + voxel_index] & 0xFFu;

            if (block_id > 0u) {
                hit_info.did_hit = true;
                hit_info.material.color = palette.colors[block_id].xyz;

                if (last_side_hit == 0) { 
                    hit_info.material.normal.x = -f32(dda.step_dir.x);
                } else if (last_side_hit == 1) { 
                    hit_info.material.normal.y = -f32(dda.step_dir.y); 
                } else { 
                    hit_info.material.normal.z = -f32(dda.step_dir.z); 
                }

                break;
            }
        }

        // DDA Step
        if (dda.side_dist.x < dda.side_dist.y && dda.side_dist.x < dda.side_dist.z) {
            dda.side_dist.x += dda.delta_dist.x; 
            dda.step_pos.x += dda.step_dir.x; 
            last_side_hit = 0;
        } else if (dda.side_dist.y < dda.side_dist.z) {
            dda.side_dist.y += dda.delta_dist.y; 
            dda.step_pos.y += dda.step_dir.y; 
            last_side_hit = 1;
        } else {
            dda.side_dist.z += dda.delta_dist.z; 
            dda.step_pos.z += dda.step_dir.z; 
            last_side_hit = 2;
        }

        enter_t = exit_t;
    }

    hit_info.t = enter_t;
    return hit_info;
}

fn init_dda(ray: Ray) -> DDA {
    var dda: DDA;

    dda.step_pos = vec3i(floor(ray.org));
    dda.step_dir = vec3i(sign(ray.dir));

    let inv_dir = 1.0 / ray.dir;
    dda.delta_dist = abs(inv_dir);

    let init_side_dist = ray.org - vec3f(dda.step_pos);
    let t = select(init_side_dist, 1.0 - init_side_dist, ray.dir >= vec3f(0.0));
    dda.side_dist = t * dda.delta_dist;
    dda.side_dist = max(dda.side_dist, vec3f(1e-6));

    return dda;
}

// transform the shader invocation id into the ray used for rendering
fn init_ray(id: vec3<u32>, size: vec2<u32>) -> Ray {
    let uv = ((vec2f(id.xy) / vec2f(size) * 2.0) - 1.0) * vec2f(1.0, -1.0);

    let near = camera.inv_view_proj * vec4f(uv.x, uv.y, 0.0, 1.0);
    let near_pos = near.xyz / near.w;

    let far = camera.inv_view_proj * vec4f(uv.x, uv.y, 1.0, 1.0);
    let far_pos = far.xyz / far.w;

    var ray: Ray;
    ray.org = (camera.position - floor(camera.position)); // fractional part of camera position
    ray.dir = normalize(far_pos - near_pos);
    return ray;
}

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_size = textureDimensions(output);

    if (id.x >= tex_size.x || id.y >= tex_size.y) {
        return;
    }

    let ray = init_ray(id, tex_size);
    let hit_info = march(ray);

    let sun_dir = normalize(env.sun_dir.xyz);

    var color = vec3f(0.0);
    if (hit_info.did_hit)  {
        color = calc_lighting(hit_info.material, sun_dir, -ray.dir);
    } else {
        color = get_background_color(ray.dir, sun_dir);
    }

    // let max_dist: f32 = 1000.0;
    // let depth = clamp(hit_info.t / max_dist, 0.0, 1.0);
    // color = vec3f(depth, depth, depth);

    let out_color = vec4f(color, 1.0);
    textureStore(output, id.xy, out_color);
}