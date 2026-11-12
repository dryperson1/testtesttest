// Gradient: every pixel blends the two colours by its place in the frame.
//
// A source stage makes a picture from nothing. It gets the place in the
// layer (`uv`: 0, 0 top left, 1, 1 bottom right; a source's layer is the
// whole frame) and returns the colour ENCODED as footage is (sRGB values)
// with straight alpha; the editor turns it into linear light as it does
// any picture. So the blend here is in sRGB values, the way most drawing
// programs blend.
//
// A switch (bool) arrives as a u32, 0 or 1.

fn source(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    var t = uv.y;
    if (p.horizontal != 0u) {
        t = uv.x;
    }
    return mix(p.start_color, p.end_color, clamp(t, 0.0, 1.0));
}
