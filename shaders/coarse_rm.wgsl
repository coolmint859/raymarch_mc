/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

const REGION_SIZE: i32 = 32;
const REGION_VOL: u32 = u32(REGION_SIZE * REGION_SIZE * REGION_SIZE);

const AIR_BRICK4: i32 = -1;     // indicates that a grid4 brick is completely air
const AIR_BRICK8: i32 = -2;     // indicates that a grid8 brick is completely air
const AIR_BRICK16: i32 = -3;    // indicates that a grid16 brick is completely air
const AIR_REGION: i32 = -4;     // indicates that a full region is completely air

const AIR_VOXEL: i32 = 0; // indicates that a voxel is an air block.

const SCALES = array<f32, 5>(1.0, 4.0, 8.0, 16.0, 32.0);

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

struct RegionGrids {
    grid4: array<u32, 16>,  // 4x4x4 voxel bricks, 512 per region
    grid8: array<u32, 2>,   // 8x8x8 voxel bricks, 64 per region
    grid16: u32             // 16x16x16 bricks, 8 per region
}

/// world data
@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var<storage, read> voxels: array<u32>;
@group(1) @binding(1) var<storage, read> grids: array<RegionGrids>;

/// output textures
@group(2) @binding(0) var positions: texture_storage_2d<rgba16float, write>;
@group(2) @binding(1) var normals: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(2) var depth: texture_storage_2d<r32float, write>;
@group(2) @binding(3) var material: texture_storage_2d<r32float, write>;

struct HitInfo {
    did_hit: bool,
    hit_pos: vec3f,
    normal: vec3i,
    mat_id: u32,

    t: f32,
    steps: u32,
}

struct DdaState {
    t: f32,
    cam_int: vec3i,
    pos: vec3i,
    dist: vec3f,
    lsh: u32,
    scale_idx: u32,
}

fn init_dda(ray: Ray) -> DdaState {
    var state: DdaState;
    state.t = 0.0;
    state.lsh = 0u;
    state.scale_idx = 0u;

    state.cam_int = vec3i(floor(camera.position));
    state.pos = vec3i(floor(ray.org));
    state.dist = (vec3f(state.pos + ray.pos_sign) - ray.org) * ray.inv_dir;
    
    return state;
}

struct Ray {
    org: vec3f,
    dir: vec3f,
    inv_dir: vec3f,
    sign: vec3i,
    pos_sign: vec3i,
}

fn create_ray(org: vec3f, dir: vec3f) -> Ray {
    var ray: Ray;
    ray.org = org;
    ray.dir = normalize(dir);
    ray.inv_dir = 1.0 / ray.dir;
    ray.sign = vec3i(sign(dir));
    ray.pos_sign = max(ray.sign, vec3i(0));

    return ray;
}

