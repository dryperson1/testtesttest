// Color: exposure, gain and lift, gamma, contrast, temperature and tint,
// saturation, in linear light.
//
// grade() is today's, statement for statement (crates/render/shaders/
// layer.wgsl and output.wgsl): the same operations in the same order, with
// the parameters read from Params instead of today's uniform vectors. It
// runs even with neutral parameters, as today, because exp2 and pow are not
// exact. Values above 1 are kept: only the output pass clamps.
//
// All colour nodes of a stack fold into one run (fold "params"): exposure,
// temperature, tint and lift add; contrast, saturation, gamma and gain
// multiply, exactly as today's ColorParams::compose.

fn grade(rgb_in: vec3<f32>, p: Params) -> vec3<f32> {
    var rgb = rgb_in * exp2(p.exposure);
    rgb = max(rgb * p.gain + p.lift, vec3<f32>(0.0));
    rgb = pow(rgb, vec3<f32>(1.0) / max(p.gamma, vec3<f32>(0.01)));
    rgb = (rgb - vec3<f32>(0.18)) * p.contrast + vec3<f32>(0.18);
    rgb = rgb * vec3<f32>(1.0 + p.temperature / 200.0, 1.0 - p.tint / 200.0, 1.0 - p.temperature / 200.0);
    let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    rgb = vec3<f32>(luma) + (rgb - vec3<f32>(luma)) * p.saturation;
    return max(rgb, vec3<f32>(0.0));
}

fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return vec4<f32>(grade(c.rgb, p), c.a);
}
