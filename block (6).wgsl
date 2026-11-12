// Crop: the visible part of the layer, as x0, x1, y0, y1 in layer
// coordinates (0..1, y down).
//
// Today's maths (crates/render/shaders/layer.wgsl, vs_main): the left and
// top fractions are the first edges, one minus the right and bottom
// fractions the far edges. Like today the crop is of the whole layer: the
// region so far (`r`) is not used, and a crop whose edges cross (left +
// right > 1) flips the quad. `feather` is kept on old nodes but has never
// been drawn.

fn region(r: vec4<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return vec4<f32>(p.left, 1.0 - p.right, p.top, 1.0 - p.bottom);
}
