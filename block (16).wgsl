// Glow: the bright parts of the layer, blurred at half size and added back.
//
// bright  (layer -> half, half size): keeps the light above the threshold.
//         The test is on the straight (not premultiplied) brightness, with
//         a soft knee 0.1 wide, so threshold 1 still lets pure white glow;
//         what is kept is premultiplied, so a faded layer glows less.
// blur_h  (half -> tmp) and blur_v (tmp -> soft): a separable Gaussian, as
//         in the blur block, with the radius in sequence pixels.
// combine (layer and soft -> result): the layer plus the glow times the
//         intensity. Colour may go above 1 (glow is light); alpha is capped
//         at 1 so the glow never darkens what is beneath.

// The glow's blur reads every texel of its reach, two per sample, up to
// REACH texels each side (as in the blur block): radius 240 at the
// sequence's size, so the whole range of its radius in any export up to 1.2
// times the sequence's width. Beyond that its samples stand for evenly
// spaced steps, and very fine bright detail glows a little unevenly. At most
// REACH / 2 = 60 samples each side: a whole pass stays under the loop budget.
const REACH: f32 = 120.0;

fn bright(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let c = vep_sample(0u, uv);
    if (c.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    let luma = vep_luma(c.rgb / c.a);
    return c * smoothstep(p.threshold - 0.1, p.threshold, luma);
}

// The Gaussian of `sigma` texels along `dir`, out to `reach` texels each
// side, in steps of at least `least` texels (the blur block's, unchanged).
fn gaussian(dir: vec2<f32>, uv: vec2<f32>, sigma: f32, reach: f32, least: f32) -> vec4<f32> {
    let centre = vep_sample(0u, uv);
    if (reach < 0.5) {
        return centre;
    }
    let size = max(vep_input_size(0u), vec2<f32>(1.0));
    let step = max(reach / REACH, least);
    // Steps 1 to `steps` each side: the first `paired` two at a time, an odd
    // last one alone.
    let steps = min(ceil(reach / step), REACH);
    let paired = 2.0 * floor(steps / 2.0);
    let along = dir * step / size;
    let k = -0.5 * step * step / (sigma * sigma);
    var sum = centre;
    var side = 0.0;
    for (var near = 1.0; near < paired; near += 2.0) {
        let first = exp(k * near * near);
        let far = near + 1.0;
        let second = exp(k * far * far);
        let pair = first + second;
        let offset = along * (near + second / pair);
        sum += (vep_sample(0u, uv + offset) + vep_sample(0u, uv - offset)) * pair;
        side += pair;
    }
    if (paired < steps) {
        let last = exp(k * steps * steps);
        let offset = along * steps;
        sum += (vep_sample(0u, uv + offset) + vep_sample(0u, uv - offset)) * last;
        side += last;
    }
    return sum / (1.0 + 2.0 * side);
}

fn blur_h(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let texels = p.radius * vep_input_size(0u).x / max(ctx.seq_size.x, 1.0);
    return gaussian(vec2<f32>(1.0, 0.0), uv, texels / 3.0, texels, 1.0);
}

fn blur_v(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let texels = p.radius * vep_input_size(0u).y / max(ctx.seq_size.y, 1.0);
    return gaussian(vec2<f32>(0.0, 1.0), uv, texels / 3.0, texels, 1.0);
}

fn combine(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let layer = vep_sample(0u, uv);
    let glow = vep_sample(1u, uv) * p.intensity;
    return vec4<f32>(layer.rgb + glow.rgb, min(layer.a + glow.a, 1.0));
}
