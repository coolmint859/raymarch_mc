/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

const REGION_SIZE: i32 = 32;
const REGION_VOL: u32 = u32(REGION_SIZE * REGION_SIZE * REGION_SIZE);

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
    color: vec3f,
}

struct VoxelFace {
    uv: vec2f,
    normal: vec3f,
    tan1: vec3f,
    tan2: vec3f,
}

struct HitInfo {
    did_hit: bool,
    hit_pos: vec3f,
    world_pos: vec3i,
    face: VoxelFace,
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

struct RayMarchConfig {
    max_iter: u32,
    max_t: f32,
}

fn calc_face(hit_pos: vec3f, normal: vec3f) -> VoxelFace {
    var face: VoxelFace;
    face.normal = normal;

    let local_pos = fract(hit_pos);
    if (abs(normal.y) > 0.5) {
        face.uv = local_pos.xz;
        face.tan1 = vec3f(1.0, 0.0, 0.0);
        face.tan2 = vec3f(0.0, 0.0, 1.0);
    } else if (abs(normal.x) > 0.5) { 
        face.uv = local_pos.yz;
        face.tan1 = vec3f(0.0, 1.0, 0.0);
        face.tan2 = vec3f(0.0, 0.0, 1.0);
    } else { 
        face.uv = local_pos.xy;
        face.tan1 = vec3f(1.0, 0.0, 0.0);
        face.tan2 = vec3f(0.0, 1.0, 0.0);
    }

    return face;
}

fn calc_lighting(hit_info: HitInfo, sun_dir: vec3f, view_dir: vec3f) -> vec3f {
    let ao = calc_ao_volumetric(hit_info.face, hit_info.world_pos);
    let normal = hit_info.face.normal;
    
    let amb_strength = clamp(sun_dir.y * 0.5 + 0.5, 0.05, 1.0);
    let ambient = (env.sky_zenith.xyz * amb_strength + 0.1) * ao;

    let shadow = calc_shadow(hit_info.hit_pos, normal, sun_dir);

    let diff_strength = max(dot(normal, sun_dir), 0.0);
    let diffuse = env.sun_color.xyz * diff_strength * shadow;

    let half = normalize(sun_dir + view_dir);
    let spec_strength = pow(max(dot(normal, half), 0.0), 256.0);
    let specular = env.sun_color.xyz * spec_strength * shadow;

    return (ambient + diffuse + specular) * hit_info.material.color;
}

fn calc_shadow(start_pos: vec3f, normal: vec3f, light_dir: vec3f) -> f32 {
    var shadow_ray: Ray;
    shadow_ray.org = start_pos + normal * 0.001;
    shadow_ray.dir = light_dir;

    var config: RayMarchConfig;
    config.max_iter = 50;
    config.max_t = 50.0;

    let shadow_hit = march(shadow_ray, config);

    if (shadow_hit.did_hit) { return 0.0; } else { return 1.0; }
}

fn calc_ao_volumetric(face: VoxelFace, world_pos: vec3i) -> f32 {
    let face_neighbor = world_pos + vec3i(face.normal);
    var total_occlusion = 0.0;
    var total_weight = 0.0;

    for (var z = 0; z < 2; z++) {
        for (var x = -1; x <= 1; x++) {
            for (var y = -1; y <= 1; y++) {
                let tan_offset = face.tan1 * f32(x) + face.tan2 * f32(y);
                let depth_offset = face.normal * f32(z);
                let neighbor = face_neighbor + vec3i(depth_offset) + vec3i(round(tan_offset));

                let block_id = block_id_at(neighbor);
                let is_solid = f32(block_id > 0u);

                let wx = max(0.0, 1.0 - abs(face.uv.x - (f32(x) * 0.5 + 0.5)));
                let wy = max(0.0, 1.0 - abs(face.uv.y - (f32(y) * 0.5 + 0.5)));
                var weight = wx * wy * select(1.0, 0.0, z == 1);

                total_occlusion += is_solid * weight;
                total_weight += weight;
            }
        }
    }

    let occlusion_ratio = total_occlusion / max(total_weight, 0.001);
    let ao = 1.0 - pow(occlusion_ratio, 0.9);// * 0.6;
    return clamp(ao, 0.1, 1.0);
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

fn block_id_at(world_pos: vec3i) -> u32 {
    let region_x = world_pos.x >> 5u;
    let region_y = world_pos.y >> 5u;
    let region_z = world_pos.z >> 5u;

    if (region_x < -2 || region_x > 2 || 
        region_y != 0 ||
        region_z < -2 || region_z > 2) 
    { 
        return 0u; // air block
    }

    let r_x = region_x + 2;
    let r_z = region_z + 2;

    let region_idx = (r_x * 5) + r_z;
    let region_start = u32(region_idx) * REGION_VOL;

    let block_pos = vec3i(
        world_pos.x & 31,
        world_pos.y & 31,
        world_pos.z & 31,
    );

    let voxel_index = u32(block_pos.x + (block_pos.y * REGION_SIZE) + (block_pos.z * REGION_SIZE * REGION_SIZE));
    return voxels[region_start + voxel_index] & 0xFFu;
}

fn march(ray: Ray, config: RayMarchConfig) -> HitInfo {
    var dda = init_dda(ray);

    var hit_info: HitInfo;
    hit_info.did_hit = false;
    hit_info.t = 0.0;

    var last_side_hit = 0;
    var enter_t: f32 = 0.0;

    let cam_int = vec3i(floor(camera.position)); // integer part of camera position

    for (var i = 0u; i < config.max_iter; i++) {
        let exit_t = min(dda.side_dist.x, min(dda.side_dist.y, dda.side_dist.z));

        if (enter_t > config.max_t) { break; }

        let world_pos = dda.step_pos + cam_int;
        let block_id = block_id_at(world_pos);

        if (block_id > 0u) {
            hit_info.did_hit = true;
            hit_info.hit_pos = ray.org + ray.dir * enter_t;
            hit_info.world_pos = world_pos;

            hit_info.material.color = palette.colors[block_id].xyz;

            var normal = vec3f(0.0);
            if (last_side_hit == 0) { 
                normal.x = -f32(dda.step_dir.x);
            } else if (last_side_hit == 1) { 
                normal.y = -f32(dda.step_dir.y); 
            } else { 
                normal.z = -f32(dda.step_dir.z); 
            }

            hit_info.face = calc_face(hit_info.hit_pos, normal);

            break;
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

    var config = RayMarchConfig(300, 150.0);

    let ray = init_ray(id, tex_size);
    let hit_info = march(ray, config);

    let sun_dir = normalize(env.sun_dir.xyz);

    var color = vec3f(0.0);
    if (hit_info.did_hit)  {
        // color = vec3f(hit_info.face.uv, 0.0);
        color = calc_lighting(hit_info, sun_dir, -ray.dir);
    } else {
        color = get_background_color(ray.dir, sun_dir);
    }

    // let max_dist: f32 = 1000.0;
    // let depth = clamp(hit_info.t / max_dist, 0.0, 1.0);
    // color = vec3f(depth, depth, depth);

    let out_color = vec4f(color, 1.0);
    textureStore(output, id.xy, out_color);
}