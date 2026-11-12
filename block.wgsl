// Blend: how the layer (src) combines with the picture beneath it (dst), in
// linear light.
//
// Both colours are premultiplied. Every mode is the W3C formula
// ("Compositing and Blending Level 1": source-over with a blend function B
// of the straight colours Cs = src.rgb / src.a and Cd = dst.rgb / dst.a):
//
//   rgb = src.rgb * (1 - dst.a) + dst.rgb * (1 - src.a) + src.a * dst.a * B(Cs, Cd)
//   a   = src.a + dst.a - src.a * dst.a
//
// Where the layer is transparent the result is exactly what is beneath.
// The last term, `mixed` below, is computed from premultiplied values
// without dividing wherever B allows:
//
//   add         B = Cs + Cd            mixed = src * da + dst * sa  (so rgb = src + dst)
//   multiply    B = Cs * Cd            mixed = src * dst
//   screen      B = Cs + Cd - Cs * Cd  mixed = src * da + dst * sa - src * dst
//   overlay     B = hard_light(Cd, Cs)
//   soft_light  B = the W3C soft light (with its D(Cd) curve)
//   hard_light  B = Cs <= 0.5 ? 2 Cs Cd : screen(Cd, 2 Cs - 1)
//   darken      B = min(Cs, Cd)        mixed = min(src * da, dst * sa)
//   lighten     B = max(Cs, Cd)        mixed = max(src * da, dst * sa)
//   difference  B = |Cs - Cd|          mixed = |src * da - dst * sa|
//
// The formulas are defined for colours from 0 to 1; above 1 (light from a
// glow, for example) add, multiply, darken, lighten and difference still
// behave, the others are extrapolated. An unknown mode index blends
// normally (B = Cs).

fn straight(c: vec4<f32>) -> vec3<f32> {
    if (c.a <= 0.0) {
        return vec3<f32>(0.0);
    }
    return c.rgb / c.a;
}

fn hard_light(cs: vec3<f32>, cd: vec3<f32>) -> vec3<f32> {
    let low = 2.0 * cs * cd;
    let s = 2.0 * cs - vec3<f32>(1.0);
    let high = cd + s - cd * s;
    return select(high, low, cs <= vec3<f32>(0.5));
}

fn soft_light(cs: vec3<f32>, cd: vec3<f32>) -> vec3<f32> {
    let d = select(sqrt(max(cd, vec3<f32>(0.0))), ((16.0 * cd - vec3<f32>(12.0)) * cd + vec3<f32>(4.0)) * cd, cd <= vec3<f32>(0.25));
    let low = cd - (vec3<f32>(1.0) - 2.0 * cs) * cd * (vec3<f32>(1.0) - cd);
    let high = cd + (2.0 * cs - vec3<f32>(1.0)) * (d - cd);
    return select(high, low, cs <= vec3<f32>(0.5));
}

fn blend(src: vec4<f32>, dst: vec4<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let sa = src.a;
    let da = dst.a;
    let s = src.rgb;
    let d = dst.rgb;
    var mixed: vec3<f32>;
    switch p.mode {
        case 0u: {
            mixed = s * da + d * sa;
        }
        case 1u: {
            mixed = s * d;
        }
        case 2u: {
            mixed = s * da + d * sa - s * d;
        }
        case 3u: {
            mixed = sa * da * hard_light(straight(dst), straight(src));
        }
        case 4u: {
            mixed = sa * da * soft_light(straight(src), straight(dst));
        }
        case 5u: {
            mixed = sa * da * hard_light(straight(src), straight(dst));
        }
        case 6u: {
            mixed = min(s * da, d * sa);
        }
        case 7u: {
            mixed = max(s * da, d * sa);
        }
        case 8u: {
            mixed = abs(s * da - d * sa);
        }
        default: {
            mixed = s * da;
        }
    }
    let rgb = s * (1.0 - da) + d * (1.0 - sa) + mixed;
    return vec4<f32>(rgb, sa + da - sa * da);
}
