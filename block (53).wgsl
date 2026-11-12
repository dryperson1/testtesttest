// Iris: the incoming clip (b) appears inside a circle that grows from
// `center` over the outgoing clip (a).
//
// A transition stage gets one pixel of each clip at the same place, already
// drawn with their own effects, in linear light and PREMULTIPLIED (the
// composite's form), and returns the mix. `uv` is the place in the frame (0, 0
// top left, 1, 1 bottom right) and `ctx.progress` runs from 0 at the start of
// the transition to 1 at its end. Distances are measured in sequence pixels,
// so the circle is round on any frame shape.
//
// The circle's radius grows from 0 to the distance of the corner farthest
// from the centre plus the soft edge, so at progress 1 it covers everything.
// At progress 0 and 1 the result is exactly one clip.

fn transition(a: vec4<f32>, b: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    if (ctx.progress <= 0.0) {
        return a;
    }
    if (ctx.progress >= 1.0) {
        return b;
    }
    let size = max(ctx.seq_size, vec2<f32>(1.0));
    let centre = clamp(p.center, vec2<f32>(0.0), vec2<f32>(1.0)) * size;
    let far = max(max(length(centre), length(vec2<f32>(size.x, 0.0) - centre)), max(length(vec2<f32>(0.0, size.y) - centre), length(size - centre)));
    let soft = clamp(p.softness, 0.0, 0.5) * max(size.x, size.y);
    let radius = ctx.progress * (far + soft);
    let inside = clamp((radius - distance(uv * size, centre)) / max(soft, 1e-5), 0.0, 1.0);
    return mix(a, b, inside);
}
