/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

const REGION_SIZE: i32 = 32;
const REGION_VOL: u32 = u32(REGION_SIZE * REGION_SIZE * REGION_SIZE);
const AIR_REGION: i32 = -1; // indicates that a region is completely air
const AIR_VOXEL: i32 = 0; // indicates that a voxel is an air block.

const MACRO_SCALE: f32 = f32(REGION_SIZE);
const MICRO_SCALE: f32 = 1.0;

const LOCAL_AXIS = array<vec3i, 3>(
    vec3i(1, 0, 0), // x-axis
    vec3i(0, 1, 0), // y-axis
    vec3i(0, 0, 1), // z-axis
);

struct CameraUniform {
    inv_view_proj: mat4x4f,
    position: vec3f,
    frame: f32
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
    normal: vec3i,
    tan1: vec3i,
    tan2: vec3i,
}

struct HitInfo {
    did_hit: bool,
    hit_pos: vec3f,
    voxel_pos: vec3i,
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

/// Generates a pseudo-random point based on the current pixel and frame number
fn random_point(pixel_id: vec2f) -> vec2f {
    let phi = 1.61803398875;
    let n = pixel_id.x * 12.9898 + pixel_id.y * 78.233 + camera.frame * 437.585;
    let angle = fract(n * phi) * 6.2831853;
    let radius = sqrt(fract(n * 0.754877));
    return vec2f(angle, radius);
}

/// Perturbs the origin of a ray within a spread radius, converging the direction towards a focal plane.
fn gen_perturbed_ray(seed: vec2f, base_pos: vec3f, base_dir: vec3f, spread: f32, focal_dist: f32) -> Ray {
    let arb_vec = select(vec3f(0.0, 1.0, 0.0), vec3f(1.0, 0.0, 0.0), abs(base_dir.y) > 0.99);
    let right = normalize(cross(arb_vec, base_dir));
    let up = cross(base_dir, right);

    let jitter = random_point(seed);
    let angle = jitter.x * 6.2831853;
    let radius = sqrt(jitter.y) * spread;
    let offset = (right * cos(angle) + up * sin(angle)) * radius;

    let focal_point = base_pos + base_dir * focal_dist;
    let ray_org = base_pos + offset;
    let ray_dir = normalize(focal_point - ray_org);

    return create_ray(ray_org, ray_dir);
}

struct RayMarchConfig {
    max_iter: u32,
    max_t: f32,
    is_shadow: bool,
}

/// calculates voxel face properties based on the face normal and ray hit position
fn calc_face(hit_pos: vec3f, normal: vec3i) -> VoxelFace {
    var face: VoxelFace;
    face.normal = normal;

    let local_pos = fract(hit_pos);
    if (abs(f32(normal.y)) > 0.5) {
        face.uv = local_pos.xz;
        face.tan1 = LOCAL_AXIS[0];
        face.tan2 = LOCAL_AXIS[2];
    } else if (abs(f32(normal.x)) > 0.5) { 
        face.uv = local_pos.yz;
        face.tan1 = LOCAL_AXIS[1];
        face.tan2 = LOCAL_AXIS[2];
    } else { 
        face.uv = local_pos.xy;
        face.tan1 = LOCAL_AXIS[0];
        face.tan2 = LOCAL_AXIS[1];
    }

    return face;
}

/// calculates lighting with the blinn-phong lighting model as the base
fn calc_lighting(hit_info: HitInfo, sun_dir: vec3f, view_dir: vec3f) -> vec3f {
    // shadow
    let normal = vec3f(hit_info.face.normal);
    let shadow = calc_shadow(hit_info.hit_pos, normal, sun_dir);

    // ambient term
    let ao = calc_ao(hit_info.face, hit_info.voxel_pos);
    let amb_strength = clamp(sun_dir.y * 0.5 + 0.5, 0.05, 1.0);
    let ambient = (env.sky_zenith.xyz * amb_strength + 0.1) * ao;

    // diffuse term
    let diff_strength = max(dot(normal, sun_dir), 0.0);
    let diffuse = env.sun_color.xyz * diff_strength * shadow * ao;

    // specular term
    let half = normalize(sun_dir + view_dir);
    let spec_strength = pow(max(dot(normal, half), 0.0), 1024.0);
    let specular = env.sun_color.xyz * spec_strength * shadow;

    return (ambient + diffuse + specular) * hit_info.material.color;
}

/// calculates a semi-soft shadow by tracing a set of shadow rays by perturbing their origin
fn calc_shadow(start_pos: vec3f, normal: vec3f, light_dir: vec3f) -> f32 {
    let config = RayMarchConfig(30, 20.0, true);

    let shadow_radius = 0.0001;
    let light_dist = 0.01;
    let samples = 3.0;

    var in_shadow = 0.0;
    let ray_org = start_pos + normal * shadow_radius;
    for (var i = 0u; i < u32(samples); i++) {
        let jitter = vec2f(f32(i) * 45.12, f32(i) * 89.43);
        let seed = ray_org.xy + ray_org.zz + jitter;

        let sample_ray = gen_perturbed_ray(seed, ray_org, light_dir, shadow_radius, light_dist);
        let shadow_hit = dda_march(sample_ray, config);

        in_shadow += select(1.0, 0.0, shadow_hit.did_hit);
    }

    return in_shadow / samples;
}

/// calculates ambient occlusion by sampling the 3x3x2 grid of voxels around the occluded voxel.
fn calc_ao(face: VoxelFace, voxel_pos: vec3i) -> f32 {
    let face_neighbor = voxel_pos + vec3i(face.normal);
    var total_occlusion = 0.0;
    var total_weight = 0.0;

    for (var z = 0; z < 2; z++) {
        for (var x = -1; x <= 1; x++) {
            let depth_weight = select(1.0, 0.35, z == 1);
            let depth_offset = face.normal * z;

            for (var y = -1; y <= 1; y++) {
                let neighbor = face_neighbor + depth_offset + (face.tan1 * x) + (face.tan2 * y);
                let block_id = get_block_at(neighbor);
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
    // var color: vec3f;

    let horizon_blend = smoothstep(-0.1, 0.00, y);

    var base_color: vec3f;
    if (y < -0.05) {
        let ground_blend = smoothstep(-0.5, 0.0, y);
        base_color = mix(env.ground_color.xyz, env.sky_horizon.xyz, ground_blend);
    } else {
        let sky_blend = smoothstep(0.0, 0.2, y);
        base_color = mix(env.sky_horizon.xyz, env.sky_zenith.xyz, sky_blend);
    }

    var color = mix(env.ground_color.xyz * 0.5, base_color, horizon_blend);

    let align = max(dot(ray_dir, sun_dir), 0.0);
    let mask = smoothstep(-0.1, 0.1, y);
    let corona = pow(align, 16.0) * 0.2;
    let disk = pow(align, 2000.0) * 2.0;

    let sun_factor = (corona + disk) * mask * env.sun_color.w;
    color += env.sun_color.xyz * sun_factor;

    return saturate(color);
}

/// Determines the block of a voxel at the given world position
fn get_block_at(voxel_pos: vec3i) -> i32 {
    let region = vec3i(
        voxel_pos.x >> 5u,
        voxel_pos.y >> 5u,
        voxel_pos.z >> 5u,
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
        voxel_pos.x & 31,
        voxel_pos.y & 31,
        voxel_pos.z & 31,
    );

    let voxel_index = u32(block_pos.x + (block_pos.y * REGION_SIZE) + (block_pos.z * REGION_SIZE * REGION_SIZE));
    return i32(voxels[region_start + voxel_index]);
}

/// march a ray through the world using the dda algorithm.
fn dda_march(ray: Ray, config: RayMarchConfig) -> HitInfo {
    let cam_int = vec3i(floor(camera.position)); // integer part of camera position
    var dda = init_dda(ray);
    
    let pos_ray_sign = max(ray.sign, vec3i(0));
    let rgn_size_f = f32(REGION_SIZE);
    let rgn_offset = vec3f(pos_ray_sign) * rgn_size_f;

    var hit_info: HitInfo;
    hit_info.did_hit = false;

    var last_side_hit = 0u;
    var t = 0.0;

    for (var step = 0u; step < config.max_iter; step++) {
        if ( t > config.max_t) {
            hit_info.t = config.max_t;
            break; 
        }

        let voxel_pos = dda.step_pos + cam_int;
        let block_id = get_block_at(voxel_pos);

        // voxel is solid, register a hit detection
        if (block_id > AIR_VOXEL) {
            hit_info.did_hit = true;
            
            if (config.is_shadow) { break; }

            hit_info.hit_pos = ray.org + ray.dir * t;
            hit_info.voxel_pos = voxel_pos;

            hit_info.material.color = palette.colors[u32(block_id)].xyz;
            let normal = -LOCAL_AXIS[last_side_hit] * ray.sign;
            hit_info.face = calc_face(hit_info.hit_pos, normal);

            break;
        }

        // calculate standard dda step
        let mask = step(dda.side_dist, dda.side_dist.yzx) * step(dda.side_dist, dda.side_dist.zxy);

        t = dot(mask, dda.side_dist);
        dda.side_dist += mask * dda.delta_dist;
        dda.step_pos += vec3i(mask) * ray.sign;

        last_side_hit = u32(mask.y) | (u32(mask.z) << 1u);

        // skip whole region if entirely empty (calculates region step)
        if (block_id == AIR_REGION) {
            let region_base = floor(vec3f(voxel_pos) / rgn_size_f) * rgn_size_f;
            let boundary = region_base + rgn_offset;
            
            let t_boundary = (boundary - vec3f(cam_int) - ray.org) * ray.inv_dir;
            let t_next = min(t_boundary.x, min(t_boundary.y, t_boundary.z));

            let side_match = abs(vec3f(t_next) - t_boundary) < vec3f(0.0001);
            last_side_hit = select(select(2u, 1u, side_match.y), 0u, side_match.x);
            
            t = t_next + 0.001;
            
            let current_pos = ray.org + ray.dir * t;
            dda.step_pos = vec3i(floor(current_pos));
            
            let voxel_boundary = vec3f(dda.step_pos + pos_ray_sign);
            dda.side_dist = (voxel_boundary - ray.org) * ray.inv_dir;
        }

        hit_info.steps = step;
        hit_info.t = t;
    }

    return hit_info;
}

// transform the shader invocation id into the ray used for rendering
fn init_ray(id: vec3<u32>, size: vec2<u32>) -> Ray {
    let pix_center = vec2f(id.xy) + vec2f(0.5);
    let uv = ((pix_center / vec2f(size) * 2.0) - 1.0) * vec2f(1.0, -1.0);

    let clip = vec4f(uv.x, uv.y, 1.0, 1.0);
    let unproj = camera.inv_view_proj * clip;
    let world_target = unproj.xyz / unproj.w;

    let base_org = fract(camera.position);
    let base_dir = normalize(world_target - camera.position);

    let focal_dist = 20.0;
    let aperture =  0.03;
    return gen_perturbed_ray(pix_center, base_org, base_dir, aperture, focal_dist);
}

fn get_density() -> f32 {
    return 0.003;
}

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_size = textureDimensions(output);

    if (id.x >= tex_size.x || id.y >= tex_size.y) {
        return;
    }

    let max_dist = 300.0;
    let max_steps = 300u;
    var config = RayMarchConfig(max_steps, max_dist, false);

    let ray = init_ray(id, tex_size);
    let hit_info = dda_march(ray, config);
    
    let depth = clamp(hit_info.t / max_dist, 0.0, 1.0);
    // let color = vec3f(depth);

    // // Step heatmap
    // let ratio = f32(hit_info.steps) / f32(max_steps);
    // let color = mix(vec3f(0.0, 0.2, 1.0), vec3f(1.0, 0.1, 0.0), ratio);

    /// regular rendering
    let sun_dir = normalize(env.sun_dir.xyz);
    var color = vec3f(0.0);
    if (hit_info.did_hit)  {
        // color = vec3f(hit_info.face.uv, 0.0);
        color = calc_lighting(hit_info, sun_dir, -ray.dir);
    } else {
        color = get_background_color(ray.dir, sun_dir);
    }

    var fog_factor = 1.0 - exp(-hit_info.t * get_density());
    let hit_height = (camera.position + ray.dir * hit_info.t).y;
    let height_falloff = saturate(1.0 - abs(ray.dir.y) * 4.0);

    fog_factor = saturate(fog_factor * height_falloff);
    color = mix(color, env.sky_horizon.xyz, fog_factor);

    let out_color = vec4(color, depth);
    textureStore(output, id.xy, out_color);
}