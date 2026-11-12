// Mask: multiplies the layer's alpha by a shape.
//
// mask_value() is today's, statement for statement (crates/render/shaders/
// layer.wgsl), with its quirks kept so old projects look the same:
// - a path arrives as its bounding rectangle (kind 0), and a rectangle's
//   corner radius is ignored;
// - the shape's rectangle is `shape.bounds`, computed by the host in f64
//   exactly as today (an ellipse is bounded by its centre minus and plus
//   its radii);
// - the feather, in pixels, is divided by the layer WIDTH for both axes and
//   centred on the edge;
// - the ellipse's distance is an approximation (one minus the normalised
//   radius, times the larger radius).
// The layer's alpha arrives already multiplied by the layer opacity, so the
// product is today's `src.a * opacity * mask`.

fn mask_value(uv: vec2<f32>, ctx: VepCtx, p: Params) -> f32 {
    let rect = p.shape.bounds;
    let feather = p.feather / max(ctx.layer_size.x, 1.0);
    var m: f32;
    if (p.shape.kind != 1u) {
        let d = min(min(uv.x - rect.x, rect.x + rect.z - uv.x), min(uv.y - rect.y, rect.y + rect.w - uv.y));
        m = clamp(d / max(feather, 1e-5) + 0.5, 0.0, 1.0);
    } else {
        let centre = vec2<f32>(rect.x + rect.z * 0.5, rect.y + rect.w * 0.5);
        let radii = max(vec2<f32>(rect.z * 0.5, rect.w * 0.5), vec2<f32>(1e-5));
        let q = (uv - centre) / radii;
        let dist = (1.0 - length(q)) * max(radii.x, radii.y);
        m = clamp(dist / max(feather, 1e-5) + 0.5, 0.0, 1.0);
    }
    if (p.invert != 0u) {
        m = 1.0 - m;
    }
    return mix(1.0, m, p.opacity);
}

fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return vec4<f32>(c.rgb, c.a * mask_value(uv, ctx, p));
}
