struct CameraUniform {
    prev_vp: mat4x4f,
    curr_vp_inv: mat4x4f,
}

@group(0) @binding(0) var<uniform> cam_mats: CameraUniform;
@group(0) @binding(1) var current: texture_2d<f32>;
@group(0) @binding(2) var history: texture_2d<f32>;
@group(0) @binding(3) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var samp: sampler;

fn calc_history_uv(dims: vec2u, pixel: vec2u, depth: f32) -> vec2f {
    let uv = (vec2f(pixel) + 0.5) / vec2f(dims);
    let ndc = vec3f(
        uv.x * 2.0 - 1.0, 
        (1.0-uv.y) * 2.0 - 1.0, 
        depth
    );

    let world_pos4 = cam_mats.curr_vp_inv * vec4f(ndc, 1.0);
    let world_pos = world_pos4.xyz / world_pos4.w;

    let prev_clip = cam_mats.prev_vp * vec4f(world_pos, 1.0);
    let prev_ndc = prev_clip.xyz / prev_clip.w;

    return vec2f(prev_ndc.x * 0.5 + 0.5, 1.0 - (prev_ndc.y * 0.5 + 0.5));
}

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>)  {
    let dims = textureDimensions(current);
    let pixel = id.xy;

    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }
    
    let curr_texel = textureLoad(current, pixel, 0);
    let depth = curr_texel.a;

    if (depth >= 1.5) {
        textureStore(output, pixel, curr_texel);
        return;
    }

    let history_uv = calc_history_uv(dims, pixel, depth);
    let hist_texel = textureSampleLevel(history, samp, history_uv, 0.0);
    
    var m1 = vec3f(0.0);
    var m2 = vec3f(0.0);

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let neighbor = textureLoad(current, vec2i(pixel) + vec2i(x, y), 0).rgb;
            m1 += neighbor;
            m2 += neighbor * neighbor;
        }
    }

    let samples = 9.0;
    let mean = m1 / samples;
    let variance = (m2 / samples) - (mean * mean);
    let std_dev = sqrt(max(variance, vec3f(0.0)));

    let gamma = 0.99;
    let b_min = mean - gamma * std_dev;
    let b_max = mean + gamma * std_dev;
    var clamped = clamp(hist_texel.rgb, b_min, b_max);

    var blend = 0.01;
    if (any(history_uv < vec2f(0.0)) || any(history_uv > vec2f(1.0))) {
        blend = 1.0;
    }

    let accumulated = mix(clamped.rgb, curr_texel.rgb, blend);

    let out_color = vec4(accumulated, curr_texel.a);
    textureStore(output, pixel, out_color);
}