struct RayMarchConfig {
    max_iter: u32,
    max_t: f32,
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

fn check_grid16(grids: RegionGrids, pos: vec3i) -> bool {
    let b16 = pos >> vec3u(4);
    let idx16 = u32(b16.x + (b16.y * 2) + (b16.z * 4));
    let bit_idx = idx16 >> 5u;
    return ((grids.grid16 >> bit_idx) & 1u) == 0u;
}

fn check_grid8(grids: RegionGrids, pos: vec3i) -> bool {
    let b8 = pos >> vec3u(3);
    let idx8 = u32(b8.x + (b8.y * 4) + (b8.z * 16));
    let word_idx = idx8 >> 5u;
    let bit_idx = idx8 & 31;
    return ((grids.grid8[word_idx] >> bit_idx) & 1u) == 0u;
}

fn check_grid4(grids: RegionGrids, pos: vec3i) -> bool {
    let b4 = pos >> vec3u(2);
    let idx4 = u32(b4.x + (b4.y * 8) + (b4.z * 64));
    let word_idx = idx4 >> 5u;
    let bit_idx = idx4 & 31;
    return ((grids.grid4[word_idx] >> bit_idx) & 1u) == 0u;
}

/// Determines the block of a voxel at the given world position
fn get_block_at(voxel_pos: vec3i) -> i32 {
    let region: vec3i = voxel_pos >> vec3u(5);

    if (any(region < vec3i(-4, 0, -4)) || any(region > vec3i(4, 0, 4))) {
        return AIR_REGION;
    }

    let r_x = region.x + 4;
    let r_z = region.z + 4;
    let region_idx = (r_x * 7) + r_z;

    let block_pos = voxel_pos & vec3i(31);
    let grids = grids[region_idx];

    if (check_grid16(grids, block_pos)) { return AIR_BRICK16; }
    if (check_grid8(grids, block_pos)) { return AIR_BRICK8; }
    if (check_grid4(grids, block_pos)) { return AIR_BRICK4; }

    let region_start = u32(region_idx) * REGION_VOL;
    let voxel_index = u32(block_pos.x + (block_pos.y * REGION_SIZE) + (block_pos.z * REGION_SIZE * REGION_SIZE));
    return i32(voxels[region_start + voxel_index]);
}

/// Step the ray forward through the grid based on the scale
fn dda_step(
    ray: Ray, 
    curr_step: ptr<function, DdaState>, 
    voxel_pos: vec3i, 
    scale: f32
) {
    let scale_offset = vec3f(ray.pos_sign) * scale;
    let cell_base = floor(vec3f(voxel_pos) / scale) * scale;
    let boundary = cell_base + scale_offset;
    
    let t_boundary = (boundary - vec3f((*curr_step).cam_int) - ray.org) * ray.inv_dir;
    let t_next = min(t_boundary.x, min(t_boundary.y, t_boundary.z));
    
    let side_match = abs(vec3f(t_next) - t_boundary) < vec3f(0.0001);
    (*curr_step).lsh = select(select(2u, 1u, side_match.y), 0u, side_match.x);
    
    (*curr_step).t = t_next + 0.0001;
    
    let current_pos = ray.org + ray.dir * (*curr_step).t;
    (*curr_step).pos = vec3i(floor(current_pos));
    
    let voxel_boundary = vec3f((*curr_step).pos + ray.pos_sign);
    (*curr_step).dist = (voxel_boundary - ray.org) * ray.inv_dir;
}

/// march a ray through the world using the dda algorithm.
fn dda_march(ray: Ray, config: RayMarchConfig) -> HitInfo {
    var curr_step = init_dda(ray);

    var hit_info: HitInfo;
    hit_info.did_hit = false;

    for (var i = 0u; i < config.max_iter; i++) {
        if (curr_step.t > config.max_t) {
            hit_info.t = config.max_t;
            break; 
        }

        let voxel_pos = curr_step.pos + curr_step.cam_int;
        let block_id = get_block_at(voxel_pos);

        // voxel is solid, register a hit detection
        if (block_id > AIR_VOXEL) {
            hit_info.did_hit = true;

            hit_info.hit_pos = ray.org + ray.dir * curr_step.t;

            hit_info.normal = -LOCAL_AXIS[curr_step.lsh] * ray.sign;
            hit_info.mat_id = u32(block_id);

            break;
        }

        let scale_idx = select(0u, u32(-block_id), block_id <= AIR_BRICK4);
        dda_step(ray, &curr_step, voxel_pos, SCALES[scale_idx]);

        hit_info.steps += 1;
        hit_info.t = curr_step.t;
    }

    return hit_info;
}

// transform the shader invocation id into the ray used for rendering
fn init_ray(id: vec3<u32>, size: vec2<u32>) -> Ray {
    let pix_id = vec2f(id.xy);
    let pix_center = pix_id + vec2f(0.5);
    let uv = ((pix_center / vec2f(size) * 2.0) - 1.0) * vec2f(1.0, -1.0);

    let clip = vec4f(uv.x, uv.y, 1.0, 1.0);
    let unproj = camera.inv_view_proj * clip;
    let world_target = unproj.xyz / unproj.w;

    let base_org = fract(camera.position);
    let base_dir = normalize(world_target - camera.position);

    let aperture = 0.02;
    let focal_dist = 10.0;
    return gen_perturbed_ray(pix_id, base_org, base_dir, aperture, focal_dist);
    // return create_ray(base_org, base_dir);
}

fn map_normal(n: vec3i) -> vec4f {
    let x = f32(n.x) * 0.5 + 0.5;
    let y = f32(n.y) * 0.5 + 0.5;
    let z = f32(n.z) * 0.5 + 0.5;

    return vec4f(x, y, z, 0.0);
}

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_size = textureDimensions(positions);

    if (id.x >= tex_size.x || id.y >= tex_size.y) {
        return;
    }

    let max_dist = 300.0;
    let max_steps = 300u;
    var config = RayMarchConfig(max_steps, max_dist);

    let ray = init_ray(id, tex_size);
    let hit_info = dda_march(ray, config);
    
    let d: f32 = clamp(hit_info.t / max_dist, 0.0, 1.0);
    let p: vec4f = select(vec4f(0.0), vec4f(hit_info.hit_pos, 0.0), hit_info.did_hit); 
    let n: vec4f = select(vec4f(0u), map_normal(hit_info.normal), hit_info.did_hit);
    let m: f32 = select(0.0, f32(hit_info.mat_id), hit_info.did_hit);

    textureStore(depth, id.xy, vec4f(d, 0.0, 0.0, 0.0));
    textureStore(normals, id.xy, n);
    textureStore(positions, id.xy, p);
    textureStore(material, id.xy, vec4f(m, 0.0, 0.0, 0.0));
}