/// Performs voxel raymarching into a scene using the provided camera and voxel data.
/// Saves the final colors into a storage texture.

const REGION_SIZE: i32 = 32;
const REGION_VOL: u32 = u32(REGION_SIZE * REGION_SIZE * REGION_SIZE);

const AIR_BRICK4: i32 = -1;     // indicates that a grid4 brick is completely air
const AIR_BRICK8: i32 = -2;     // indicates that a grid8 brick is completely air
const AIR_BRICK16: i32 = -3;    // indicates that a grid16 brick is completely air
const AIR_REGION: i32 = -4;     // indicates that a full region is completely air

const AIR_VOXEL: i32 = 0; // indicates that a voxel is an air block.
const GRASS_VOXEL: i32 = 2;
const WATER_VOXEL: i32 = 4;

const SCALES = array<f32, 5>(1.0, 4.0, 8.0, 16.0, 32.0);

const RAY_DIR_OFFSET: f32 = 0.1;
const RAY_ORG_OFFSET: f32 = 0.002;

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
    ao_params: vec3f,
}

struct RegionGrids {
    grid4: array<u32, 16>,  // 4x4x4 voxel bricks, 512 per region
    grid8: array<u32, 2>,   // 8x8x8 voxel bricks, 64 per region
    grid16: u32             // 16x16x16 bricks, 8 per region
}

/// Global uniforms (updated once per frame)
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> env: EnvironmentUniform;

/// Material related bindings (static)
@group(1) @binding(0) var grass_alpha_mask: texture_2d<f32>;
@group(1) @binding(1) var block_atlas: texture_2d<f32>;
@group(1) @binding(2) var block_sampler: sampler;
@group(1) @binding(3) var<uniform> atlas_uvs: array<vec4f, 5>;

/// voxel data (updated dynamically based on player interaction)
@group(2) @binding(0) var<storage, read> voxels: array<u32>;
@group(2) @binding(1) var<storage, read> grids: array<RegionGrids>;

/// screen-space textures
@group(3) @binding(0) var positions: texture_2d<f32>;
@group(3) @binding(1) var normals: texture_2d<f32>;
@group(3) @binding(2) var depth: texture_2d<f32>;
@group(3) @binding(3) var material: texture_2d<f32>;
@group(3) @binding(4) var output: texture_storage_2d<rgba16float, write>;

/// describes the properties voxel that was hit by a ray
struct VoxelHit {
    mat_id: u32,
    coords: vec3i,
    position: vec3f,
    face: VoxelFace,
}

/// Describes the voxel face that way hit by a ray
struct VoxelFace {
    uv: vec2f,
    normal: vec3i,
    tan1: vec3i,
    tan2: vec3i,
}

/// Describes the ray marching result after traversal
struct RayMarchResult {
    did_hit: bool,
    hit_info: VoxelHit,
    t: f32,
    steps: u32,
}

/// Describes the configuration state of a DDA traversal
struct RayMarchConfig {
    max_iter: u32,
    max_t: f32,
    is_shadow: bool,
}

/// The current state of a DDA traversal
struct DdaState {
    cam_int: vec3i,
    pos: vec3i,
    t: f32,
    dist: vec3f,
    lsh: u32,
    scale_idx: u32,
}

/// Initializes a DDA state for use in traversal
fn init_dda(ray: Ray) -> DdaState {
    var state: DdaState;
    state.lsh = 0u;
    state.scale_idx = 0u;
    state.t = ray.t;
    state.cam_int = vec3i(floor(camera.position));
    state.pos = vec3i(floor(ray.org));
    state.dist = (vec3f(state.pos + ray.pos_sign) - ray.org) * ray.inv_dir;
    
    return state;
}

/// Holds information about a ray that was cast
struct Ray {
    org: vec3f,
    dir: vec3f,
    inv_dir: vec3f,
    sign: vec3i,
    pos_sign: vec3i,
    t: f32,
}

/// creates a ray given the origin and direction
fn create_ray(org: vec3f, dir: vec3f, t: f32) -> Ray {
    var ray: Ray;
    ray.org = org;
    ray.dir = normalize(dir);
    ray.t = t;
    ray.inv_dir = 1.0 / ray.dir;
    ray.sign = vec3i(sign(dir));
    ray.pos_sign = max(ray.sign, vec3i(0));

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

    return create_ray(ray_org, ray_dir, 0.0);
}

