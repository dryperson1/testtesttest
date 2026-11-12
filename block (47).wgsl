// Warm tint: moves every pixel's colour towards the colour multiplied by a
// tint.
//
// A pixel stage gets one pixel at a time, in linear light with straight
// (not premultiplied) alpha, and returns it changed. `p` holds the block's
// parameters (the manifest's "params") by their ids: `p.amount` is an f32
// and `p.tint` a vec4<f32> (r, g, b, a as stored). The editor has already
// evaluated any keyframes at the frame's time.
//
// The tint is stored as people pick colours (sRGB values, like footage), so
// it is turned into linear light with the editor's own vep_srgb_to_linear
// before it multiplies the picture. At amount 0 the pixel comes back
// exactly as it was; a white tint changes nothing either.

fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let tint = vep_srgb_to_linear(p.tint.rgb);
    let amount = clamp(p.amount, 0.0, 1.0);
    return vec4<f32>(mix(c.rgb, c.rgb * tint, amount), c.a);
}
