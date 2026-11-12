// Blur: a separable Gaussian, in four passes.
//
// blur_h (layer -> a) and blur_v (a -> b) blur by up to REACH texels exactly:
// every texel within the reach is read, two at a time, with one bilinear
// sample placed between the two so that it weighs them as the Gaussian does.
// wide_h (b -> c) and wide_v (c -> result) add the rest of a blur that
// reaches farther, and only copy for any other. Gaussians add up (the
// squares of their sigmas do), and what reaches the wide passes has already
// been blurred by a sigma of REACH / 3 texels, so their samples can be up to
// one and a half times that far apart and leave less than a tenth of a level
// of any fine detail behind. Each of their samples stands for two steps of
// at least an eighth of that sigma (16 samples a side at radius 200 in a
// full HD export), close enough that standing for two is off by less than a
// tenth of a level, and they take at most 60 a side, which keeps all this
// true up to a reach of about 3600 texels (radius 200 in an export eighteen
// times the sequence's width).
//
// The passes run in sequence space on the layer as drawn (premultiplied, so
// transparent surroundings do not darken the edges), at output size. The
// radius is in sequence pixels and is turned into texels of the input with
// the input's size, so the blur looks the same in a reduced preview and in
// the export. The Gaussian's sigma is a third of the radius, so the radius
// is how far the blur visibly spreads.
//
// Each side of the centre takes at most REACH / 2 = 60 bilinear samples: a
// whole pass stays under the loop budget (tested), so no radius is ever cut
// short.

// The texels the sharp passes read, each side: two for each sample.
const REACH: f32 = 120.0;

// The Gaussian of `sigma` texels along `dir`, out to `reach` texels each side,
// in steps of at least `least` texels. In steps of one texel up to REACH
// texels, every texel is read, two per sample; otherwise the samples stand
// for evenly spaced steps, which is only right for a picture with no detail
// finer than the steps (the wide passes).
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

// The radius in texels of the input, along x or y.
fn texels(p: Params, ctx: VepCtx, along: u32) -> f32 {
    let size = vep_input_size(0u);
    if (along == 0u) {
        return p.radius * size.x / max(ctx.seq_size.x, 1.0);
    }
    return p.radius * size.y / max(ctx.seq_size.y, 1.0);
}

// The sharp passes: the whole blur up to REACH texels, its first REACH beyond.
fn sharp(dir: vec2<f32>, uv: vec2<f32>, reach: f32) -> vec4<f32> {
    let near = min(reach, REACH);
    return gaussian(dir, uv, near / 3.0, near, 1.0);
}

// The wide passes: what the sharp passes left of a blur beyond REACH texels,
// in steps of at least an eighth of the sharp passes' sigma.
fn wide(dir: vec2<f32>, uv: vec2<f32>, reach: f32) -> vec4<f32> {
    if (reach <= REACH) {
        return vep_sample(0u, uv);
    }
    let sigma = sqrt(reach * reach - REACH * REACH) / 3.0;
    return gaussian(dir, uv, sigma, 3.0 * sigma, REACH / 24.0);
}

fn blur_h(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return sharp(vec2<f32>(1.0, 0.0), uv, texels(p, ctx, 0u));
}

fn blur_v(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return sharp(vec2<f32>(0.0, 1.0), uv, texels(p, ctx, 1u));
}

fn wide_h(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return wide(vec2<f32>(1.0, 0.0), uv, texels(p, ctx, 0u));
}

fn wide_v(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return wide(vec2<f32>(0.0, 1.0), uv, texels(p, ctx, 1u));
}
