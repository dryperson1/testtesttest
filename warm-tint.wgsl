/* chadfx-block
{
  "manifest": 1,
  "id": "example.warm-one-file",
  "version": "1.0.0",
  "format": 1,
  "name": "Warm tint in one file",
  "description": "Tints the picture towards a colour, warm by default. At full amount every colour is multiplied by the tint, so white becomes the tint itself.",
  "category": "colour",
  "kind": "video.filter",
  "params": [
    {
      "id": "amount",
      "type": "float",
      "default": 0.5,
      "min": 0.0,
      "max": 1.0,
      "step": 0.01,
      "unit": "fraction",
      "keyframable": true,
      "label": "Amount",
      "description": "How far the picture moves towards the tint: 0 leaves it exactly as it is, 1 multiplies it by the tint."
    },
    {
      "id": "tint",
      "type": "color",
      "default": { "r": 1.0, "g": 0.8, "b": 0.6, "a": 1.0 },
      "min": 0.0,
      "max": 1.0,
      "keyframable": true,
      "label": "Tint",
      "description": "The colour the picture is multiplied by, as it is picked (sRGB values from 0 to 1). Its alpha is not used."
    }
  ],
  "video": {
    "stages": ["pixel"]
  },
  "author": "Example Author",
  "license": "MIT OR Apache-2.0"
}
*/

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