/// calculates voxel face properties based on the face normal and ray result position
fn calc_face(hit_pos: vec3f, normal: vec3i) -> VoxelFace {
    var face: VoxelFace;
    face.normal = normal;

    let fract_pos = fract(hit_pos);
    let abs_n = abs(normal);

    face.tan1 = select(LOCAL_AXIS[0], LOCAL_AXIS[2], abs_n.x == 1);
    face.tan2 = select(LOCAL_AXIS[1], LOCAL_AXIS[2], abs_n.y == 1);

    let uv_y = select(vec2f(fract_pos.x, 1.0 - fract_pos.z), fract_pos.xz, normal.y > 0);
    let uv_x = select(vec2f(1.0 - fract_pos.z, fract_pos.y), fract_pos.zy, normal.x > 0);
    let uv_z = select(fract_pos.xy, vec2f(1.0 - fract_pos.x, fract_pos.y), normal.z > 0);

    face.uv = select(
        select(uv_z, uv_x, abs_n.x == 1),
        uv_y,
        abs_n.y == 1
    );
    return face;
}

/// calculates lighting with the blinn-phong lighting model as the base
fn calc_lighting(hit: VoxelHit, sun_dir: vec3f, view_dir: vec3f) -> vec3f {
    // shadow
    let normal = vec3f(hit.face.normal);
    let shadow = calc_shadow(hit.position, normal, sun_dir);

    // ambient term
    let ao = calc_ao(hit.face, hit.position);
    let amb_strength = clamp(sun_dir.y * 0.5 + 0.5, 0.1, 1.0);
    let ambient = (env.sky_zenith.xyz * amb_strength + 0.03) * ao;

    // diffuse term
    let diff_strength = max(dot(normal, sun_dir), 0.0);
    let diffuse = env.sun_color.xyz * diff_strength * ao * shadow;

    // specular term
    let half = normalize(sun_dir + view_dir);
    let spec_strength = pow(max(dot(normal, half), 0.0), 1024.0);
    let specular = env.sun_color.xyz * spec_strength * shadow;

    let base_color = get_palette_color(hit.face, hit.mat_id);
    return (ambient + diffuse + specular) * base_color;
}

/// calculates a semi-soft shadow by tracing a set of shadow rays by perturbing their origin
fn calc_shadow(start_pos: vec3f, normal: vec3f, light_dir: vec3f) -> f32 {
    let config = RayMarchConfig(12, 20.0, true);

    let shadow_radius = 0.0001;
    let light_dist = 0.01;
    let samples = 1.0;

    var in_shadow = 0.0;
    let ray_org = start_pos + normal * shadow_radius;
    for (var i = 0u; i < u32(samples); i++) {
        let jitter = vec2f(f32(i) * 45.12, f32(i) * 89.43);
        let seed = ray_org.xy + ray_org.zz + jitter;

        let sample_ray = gen_perturbed_ray(seed, ray_org, light_dir, shadow_radius, light_dist);
        let shadow_result = dda_march(sample_ray, config);

        in_shadow += select(1.0, 0.0, shadow_result.did_hit);
    }

    return in_shadow / samples;
}

