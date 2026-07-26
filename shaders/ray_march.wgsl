/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

const REGION_SIZE: i32 = 32;
const REGION_VOL: u32 = u32(REGION_SIZE * REGION_SIZE * REGION_SIZE);
const AIR_REGION: i32 = -1; // indicates that a region is completely air
const AIR_VOXEL: i32 = 0; // indicates that a voxel is an air block.

const MACRO_SCALE: f32 = f32(REGION_SIZE);
const MICRO_SCALE: f32 = 1.0;

const LOCAL_AXIS = array<vec3f, 3>(
    vec3f(1.0, 0.0, 0.0), // x-axis
    vec3f(0.0, 1.0, 0.0), // y-axis
    vec3f(0.0, 0.0, 1.0), // z-axis
);

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
    steps: u32,
    material: Material
}

struct DDA {
    step_pos: vec3<i32>,
    delta_dist: vec3<f32>,
    side_dist: vec3<f32>,
}

fn init_dda(ray: Ray) -> DDA {
    var dda: DDA;
    dda.step_pos = vec3i(floor(ray.org));
    dda.delta_dist = abs(ray.inv_dir);

    let init_boundary = vec3f(dda.step_pos + max(ray.sign, vec3i(0)));
    dda.side_dist = (init_boundary - ray.org) * ray.inv_dir;

    return dda;
}

struct Ray {
    org: vec3f,
    dir: vec3f,
    inv_dir: vec3f,
    sign: vec3i,
}

fn create_ray(org: vec3f, dir: vec3f) -> Ray {
    var ray: Ray;
    ray.org = org;
    ray.dir = normalize(dir);
    ray.inv_dir = 1.0 / ray.dir;
    ray.sign = vec3i(sign(dir));

    return ray;
}

struct RayMarchConfig {
    max_iter: u32,
    max_t: f32,
}

/// calculates voxel face properties based on the face normal and ray hit position
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

/// calculates lighting via the blinn-phong lighting model
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

/// calculates a sharp shadow by tracing a secondary ray from the primary ray to a light direction
fn calc_shadow(start_pos: vec3f, normal: vec3f, light_dir: vec3f) -> f32 {
    let ray_org = start_pos + normal * 0.001;
    let shadow_ray = create_ray(ray_org, light_dir);

    var config = RayMarchConfig(50, 50.0);
    let shadow_hit = dda_march(shadow_ray, config);

    return select(1.0, 0.0, shadow_hit.did_hit);
}

/// calculates ambient occlusion by sampling the 3x3x2 grid of voxels around the occluded voxel.
fn calc_ao_volumetric(face: VoxelFace, world_pos: vec3i) -> f32 {
    let face_neighbor = world_pos + vec3i(face.normal);
    var total_occlusion = 0.0;
    var total_weight = 0.0;

    let i_tan1 = vec3i(round(face.tan1));
    let i_tan2 = vec3i(round(face.tan2));
    let i_norm = vec3i(round(face.normal));

    const MAX_Z = 2;

    for (var z = 0; z < 2; z++) {
        for (var x = -1; x <= 1; x++) {
            let depth_weight = select(1.0, 0.35, z == 1);
            let depth_offset = i_norm * z;

            for (var y = -1; y <= 1; y++) {
                let neighbor = face_neighbor + depth_offset + (i_tan1 * x) + (i_tan2 * y);
                let block_id = query_grid(neighbor);
                let is_solid = f32(block_id > AIR_VOXEL);
                
                let neighbor_uv = vec2f(f32(x) + 0.5, f32(y) + 0.5);
                let dist = distance(face.uv, neighbor_uv);
                let raw_weight = 1.0 - (dist - 0.5);
                let weight = smoothstep(0.0, 1.0, clamp(raw_weight, 0.0, 1.0)) * depth_weight;

                total_occlusion += is_solid * weight;
                total_weight += weight;
            }
        }
    }

    let occlusion_ratio = total_occlusion / max(total_weight, 0.001);
    let ao = 1.0 - pow(occlusion_ratio, 1.2);// * 0.6;
    return clamp(ao, 0.15, 1.0);
}

/// calculates a background color given environmental variables
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

/// Determines the block id of a voxel at the given world position
fn query_grid(world_pos: vec3i) -> i32 {
    let region = vec3i(
        world_pos.x >> 5u,
        world_pos.y >> 5u,
        world_pos.z >> 5u,
    );

    /// voxels live in a 5x5 band of regions for now
    if (region.x < -2 || region.x > 2 || 
        region.y != 0 ||
        region.z < -2 || region.z > 2) 
    { 
        return AIR_REGION;
    }

    let r_x = region.x + 2;
    let r_z = region.z + 2;

    let region_idx = (r_x * 5) + r_z;
    let region_start = u32(region_idx) * REGION_VOL;

    let block_pos = vec3i(
        world_pos.x & 31,
        world_pos.y & 31,
        world_pos.z & 31,
    );

    let voxel_index = u32(block_pos.x + (block_pos.y * REGION_SIZE) + (block_pos.z * REGION_SIZE * REGION_SIZE));
    return i32(voxels[region_start + voxel_index]);
}

