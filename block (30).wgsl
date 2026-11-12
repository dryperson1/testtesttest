// Transform: places the layer's corners in the sequence.
//
// Today's vertex maths, statement for statement
// (crates/render/shaders/layer.wgsl, vs_main): the layer, measured from its
// anchor, is scaled, turned clockwise on screen (y points down), and its
// anchor is put at the sequence centre plus the position. The placement so
// far (`pos`) is not used: a transform places the whole layer. The layer's
// opacity is the host's (the `opacity` parameter has the opacity role).

fn place(pos: vec2<f32>, luv: vec2<f32>, ctx: VepCtx, p: Params) -> vec2<f32> {
    let q = (luv - p.anchor) * ctx.layer_size * p.scale;
    let r = radians(p.rotation);
    let rp = vec2<f32>(q.x * cos(r) - q.y * sin(r), q.x * sin(r) + q.y * cos(r));
    return rp + ctx.seq_size * 0.5 + p.position;
}
