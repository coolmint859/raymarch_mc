@group(0) @binding(0) var current: texture_2d<f32>;
@group(0) @binding(1) var history: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>)  {
    let pixel = id.xy;
    let hist_texel = textureLoad(history, pixel, 0);

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

    let gamma = 0.9;
    let b_min = mean - gamma * std_dev;
    let b_max = mean + gamma * std_dev;

    let blend = 0.9;
    var clipped = clamp(hist_texel.rgb, b_min, b_max);

    let curr_texel = textureLoad(current, pixel, 0);
    let accumulated = mix(curr_texel.rgb, clipped, blend);

    let out_color = vec4(accumulated, curr_texel.a);
    textureStore(output, pixel, out_color);
}