/// calculates ambient occlusion by sampling the 3x3 grid of voxels in front of the occluded voxel.
fn calc_ao(face: VoxelFace, hit_pos: vec3f) -> f32 {
    let fract_pos = fract(hit_pos);
    let face_neighbor = vec3i(floor(hit_pos) + floor(camera.position)) + face.normal;

    let uv = select(
        select(fract_pos.xy, fract_pos.zy, abs(face.normal.x) == 1),
        fract_pos.xz,
        abs(face.normal.y) == 1
    );

    let l = (1.0 - uv.x);
    let r = uv.x;
    let b = (1.0 - uv.y);
    let t = uv.y;
    
    /// The weights determine the influence of the neighbor voxels based on the uv coordinates.
    /// the closer the hit point is to a neighbor, the greater the neighbor affects the ao.
    let weights = array<f32, 9>(
        l*b, b,   r*b,      // bottom neighbors
        l,   0.0, r,        // middle neighbors, center block is always empty
        l*t, t,   r*t       // top neighbors
    );

    let side_t = get_block_at(face_neighbor + face.tan2) > AIR_VOXEL;
    let side_l = get_block_at(face_neighbor - face.tan1) > AIR_VOXEL;
    let side_r = get_block_at(face_neighbor + face.tan1) > AIR_VOXEL;
    let side_b = get_block_at(face_neighbor - face.tan2) > AIR_VOXEL;

    let bl = face_neighbor - face.tan1 - face.tan2;
    let br = face_neighbor + face.tan1 - face.tan2;
    let tl = face_neighbor - face.tan1 + face.tan2;
    let tr = face_neighbor + face.tan1 + face.tan2;

    // We only want to include a neighbor if it's either an occupied side, 
    // or an occupied corner with at least one adjacent side NOT occupied
    let include = array<bool, 9>(
        !(side_b && side_l) && get_block_at(bl) > AIR_VOXEL,
        side_b,
        !(side_b && side_r) && get_block_at(br) > AIR_VOXEL,
        side_l,
        false, // center, has no weight
        side_r,
        !(side_t && side_l) && get_block_at(tl) > AIR_VOXEL,
        side_t,
        !(side_t && side_r) && get_block_at(tr) > AIR_VOXEL,
    );

    var weight_sum = 0.0;
    for (var i=0; i<9; i++) {
        weight_sum += select(0.0, weights[i], include[i]);
    }

    let intensity = 0.5;
    let contrast = 1.4;
    let floor = 0.05;

    // smoothstep tends to look best with this algorithm
    let mapped = smoothstep(0.0, contrast, weight_sum * intensity);
    let ao = clamp(1.0 - (mapped * (1.0 - floor)), floor, 1.0);
    return ao;
}

/// calculates a background color given environmental variables
fn get_background_color(ray_dir: vec3f, sun_dir: vec3f) -> vec3<f32> {
    let y = ray_dir.y;

    let horizon_line = -0.05;
    let sky_t = pow(smoothstep(horizon_line, 0.1, y), 0.35);
    let sky_grad = mix(env.sky_horizon, env.sky_zenith, sky_t);

    let ground_sky_t = smoothstep(-0.08, horizon_line, y);
    var bg_color = mix(env.ground_color, sky_grad, ground_sky_t).xyz;

    let align = max(dot(ray_dir, sun_dir), 0.0);
    let mask = smoothstep(-0.1, 0.1, y);
    let corona = pow(align, 16.0) * 0.2;
    let disk = pow(align, 2000.0) * 2.0;

    let sun_factor = (corona + disk) * mask * env.sun_color.w;
    bg_color += env.sun_color.xyz * sun_factor;

    return saturate(bg_color);
}

