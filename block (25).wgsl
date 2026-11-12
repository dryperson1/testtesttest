// Solid colour: every pixel of the frame is the colour parameter.
//
// The colour is given as footage carries it (sRGB values, straight alpha);
// the host decodes it to linear light like any picture, exactly as today's
// solid layers (crates/render/shaders/layer.wgsl, the `solid` uniform).

fn source(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    return p.color;
}
