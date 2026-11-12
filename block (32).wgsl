// Wipe: the incoming clip (b) sweeps over the outgoing one (a) behind a
// straight, soft edge.
//
// The edge travels in the direction `angle` (degrees, clockwise on screen;
// 0 goes from left to right, 90 from top to bottom). `s` is a pixel's place
// along that direction, 0 at the frame's first corner and 1 at its last, in
// sequence pixels so the angle is true on any frame shape. The edge moves
// from 0 to 1 + softness as progress goes from 0 to 1; `softness` is the
// width of the soft edge as a share of the frame (0 gives a hard edge). At
// progress 0 and 1 the result is exactly one clip, also at the far corner
// and whatever the rounding. Colours are premultiplied, so a linear mix is
// correct for transparent clips too.

fn transition(a: vec4<f32>, b: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    if (ctx.progress <= 0.0) {
        return a;
    }
    if (ctx.progress >= 1.0) {
        return b;
    }
    let r = radians(p.angle);
    let dir = vec2<f32>(cos(r), sin(r));
    let size = max(ctx.seq_size, vec2<f32>(1.0));
    let pt = (uv - vec2<f32>(0.5)) * size;
    let extent = max(abs(dir.x) * size.x + abs(dir.y) * size.y, 1e-5);
    let s = dot(pt, dir) / extent + 0.5;
    let soft = max(p.softness, 0.0);
    let edge = ctx.progress * (1.0 + soft);
    let w = clamp((edge - s) / max(soft, 1e-5), 0.0, 1.0);
    return mix(a, b, w);
}