fn get_palette_color(face: VoxelFace, uv_idx: u32) -> vec3f {
    let uv_bounds = atlas_uvs[uv_idx];

    let pad = 4.0 / 1024.0;
    let width = (uv_bounds.z - uv_bounds.x); // max_x - min_x

    var is_side = abs(face.normal.y) != 1;
    let offsets = array<f32, 3>(2.0, 1.0, 0.0);
    let offset = offsets[face.normal.y + 1];

    let x_offset = (width + pad) * offset;
    let auv_bounds = vec4f(
        uv_bounds.x + x_offset,
        uv_bounds.y,
        uv_bounds.z + x_offset,
        uv_bounds.w
    );

    let inv_face_uv = vec2f(face.uv.x, 1.0-face.uv.y);
    let uv = mix(auv_bounds.xy, auv_bounds.zw, inv_face_uv);
    let texel = textureSampleLevel(block_atlas, block_sampler, uv, 0.0);

    var color_mask = vec3f(1.0);
    if (i32(uv_idx) == GRASS_VOXEL) {
        var grass_color = vec3f(0.0, 1.0, 0.0);
        if (is_side) {
            let grass_overlay = textureSampleLevel(grass_alpha_mask, block_sampler, inv_face_uv, 0.0);
            grass_color += grass_overlay.rgb;
            color_mask = mix(vec3f(1.0), grass_color, grass_overlay.a);
        } else {
            color_mask = grass_color;
        }
    } else if (i32(uv_idx) == WATER_VOXEL) {
        color_mask = vec3f(0.0, 0.3, 0.7);
    }

    return texel.rgb * color_mask;
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
fn dda_march(ray: Ray, config: RayMarchConfig) -> RayMarchResult {
    var curr_step = init_dda(ray);

    var result: RayMarchResult;
    result.did_hit = false;

    for (var i = 0u; i < config.max_iter; i++) {
        if (curr_step.t > config.max_t) {
            result.t = config.max_t;
            break; 
        }

        let voxel_pos = curr_step.pos + curr_step.cam_int;
        let block_id = get_block_at(voxel_pos);

        // voxel is solid, register a result detection
        if (block_id > AIR_VOXEL) {
            result.did_hit = true;
            
            if (config.is_shadow) { break; }

            let normal = -LOCAL_AXIS[curr_step.lsh] * ray.sign;

            var vox_hit: VoxelHit;
            vox_hit.position = ray.org + ray.dir * curr_step.t;
            vox_hit.mat_id = u32(block_id);
            vox_hit.face = calc_face(vox_hit.position, normal);
            result.hit_info = vox_hit;

            break;
        }

        let scale_idx = select(0u, u32(-block_id), block_id <= AIR_BRICK4);
        dda_step(ray, &curr_step, voxel_pos, SCALES[scale_idx]);

        result.steps += 1;
        result.t = curr_step.t;
    }

    return result;
}

fn map_normal(coarse_norm: vec3f) -> vec3i {
    let x = i32((coarse_norm.x - 0.5) * 2);
    let y = i32((coarse_norm.y - 0.5) * 2);
    let z = i32((coarse_norm.z - 0.5) * 2);

    return vec3i(x, y, z);
}

// transform the shader invocation id into the ray used for rendering
fn init_ray(id: vec3<u32>, size: vec2<u32>, coarse_pos: vec3f, coarse_t: f32) -> Ray {
    let pix_id = vec2f(id.xy);
    let pix_center = pix_id + vec2f(0.5);
    let uv = ((pix_center / vec2f(size) * 2.0) - 1.0) * vec2f(1.0, -1.0);

    let clip = vec4f(uv.x, uv.y, 1.0, 1.0);
    let unproj = camera.inv_view_proj * clip;
    let world_target = unproj.xyz / unproj.w;

    let base_dir = normalize(world_target - camera.position);

    let sample = random_point(pix_id);
    let angle = sample.x * 6.2831853;
    let radius = sample.y * RAY_ORG_OFFSET;

    let arb_vec = select(vec3f(0.0, 1.0, 0.0), vec3f(1.0, 0.0, 0.0), abs(base_dir.y) > 0.99);
    let right = normalize(cross(arb_vec, base_dir));
    let up = cross(base_dir, right);
    let pos_offset = (right * cos(angle) + up * sin(angle)) * radius;

    let base_org = coarse_pos - base_dir * RAY_DIR_OFFSET + pos_offset;
    return create_ray(base_org, base_dir, coarse_t - RAY_DIR_OFFSET);
}

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_size = textureDimensions(output);

    if (id.x >= tex_size.x || id.y >= tex_size.y) {
        return;
    }

    let coarse_coord = id.xy / 2u;
    let coarse_depth = textureLoad(depth, coarse_coord, 0).x;
    let coarse_norm = textureLoad(normals, coarse_coord, 0).xyz;
    let coarse_pos = textureLoad(positions, coarse_coord, 0).xyz;
    let coarse_mat = u32(textureLoad(material, coarse_coord, 0).x);

    let ray = init_ray(id, tex_size, coarse_pos, coarse_depth);

    let mapped_norm = map_normal(coarse_norm);
    let face = calc_face(coarse_pos, mapped_norm);

    let sun_dir = normalize(env.sun_dir.xyz);
    var color = vec3f(0.0);
    if (coarse_mat > u32(AIR_VOXEL))  {
        let dda_config = RayMarchConfig(4u, 20, false);

        let hit_result = dda_march(ray, dda_config);

        if hit_result.did_hit {
            color = calc_lighting(hit_result.hit_info, sun_dir, -ray.dir);
        } else {
            color = get_background_color(ray.dir, sun_dir);
        }
    } else {
        color = get_background_color(ray.dir, sun_dir);
    }

    var fog_factor = 1.0 - exp(-coarse_depth * 0.06);
    let result_height = (camera.position + ray.dir * coarse_depth).y;
    let height_falloff = saturate(1.0 - abs(ray.dir.y) * 4.0);

    fog_factor = saturate(fog_factor * height_falloff);
    color = mix(color, env.sky_horizon.xyz, fog_factor);

    let out_color = vec4f(color, coarse_depth);
    textureStore(output, id.xy, out_color);
}