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
    colors: array<vec4<f32>, 4>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> env: EnvironmentUniform;
@group(0) @binding(2) var<uniform> palette: PaletteUniform;
@group(0) @binding(3) var<storage, read> voxels: array<u32>;
@group(0) @binding(4) var output: texture_storage_2d<rgba16float, write>;

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
    map_pos: vec3<i32>,
    delta_dist: vec3<f32>,
    step_dir: vec3<i32>,
    side_dist: vec3<f32>
}

fn calc_lighting(material: Material, sun_dir: vec3f, view_dir: vec3f) -> vec3f {
    let sun_intensity = env.sun_color.w;
    let amb_strength = dot(sun_dir, vec3f(0.0, 1.0, 0.0));
    let ambient: vec3f = amb_strength * env.sun_color.xyz;

    let half = normalize(sun_dir + view_dir);
    let spec = pow(max(dot(material.normal, half), 0.0), 256.0);
    let specular = env.sun_color.xyz * spec;

    let diffuse = max(dot(material.normal, sun_dir) * sun_intensity, 0.3);
    return (ambient + diffuse + specular) * material.color;
}

fn get_background_color(ray_dir: vec3f, sun_dir: vec3f) -> vec3<f32> {
    let sky_zenith = env.sky_zenith.xyz;
    let sky_horizon = env.sky_horizon.xyz;
    let sun_color = env.sun_color.xyz;
    let sun_intensity = env.sun_color.w;
    let ground_color = env.ground_color.xyz;
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

fn init_dda(ray: Ray) -> DDA {
    var dda: DDA;
    dda.map_pos = vec3<i32>(floor(ray.org));

    // Calculate how far the ray must travel along an axis to cross a full 1.0 grid unit
    // (Protects against divide-by-zero using a massive fallback distance)
    dda.delta_dist = vec3<f32>(
        select(1e30, abs(1.0 / ray.dir.x), ray.dir.x != 0.0),
        select(1e30, abs(1.0 / ray.dir.y), ray.dir.y != 0.0),
        select(1e30, abs(1.0 / ray.dir.z), ray.dir.z != 0.0)
    );

    // Initialize tracking steps and start offsets based on ray direction vector
    if (ray.dir.x < 0.0) {
        dda.step_dir.x = -1;
        dda.side_dist.x = (ray.org.x - f32(dda.map_pos.x)) * dda.delta_dist.x;
    } else {
        dda.step_dir.x = 1;
        dda.side_dist.x = (f32(dda.map_pos.x + 1) - ray.org.x) * dda.delta_dist.x;
    }

    if (ray.dir.y < 0.0) {
        dda.step_dir.y = -1;
        dda.side_dist.y = (ray.org.y - f32(dda.map_pos.y)) * dda.delta_dist.y;
    } else {
        dda.step_dir.y = 1;
        dda.side_dist.y = (f32(dda.map_pos.y + 1) - ray.org.y) * dda.delta_dist.y;
    }

    if (ray.dir.z < 0.0) {
        dda.step_dir.z = -1;
        dda.side_dist.z = (ray.org.z - f32(dda.map_pos.z)) * dda.delta_dist.z;
    } else {
        dda.step_dir.z = 1;
        dda.side_dist.z = (f32(dda.map_pos.z + 1) - ray.org.z) * dda.delta_dist.z;
    }

    return dda;
}

fn march(ray: Ray) -> HitInfo {
    var dda = init_dda(ray);

    var hit_info: HitInfo;
    hit_info.did_hit = false;
    hit_info.material.normal = vec3f(0.0);
    hit_info.material.color = vec3f(0.0);

    var last_side_hit = 0; 
    let CHUNK_SIZE = 32;
    var entry_t: f32 = 0.0;

    let world_pos = vec3f(0.0) - camera.position;

    for (var i = 0; i < 200; i++) {
        // Current continuous float position of the ray right now
        let exit_t = min(dda.side_dist.x, min(dda.side_dist.y, dda.side_dist.z));
        let p = ray.org + ray.dir * exit_t;

        let block_pos = dda.map_pos - vec3<i32>(floor(world_pos));

        // --- Standard Voxel Solid Hit Detection ---
        // if (dda.map_pos.x >= 0 && dda.map_pos.x < CHUNK_SIZE &&
        //     dda.map_pos.y >= 0 && dda.map_pos.y < CHUNK_SIZE &&
        //     dda.map_pos.z >= 0 && dda.map_pos.z < CHUNK_SIZE) {

        if (block_pos.x >= 0 && block_pos.x < CHUNK_SIZE &&
            block_pos.y >= 0 && block_pos.y < CHUNK_SIZE &&
            block_pos.z >= 0 && block_pos.z < CHUNK_SIZE) {
            
            let voxel_index = u32(block_pos.x + (block_pos.y * CHUNK_SIZE) + (block_pos.z * CHUNK_SIZE * CHUNK_SIZE));
            // let voxel_index = u32(dda.map_pos.x + (dda.map_pos.y * CHUNK_SIZE) + (dda.map_pos.z * CHUNK_SIZE * CHUNK_SIZE));
            let block_id = voxels[voxel_index] & 0xFFu;

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

        entry_t = exit_t;

        // DDA Step
        if (dda.side_dist.x < dda.side_dist.y && dda.side_dist.x < dda.side_dist.z) {
            dda.side_dist.x += dda.delta_dist.x; 
            dda.map_pos.x += dda.step_dir.x; 
            last_side_hit = 0;
        } else if (dda.side_dist.y < dda.side_dist.z) {
            dda.side_dist.y += dda.delta_dist.y; 
            dda.map_pos.y += dda.step_dir.y; 
            last_side_hit = 1;
        } else {
            dda.side_dist.z += dda.delta_dist.z; 
            dda.map_pos.z += dda.step_dir.z; 
            last_side_hit = 2;
        }
    }

    hit_info.t = entry_t;
    return hit_info;
}

// transform the shader invocation id into the ray used for rendering
fn init_ray(id: vec3<u32>, size: vec2<u32>) -> Ray {
    let x = (f32(id.x) / f32(size.x)) * 2.0 - 1.0;
    let y = 1.0 - (f32(id.y) / f32(size.y)) * 2.0;
    let uv = vec2f(x, y);

    let near_target = camera.inv_view_proj * vec4f(uv.x, uv.y, 0.0, 1.0);
    let far_target = camera.inv_view_proj * vec4f(uv.x, uv.y, 1.0, 1.0);

    let cam_int = vec3<i32>(floor(camera.position));
    let cam_frac = camera.position - floor(camera.position);

    var ray: Ray;
    ray.org = (near_target.xyz / near_target.w) + cam_frac;
    ray.dir = normalize((far_target.xyz / far_target.w) - (near_target.xyz / near_target.w));

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

    // let max_dist: f32 = 100.0;
    // let depth = clamp(hit_info.t / max_dist, 0.0, 1.0);
    // color = vec3f(depth, depth, depth);

    let out_color = vec4f(color, 1.0);
    textureStore(output, id.xy, out_color);
}