/// march a ray through the world using the dda algorithm.
fn dda_march(ray: Ray, config: RayMarchConfig) -> HitInfo {
    let cam_int = vec3i(floor(camera.position)); // integer part of camera position
    var dda = init_dda(ray);

    var hit_info: HitInfo;
    hit_info.did_hit = false;

    var last_side_hit = 0u;
    var t = 0.0;

    for (var step = 0u; step < config.max_iter; step++) {
        let world_pos = dda.step_pos + cam_int;
        let block_id = query_grid(world_pos);

        /// break out of loop if max t is reached
        if ( t > config.max_t) { break; }

        /// skip whole region if entirely empty
        if (block_id == AIR_REGION) {
            let region_base = floor(vec3f(world_pos) / f32(REGION_SIZE)) * f32(REGION_SIZE);
            let boundary = region_base + vec3f(max(ray.sign, vec3i(0))) * f32(REGION_SIZE);
            
            let local_exit = boundary - vec3f(cam_int);
            let t_boundary = (local_exit - ray.org) * ray.inv_dir;
            
            let t_next = min(t_boundary.x, min(t_boundary.y, t_boundary.z));

            let diff = abs(vec3f(t_next) - t_boundary);
            let match_x = diff.x < 0.0001;
            let match_y = diff.y < 0.0001;
            last_side_hit = select(select(2u, 1u, match_y), 0u, match_x);
            
            t = t_next + 0.001;
            
            let current_pos = ray.org + ray.dir * t;
            dda.step_pos = vec3i(floor(current_pos));
            
            let voxel_boundary = vec3f(dda.step_pos + max(ray.sign, vec3i(0)));
            dda.side_dist = (voxel_boundary - ray.org) * ray.inv_dir;

            hit_info.t = t;
            hit_info.steps = step;
            
            continue;
        }

        /// voxel is solid, register a hit detection
        if (block_id > AIR_VOXEL) {
            hit_info.did_hit = true;
            hit_info.hit_pos = ray.org + ray.dir * t;
            hit_info.world_pos = world_pos;

            hit_info.material.color = palette.colors[u32(block_id)].xyz;
            let normal = -LOCAL_AXIS[last_side_hit] * vec3f(ray.sign);
            hit_info.face = calc_face(hit_info.hit_pos, normal);

            break;
        }

        let mask = step(dda.side_dist, dda.side_dist.yzx) * step(dda.side_dist, dda.side_dist.zxy);

        t = dot(mask, dda.side_dist);
        dda.side_dist += mask * dda.delta_dist;
        dda.step_pos += vec3i(mask) * ray.sign;

        last_side_hit = u32(mask.y) | (u32(mask.z) << 1u);

        hit_info.steps = step;
        hit_info.t = t;
    }

    return hit_info;
}

// transform the shader invocation id into the ray used for rendering
fn init_ray(id: vec3<u32>, size: vec2<u32>) -> Ray {
    let pix_center = vec2f(id.xy) + vec2f(0.5);
    let uv = ((pix_center / vec2f(size) * 2.0) - 1.0) * vec2f(1.0, -1.0);

    let near = camera.inv_view_proj * vec4f(uv.x, uv.y, 0.0, 1.0);
    let near_pos = near.xyz / near.w;

    let far = camera.inv_view_proj * vec4f(uv.x, uv.y, 1.0, 1.0);
    let far_pos = far.xyz / far.w;

    return create_ray(
        fract(camera.position),         // ray origin
        normalize(far_pos - near_pos)   // ray direction
    );
}

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_size = textureDimensions(output);

    if (id.x >= tex_size.x || id.y >= tex_size.y) {
        return;
    }

    let max_dist = 150.0;
    var config = RayMarchConfig(300, max_dist);

    let ray = init_ray(id, tex_size);
    let hit_info = dda_march(ray, config);

    // /// Step heatmap
    // let ratio = clamp(f32(hit_info.steps) / 300.0, 0.0, 1.0);
    // let color = mix(vec3f(0.0, 0.2, 1.0), vec3f(1.0, 0.1, 0.0), ratio);

    // /// depth map
    // let depth = clamp(hit_info.t / max_dist, 0.0, 1.0);
    // let color = vec3f(depth, depth, depth);

    /// regular rendering
    let sun_dir = normalize(env.sun_dir.xyz);

    var color = vec3f(0.0);
    if (hit_info.did_hit)  {
        // color = vec3f(hit_info.face.uv, 0.0);
        color = calc_lighting(hit_info, sun_dir, -ray.dir);
    } else {
        color = get_background_color(ray.dir, sun_dir);
    }

    let out_color = vec4f(color, 1.0);
    textureStore(output, id.xy, out_color);
}