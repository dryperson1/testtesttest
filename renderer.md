# How the renderer draws effects today, and what a plug-in shader runner must keep to match its pixels

**Short answer to question 6:** no. If each built-in effect became its own pass over `rgba16float` intermediates, the 8-bit output would change. On scaled high-detail pictures every pixel changes, by up to 72 levels. Even small restructurings change 1 level on many pixels. One design kept the bytes identical on every adapter I tested: generating **one fused fragment shader per layer** that keeps today's arithmetic, with today's vertex shader, fixed-function blending and output pass unchanged. Measurements are in §6.

Most of the content in §1–5 is read directly from the code. To answer §6 with evidence instead of guesses, I built a probe in my scratch folder, outside the project:
- `docs/blocks-rewrite\fxprobe\` contains `src\main.rs`, `src\variants.wgsl`, `src\out_load.wgsl`, and the raw results in `small.txt` and `big.txt`.
- It re-implements today's pipeline and matches the real `render` crate **byte for byte** on 15 small and 5 1080p scenes (NVIDIA Vulkan). It then runs about 17 restructured variants and counts differing bytes.
- I changed nothing in the project. Only build outputs were written to `target/`.

Versions and adapter (verified):
- `wgpu` 30.0.1, `naga` 30.0.1, `wgpu-core`/`wgpu-hal`/`wgpu-types` 30.0.1 (Cargo.lock:5236, 2532, 5266, 5327, 5393).
- `render` depends on `wgpu = "30"` with default features (crates/render/Cargo.toml:14). It does not use naga directly. naga comes in through wgpu-core with features `wgsl-in`, `spv-out`, `hlsl-out` and `glsl-out` (`cargo tree`).
- Adapter on this machine: **NVIDIA RTX A4000, Vulkan** (seen in a test run). The software fallback is **Microsoft Basic Render Driver (WARP), DX12, type Cpu**.

---

## 1. One frame, step by step

### 1.1 Entry points

| Entry | Lines | Does |
|---|---|---|
| `render` | renderer.rs:313-317 | `render_into`, then wraps the bytes in an `RgbaImage` |
| `render_into` | renderer.rs:326-335 | looks up the sequence (327), refuses size 0 (328-330), `layers_at(state, seq, time, source)` (331), `output_size` (332), `render_plan_into` (333) |
| `render_plan(_into)` | renderer.rs:338-357 | `abandon_in_flight`, then in `with_recovery`: `check`, `start`, `finish` |
| `render_ahead` | renderer.rs:376-387 | `layers_at` for this frame **first** (382), then `collect_into(out)` for the previous frame (384), then `submit_plan` (385) |
| `submit_plan` / `collect_into` | renderer.rs:395-403 / 408-430 | one frame in flight; the plan is kept in `InFlight` (180-191) so it can be redrawn after a device loss |

The output size is rounded up to even, minimum 2 (`output_size`, renderer.rs:300-310). The sequence size is used for layout and the output size only for rasterization: the vertex shader maps to NDC using `seq_size` (layer.wgsl:52), so a preview at a different size scales automatically.

### 1.2 `layers_at` (graph.rs:284-335): finding the layers, bottom first

1. **Solo and mute.** `soloed = any video track has solo` (285). A track is skipped if it is muted, or if something is soloed and this track is not (288-290).
2. **Track order.** Tracks come from `seq.video_tracks()` (287), which is `seq.tracks` filtered to Video (timeline.rs:442-444). Index 0 is drawn first, so it is the bottom layer (test render.rs:97-105).
3. **The clip at `t`.** The first clip whose `timeline_range.contains(t)` (291); a range is half-open, `start <= t < end` (core-time range.rs:56-58). A disabled clip draws nothing (293-295).
4. **Neighbours.** `prev_adjacent` / `next_adjacent` count only when they touch exactly and are enabled (296-298). `local = t - clip.start` (299, timeline.rs:255-257).
5. **Incoming transition** (302-311). If `clip.transition_in` exists and `local < tr.duration`:
   - `u = clamp(local / duration)`, computed in f64 seconds (276-281; duration ≤ 0 gives 1.0).
   - Draws the previous adjacent clip, if any, at opacity factor 1.0 and **evaluated at `t`**, which is past its end, so keyframes hold their last value and the source time runs into the media's handles (306).
   - Then draws this clip at factor `u` (308).
6. **Outgoing transition** (313-327). If `transition_out` exists, `t >= clip.end - dur`, and the next adjacent clip has **no** `transition_in`:
   - `u = (t - into) / dur`.
   - With a next clip: this clip at 1.0, then the next clip at `u`, evaluated at `t`, which is before its start (319-322).
   - Without one: this clip at `1 - u`, a fade out (323).
   - An incoming transition is checked first and wins.
7. Otherwise the clip is drawn at factor 1.0 (328).
8. **Sequence colour.** Every enabled `ColorMatrix` among `seq.effects` is folded with `compose`, evaluated at **sequence** time `t` (330-333). Any other sequence-level kind is ignored.

**Every `TransitionKind` (DipToBlack, DipToWhite, Wipe, Slide, Zoom, Custom) is drawn as this opacity-based dissolve.** The kind and `Transition.params` are never read. The "dissolve" is "B over A at opacity u" (premultiplied over, §1.5). That equals a linear mix only when both layers are opaque and cover the same pixels.

### 1.3 `layer_for` (graph.rs:203-274): one clip becomes one `LayerOp`

**Time.**
- `local = clip.to_local(t)` (204).
- `quantum` = the source stream's frame duration (205, project.rs:275-282).
- `source_time = time_map::source_time_at(clip, local, quantum)` (206). This reads the first enabled `TimeRemap` (time_map.rs:59-61). `frame_blend` is never read.

**Source.**
- **Asset clip** (208-218): `source.frame(clip, asset, source_time)?`. `None` means offline media, and the layer is dropped (210).
  - `layer_size = displayed_size(stream)`, falling back to the decoded frame size (214-215). `displayed_size` (167-183) widens by pixel aspect with rounding, and swaps width and height for 90/270° rotation.
  - `transfer = frame.transfer` (216).
- **Generator** (219-222): `generator_color` (185-200).
  - `SolidColor` reads `clip.metadata["color"]`: an array of at least 3 numbers, clamped to 0..1, alpha defaulting to 1. The default colour is white. **The colour is not an effect parameter.**
  - `ColorBars` draws a flat `[0.5, 0.5, 0.5, 1]`.
  - `Adjustment`, `Text`, `Silence` and `Tone` return `None`, so they draw nothing.
  - A generator layer has `layer_size = sequence size` and transfer `Srgb`.
  - **Side finding (confirmed with a probe):** metadata such as `["x", 0.5, 0.5]` passes the `len() < 3` check, but then `c[2]` panics with an index out of bounds at **graph.rs:193**. A malformed project can crash the renderer thread.

**The effect stack** (224-271). The nodes are `clip.effects` followed by `track.effects`, enabled ones only (228). **Track effects are evaluated at clip-local time** `local`. Each node is matched on `node.kind`:

| Kind | Lines | Rule |
|---|---|---|
| `Transform` | 230-236 | `position`, `scale`, `rotation`, `anchor`: **the last node wins** (overwrite). `opacity`: multiplied. A track Transform therefore replaces the clip's geometry. |
| `Crop` | 237-239 | `[left, right, top, bottom]`, last wins. `feather` is ignored. |
| `Composite` | 240 | `opacity` multiplied. The `blend` parameter and `node.blend` are ignored. |
| `ColorMatrix` | 241 | `color.compose(color_params_of(node, local))` |
| `Mask` | 242-268 | `shape` (falls back to the registry default rect 0,0,1,1). Rect gives kind 0 with `[x, y, w, h]`; **`corner_radius` is ignored**. Ellipse gives kind 1 with `[cx-rx, cy-ry, 2rx, 2ry]`. **Path becomes its bounding rectangle** (kind 0), seeded with `(1, 1, 0, 0)`, and `closed` is ignored. Also reads `feather`, `invert`, `opacity`. Last wins; a non-Shape `shape` value keeps the previous mask. |
| anything else | 269 | ignored |

Then `transform.opacity *= opacity_mul`, the transition factor (272). Opacity is never clamped in the renderer.

**Parameter readers.**
- `f`, `v2`, `v3` (116-136) evaluate keyframes (`Param::value_at`, effect.rs:353-386: hold, linear or cubic-bezier, in f64). They fall back to `effects_registry::default_value` (120, 125, 132), then to a hard-coded default.
- `b` (138-140) has **no** registry fallback.
- `color_params_of` (143-154) reads `exposure`, `contrast`, `saturation`, `temperature`, `tint`, `lift`, `gamma`, `gain`.

**`compose`** (57-68), used for multiple `ColorMatrix` nodes on clip and track. It is commutative, so node order does not matter:
```rust
self.exposure += o.exposure; self.contrast *= o.contrast; self.saturation *= o.saturation;
self.temperature += o.temperature; self.tint += o.tint;
lift[i] += o.lift[i]; gamma[i] *= o.gamma[i]; gain[i] *= o.gain[i];
```

**Consequence: the order of nodes in a stack has no effect on pixels**, apart from last-wins for Transform, Crop and Mask. The shader always applies a fixed order (§1.5).

### 1.4 `LayerOp`, `FramePlan` and the uniform packing

- `LayerOp` (graph.rs:89-104): `clip`, `source: Texture(SourceFrame) | Solid([f32; 4] sRGB-encoded)`, `layer_size [u32; 2]`, `transform: TransformParams` (13-28; defaults: centred, scale 1, anchor 0.5), `crop [l, r, t, b]`, `color: ColorParams` (32-48), `mask: Option<MaskParams>` (71-80), `transfer`.
- `FramePlan` (106-114): `width`, `height` (sequence), `layers`, `output_color`.

`layer_uniforms` (renderer.rs:69-97) builds a 192-byte `LayerUniforms` (27-46). It matches the WGSL struct at layer.wgsl:5-22:

| Field (offset) | Contents |
|---|---|
| `seq_size` (0) | sequence width and height |
| `layer_size` (8) | `layer.layer_size` |
| `position` (16), `scale` (24), `anchor` (32), `rotation` deg (40), `opacity` (44) | from `TransformParams` |
| `crop` (48) | l, r, t, b |
| `color_a` (64) | exposure, contrast, saturation, temperature |
| `color_b` (80) | **tint, transfer flag (1 only for `TransferCharacteristic::Linear`, 71; 0 for solids, 72), mask kind (-1 none / 0 rect / 1 ellipse), feather px** |
| `lift` (96) / `gamma` (112) / `gain` (128) | rgb (w = 0 / 1 / 1) |
| `mask_rect` (144) | x, y, w, h |
| `mask_misc` (160) | invert, mask opacity, is_solid, 0 |
| `solid` (176) | colour of a solid layer |

`OutputUniforms` (48-56) is 80 bytes: `color_a`, `color_b = [tint, 0, 0, 0]`, `lift`, `gamma`, `gain` (renderer.rs:590-591).

### 1.5 The GPU work and the exact maths

**Composite pass** (renderer.rs:594-613):
- Clears the composite to `background` (linear light, f64 into `wgpu::Color`; every caller uses opaque black).
- `set_pipeline(layer_pipeline)` once, then for each layer in plan order: `set_bind_group`, `draw(0..6)`.

**Vertex shader** (layer.wgsl:33-57). This is the crop and Transform geometry. y points down; layer uv (0,0) is top-left.
```wgsl
let x0 = u.crop.x; let x1 = 1.0 - u.crop.y; let y0 = u.crop.z; let y1 = 1.0 - u.crop.w;
let luv = vec2<f32>(mix(x0, x1, c.x), mix(y0, y1, c.y));          // corners: (0,0)(1,0)(0,1)(0,1)(1,0)(1,1)
let p = (luv - u.anchor) * u.layer_size * u.scale;
let r = radians(u.rotation);
let rp = vec2<f32>(p.x * cos(r) - p.y * sin(r), p.x * sin(r) + p.y * cos(r));   // clockwise on screen
let sp = rp + u.seq_size * 0.5 + u.position;
let ndc = vec2<f32>(sp.x / u.seq_size.x * 2.0 - 1.0, 1.0 - sp.y / u.seq_size.y * 2.0);
out.uv = luv;   // uncropped layer coordinates, interpolated by the rasteriser
```
- Crop shrinks the quad; it does not change uv. With `l + r > 1` the quad flips and still draws, because culling is off (gpu.rs:129). I inferred this from the code and did not test it.
- Edges are hard: one sample, no anti-aliasing (gpu.rs:131).

**Fragment shader** (layer.wgsl:99-114), with its helper functions:
```wgsl
if (u.mask_misc.z > 0.5) { src = u.solid; } else { src = textureSample(tex, samp, in.uv); }  // bilinear on ENCODED 8-bit values
var rgb = src.rgb;
if (u.color_b.y < 0.5) { rgb = srgb_to_linear(rgb); }   // BT.709/sRGB/unknown and every solid: sRGB curve
rgb = grade(rgb);                                        // always runs, also with identity parameters
let a = src.a * u.opacity * mask_value(in.uv);
return vec4<f32>(rgb * a, a);                            // premultiplied
```
`srgb_to_linear` (59-63):
```wgsl
lo = c/12.92; hi = pow((c+0.055)/1.055, 2.4); select(hi, lo, c <= 0.04045)
```
`grade` (65-74), in this order:
```wgsl
var rgb = rgb_in * exp2(u.color_a.x);                                  // exposure
rgb = max(rgb * u.gain.rgb + u.lift.rgb, vec3<f32>(0.0));              // gain, lift
rgb = pow(rgb, vec3<f32>(1.0) / max(u.gamma.rgb, vec3<f32>(0.01)));    // gamma
rgb = (rgb - vec3<f32>(0.18)) * u.color_a.y + vec3<f32>(0.18);         // contrast around 0.18
rgb = rgb * vec3<f32>(1.0 + u.color_a.w / 200.0, 1.0 - u.color_b.x / 200.0, 1.0 - u.color_a.w / 200.0); // temperature, tint
let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
rgb = vec3<f32>(luma) + (rgb - vec3<f32>(luma)) * u.color_a.z;         // saturation
return max(rgb, vec3<f32>(0.0));                                        // no upper clamp: values above 1 reach the composite
```
`mask_value` (76-97). It is evaluated per output pixel at the interpolated layer uv:
```wgsl
if (u.color_b.z < -0.5) { return 1.0; }
let feather = u.color_b.w / max(u.layer_size.x, 1.0);   // normalised by layer WIDTH only
rect:    d = min(min(uv.x-rect.x, rect.x+rect.z-uv.x), min(uv.y-rect.y, rect.y+rect.w-uv.y));
         m = clamp(d / max(feather, 1e-5) + 0.5, 0.0, 1.0);
ellipse: q = (uv - centre) / max(radii, 1e-5); dist = (1.0 - length(q)) * max(radii.x, radii.y);
         m = clamp(dist / max(feather, 1e-5) + 0.5, 0.0, 1.0);
if (invert) { m = 1.0 - m; }
return mix(1.0, m, mask_opacity);
```
- The feather is centred on the edge.
- Vertically, the feather in pixels is `feather × layer_h / layer_w`.
- The ellipse distance is an approximation.

**Blending.** `BlendState::PREMULTIPLIED_ALPHA_BLENDING` (gpu.rs:215): colour and alpha are both `src*1 + dst*(1 - src.a)`, into `Rgba16Float`. **Normal is the only blend mode that exists.**

**Output pass** (renderer.rs:614-628, output.wgsl:20-49). A full-screen triangle (3 vertices) with:
```wgsl
let s = textureSample(tex, samp, in.uv);          // equals a texel fetch (probe V9: 0 differences)
... same grade lines on s.rgb, with sequence ColorParams (output.wgsl:40-46), on the PREMULTIPLIED composite ...
rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
return vec4<f32>(linear_to_srgb(rgb), 1.0);       // lo = c*12.92; hi = 1.055*pow(c, 1/2.4) - 0.055; cut 0.0031308
```
Alpha is forced to 1.0. A non-opaque background therefore shows as if it were over black.

Then:
- `copy_texture_to_buffer` with rows padded to 256 bytes (renderer.rs:629-633, 193-196);
- `submit`, then `map_async` (634-639);
- `finish_unchecked` waits for this submission with a 60 s timeout (652), receives the map result (655), refuses if the device was lost (658-660), un-pads into `out` (661-678), and unmaps.

### 1.6 Catalogued kinds and parameters that draw nothing today

The catalog is effects-registry/src/lib.rs:143-237.
- **Kinds with no visible effect:** `Blur` (161), `Curves` (176), `Lut` (182), `Text` effect (197), `Shader` (209). The audio kinds (216-237) are not video.
- **`TimeRemap`** affects only the source time; `frame_blend` (189) is ignored.
- **Parameters that are read nowhere:**
  - `Crop.feather` (155);
  - `Composite.blend` (158) and `EffectNode.blend`, `EffectNode.mask` (core-model effect.rs:510, 512);
  - `Rect.corner_radius`; `Path` reduced to its bounding box;
  - `Transition.kind` and `params`.
- **Generators:** `Text` and `Adjustment` draw nothing. `ColorBars` is a flat grey placeholder.
- **Sequence level:** every kind except `ColorMatrix` is ignored.

Three places in the docs diverge from the code:
- ARCHITECTURE §12 describes `ColorMatrix` as "4x4 + offset" and the registry (166) says "compiled to one matrix". The code is the non-linear grade above.
- §12's renderer DAG, cache and `changed_ids` do not exist.
- The colour input transform by `ColorInfo` does not exist: playback collapses every transfer except Linear to `Bt709` (playback frames.rs:543-551), and the shader decodes that with the **sRGB** curve.

---

## 2. Formats, colour spaces, precision, draws, reuse

| Stage | Format and precision | Colour space | Code |
|---|---|---|---|
| Decoded source | tightly packed RGBA8 (`SourceFrame`, source.rs:7-15); FFmpeg `-pix_fmt rgba`, bicubic+full_chroma_int+accurate_rnd (media color.rs:28, decode.rs:554) | display-encoded; `transfer` defaults to Bt709 (source.rs:19) | |
| Layer texture | `Rgba8Unorm` (**not** the sRGB variant), size of the decoded frame, 1 mip (renderer.rs:237, gpu.rs:144-155); `write_texture` (256-264) | encoded | |
| Sampling | Linear/Linear, mip Nearest, ClampToEdge, no mip chain, no anisotropy (gpu.rs:217-226). **Filtering happens on encoded values, before linearisation.** Downscaling by more than 2× aliases. | encoded | layer.wgsl:105 |
| Solid layers | 1×1 white `Rgba8Unorm` is bound but not sampled (gpu.rs:227-234, renderer.rs:244) | `solid` uniform, sRGB-encoded | |
| Shader maths | f32 | linear light, BT.709 primaries implied, no gamut handling | |
| Composite | `Rgba16Float` at output size, RENDER_ATTACHMENT + TEXTURE_BINDING (gpu.rs:24, renderer.rs:200) | linear, **premultiplied**, may exceed 1 | |
| Blending | fixed function, at the hardware's precision | | gpu.rs:215 |
| Output | `Rgba8Unorm` at output size (gpu.rs:26, renderer.rs:202). f32 maths, then the hardware's float-to-unorm8 conversion. | sRGB curve (export tags the result BT.709) | |
| Readback | `MAP_READ` buffer, rows padded to 256 bytes, un-padded on the CPU | | renderer.rs:219-225, 661-678 |

**Measured:** an untouched 1:1 picture comes back **byte-identical for every 8-bit level of R, G, B and grey** on NVIDIA Vulkan.

**Fusion and cost.**
- Everything per layer is fused into one draw: crop and transform in the vertex stage; sample, linearise, grade, mask, opacity and premultiply in the fragment stage; blending in fixed function.
- Sequence colour and sRGB encoding are fused into the output pass.
- A frame is **2 render passes, N + 1 draws (6 vertices per layer, 3 for output), 1 copy, 1 map**.
- CPU-to-GPU traffic is a 192-byte uniform write per layer every frame (renderer.rs:567), plus a texture upload only for a new picture allocation (568-573).
- Doc figures (lib.rs:26-31, 47-56): about 2 ms per 1080p frame with one picture, 1 ms when the picture is unchanged, 9 ms instead of 15 ms with `render_ahead`.

**Reuse.**
- **Targets** (117-132): composite, output, output uniform buffer and bind group, and the readback buffer. Rebuilt only when the output size changes (582-586). Dropped when a frame in flight is abandoned (439-443), because the readback buffer may still be mapped.
- **LayerSlot** (146-153): uniform buffer, bind group, optional `Picture` with the texture plus `Arc` of the uploaded pixels.
- Slot choice (`prepare_layer`, 542-575):
  - a solid takes any free slot without a picture;
  - a texture first takes a free slot of the same size holding the same `Arc` (`ptr_eq`), otherwise any free slot of the same size, otherwise a new one.
- A slot is used once per frame (`last_used != now`). Slots idle for more than `KEEP_IDLE_FRAMES = 8` frames are dropped after submission (20, 641-643).
- `render_into` reuses the caller's `Vec` (669-677).

---

## 3. Shader modules, pipelines, the device, lost devices, fallback

**Creation.** All of this happens once per device in `Gpu::create`, inside `checked_on` (gpu.rs:177-245):
- `create_shader_module(ShaderSource::Wgsl(include_str!(...)))` for `layer` and `output` (211-212). The shaders are compiled into the binary; nothing is generated at runtime. naga parses and validates the WGSL and emits SPIR-V, HLSL or GLSL inside wgpu-core.
- One bind-group-layout shape for both pipelines (97-121):
  - binding 0: uniform, VERTEX_FRAGMENT;
  - binding 1: `texture_2d<f32>`, filterable, FRAGMENT;
  - binding 2: filtering sampler, FRAGMENT.
- `pipeline()` (123-141): one group, no vertex buffers, entry points `vs_main`/`fs_main`, `compilation_options: Default`, TriangleList, no culling, no depth, single sample, one colour target, `cache: None`.
  - Layer pipeline: `Rgba16Float` with premultiplied blending.
  - Output pipeline: `Rgba8Unorm`, no blending (215-216).
- There is no pipeline cache and no disk cache. Pipelines live in `Gpu` and are shared through `Arc<Gpu>`.

**The shared device.**
- `CURRENT: Mutex<Option<Arc<Gpu>>>` (59). `shared()` (74-86) creates the device under that lock. Every renderer holds `Arc<Gpu>` (renderer.rs:164, 271-273).
- `busy: RwLock` (52): submit and poll take it shared (251-262); `has_died` takes it exclusively (295-310). This works around a wgpu-core 30 panic (comment at 286-294).
- Instance: `InstanceDescriptor::new_without_display_handle()` (178). Backends default to `all()` (wgpu-types backend.rs:140-143). **Environment variables are not read.**
- Adapter selection: wgpu-core sorts adapters by device type, stably, in backend enumeration order (wgpu-core instance.rs:561-600). Here Vulkan is chosen.
- On DX12 the shader compiler is `Dx12Compiler::Auto`: static DXC is not built in, so it uses `dxcompiler.dll` if it is on PATH, otherwise FXC (wgpu-types backend.rs:777-803). **This is a hidden machine-dependent variable for pixels on DX12.**

**Adapter, limits, fallback.**
- First a `HighPerformance` hardware adapter; otherwise `force_fallback_adapter: true` (179-193), which keeps only `DeviceType::Cpu` adapters (WARP here).
- `required_features: empty`, `required_limits: adapter.limits()` (194-200). There is no downlevel check.
- `max_texture_side = device.limits().max_texture_dimension_2d` (243). Larger sources or outputs are refused as `Invalid` (renderer.rs:496-524).
- `is_software()` means `DeviceType::Cpu` (renderer.rs:281-283).

**Errors and device loss.**
- `checked_on` pushes three error scopes (OutOfMemory, Internal, Validation) and pops them with `pollster`. A raised error becomes `RenderError::Gpu` (160-171), or `Device` while the device is being created (237-242). `on_uncaptured_error` only prints (205).
- The lost-device callback sets the `lost` flag (206-208).
- `has_died()` submits an empty probe and waits on it with zero timeout; `WrongSubmissionIndex` means dead (295-310).
- `with_recovery` (renderer.rs:447-478):
  - if `is_lost`, reconnect first;
  - on any non-`Invalid` error: forget resources, poll once, ask `has_died`, then reconnect and retry once.
- `collect_into` redraws the kept plan on the new device (417-428).
- Lost devices are kept in `RETIRED` forever (65, 82-84).
- Tests cover all of this (renderer.rs:802-872, 1000-1058).

---

## 4. Tests, reference images, and how to make golden images

No reference images are checked in. The only PNGs in the repo are UI screenshots (`docs/*.png`, `ui/e2e/artifacts/dogfood/*.png`).

| Kind | Tests |
|---|---|
| **Analytic expectations with a tolerance** (`assert_px`, tolerance 0–2, render.rs:57-64; f64 sRGB helpers `enc`/`dec` 66-75) | render.rs: `solid_generator_fills_the_frame` 85 (±1/0), `opacity_and_layer_order_blend_in_linear_light` 98 (±2), `transform_scale_position_and_crop` 117 (±1/0), `color_matrix_exposure_doubles_linear_light` 148 (±2), `sequence_level_color_applies_to_everything` 167 (±2), `texture_sources_are_sampled_and_oriented` 178 (±2), `ellipse_mask_cuts_corners` 218 (±1), `cross_dissolve_mixes_adjacent_clips` 230 (±2/1), `output_size_override_keeps_aspect` 245 (±1), `muted_tracks_are_skipped` 510 (0). threads.rs 58/95/153 sample 3 pixels (±1, ±2). Despite the header at render.rs:1, these are **not** pixel-exact. |
| **Exact pixel values** (solid colours) | first_use.rs:121; bad_requests.rs:64; render.rs:446-459; renderer.rs:1006, 1032-1033 |
| **Byte-exact self-consistency** (same frame from another path, history or thread) | render.rs:279, 322, 372; reuse.rs:36, 128, 177, 209; many_renderers.rs:48, 96, 162, 248; refused.rs:25; threads.rs:208; renderer.rs unit tests 746, 802, 883, 1000 |
| CPU plan-level tests | graph.rs:383-477 (`effects_fold_into_params`, `cross_dissolve_draws_both_clips`, `fade_out…`, `displayed_size`) |
| Elsewhere | playback hostile.rs:249-252 (render at 1:1 within ≤1 level of the decode); proxy.rs and proxy_exact.rs use `mean_abs_diff`; export tests compare with tolerances, and still-to-PNG must be exact (export tests:879) |

**Existing helpers.**
- `SolidSource` and `NoSource` (source.rs:39-57); `SourceFrame::new` and `::solid` (17-28).
- `RgbaImage::to_ppm`, `write_ppm` and `mean_abs_diff` (image.rs:37-56).
- `render_plan` (renderer.rs:338) draws any `FramePlan` without a project.
- tests/common/mod.rs:
  - `picture(w, h, seed)` (29-40): every pixel unique;
  - `layer` (43-45);
  - `scene(seed, w, h)` (52-79): solid, full-frame picture, and a turned, cropped, masked and graded picture; sequence contrast;
  - `fresh` (84-87);
  - `first_difference` (90-103).
- **`frame_bench --dump file`** (examples/frame_bench.rs:132-197, 219): 7 scenes × 2 passes covering transform, crop, ColorMatrix, ellipse mask with feather, dissolve, fade, sequence colour, odd output size and a non-black background. The format is `width u32 LE, height u32 LE, rgba` per image, written "so two builds of the renderer can be compared byte for byte".

**What my measurements say about golden images.**
- Today's output is deterministic per adapter (two runs: 0 differing bytes).
- It is **not portable across adapters**:
  - NVIDIA Vulkan, DX12 and GL are identical except 1 byte in an ellipse-mask scene;
  - WARP differs by 1 level on 1–15% of pixels, and by up to 182–200 in rotated 1080p scenes, which looks like edge-coverage differences (I did not check pixel by pixel).
- So:
  1. **Primary net:** freeze today's `graph.rs`, `renderer.rs`, `gpu.rs` and shaders as a test-only "legacy" crate, and compare the new runner byte for byte against it **in the same process, on the same device**. That works on any machine, including WARP in CI.
  2. **Secondary net:** golden files keyed by adapter name, backend, driver version, and wgpu/naga version, generated with a dump tool modelled on `frame_bench --dump`. A wgpu or naga upgrade is itself a pixel-change risk.
  3. **Build scenes through `ProjectState` and `layers_at`,** not `FramePlan`, so the goldens cover registry defaults, keyframes, last-wins, composition, track and sequence handling, and transitions. `tests/common::scene` bypasses `graph.rs`.
  4. **Cover these cases:**
     - each Transform parameter, including anchor and rotation, with clip+track override;
     - each crop edge, plus `l + r > 1`;
     - Composite opacity;
     - each ColorMatrix parameter alone, combinations, 2 nodes, clip+track;
     - mask rect, ellipse and path (bounding box), corner_radius, invert, feather 0 and > 0, opacity;
     - **all 10 `BlendMode` values and every `TransitionKind`**, which must stay equal to Normal and dissolve;
     - transition in or out, with or without a neighbour, conflicting in+out, at 0, mid and end;
     - SolidColor with and without metadata, with alpha; ColorBars; Text, Adjustment;
     - `Blur`, `Curves`, `Lut`, `Text`, `Shader` present (no change);
     - disabled nodes, solo and mute, offline media;
     - Linear versus Bt709 transfer;
     - decode size different from layer size, anamorphic, output size different from sequence size, non-opaque background;
     - both smooth and noise pictures, because noise exposes filtering differences far more strongly (§6).

---

## 5. Every place that knows a specific effect, parameter, blend, transition or generator

**crates/render/src/graph.rs**
- 5-8: imports `EffectKind`, `GeneratorKind`, `MaskShape`, `ParamValue`, `TransferCharacteristic`.
- 13-28: `TransformParams`; fields and defaults mirror the Transform catalog.
- 30-69: `ColorParams`; fields are ColorMatrix parameter names. `is_identity`; `compose` is the folding rule.
- 71-80: `MaskParams` with shape codes 0/1.
- 82-87: `LayerSource::Solid`, the generator path.
- 89-104, 112-113: `LayerOp.crop`, `color`, `mask`; `FramePlan.output_color`.
- 116-140: typed parameter readers with registry fallback (120, 125, 132).
- 143-154: the eight ColorMatrix parameter IDs.
- 185-200: `GeneratorKind` SolidColor (`metadata["color"]`, **193 panic**), ColorBars, and the four that draw nothing.
- 204-206: `time_map::source_time_at` (TimeRemap lives in core-model).
- 219-221: generator size and transfer.
- 228: clip effects then track effects.
- 230-269: the per-kind rules in §1.3.
- 272, 276-281: transition factor.
- 301-327: `transition_in` / `transition_out`, kind ignored.
- 331: sequence `EffectKind::ColorMatrix` only.
- Tests at 337-478.

**crates/render/src/renderer.rs**
- 27-46, 48-56: uniform layouts by effect.
- 58-66: `color_vectors`.
- 69-97: `layer_uniforms`: transfer flag `Linear` (71), solid (72), mask encoding (75-78), packing.
- 545-560: slot choice by Solid or Texture.
- 590-591: sequence colour uniforms.
- 600-603: background.
- Tests at 693-740 (SolidColor, Bt709).

**crates/render/src/gpu.rs**
- 24, 26: formats.
- 211-212: shaders.
- 215: Normal blending hard-wired.
- 217-226: sampler.
- 227-234: white texture for solids.

**crates/render/shaders**
- layer.wgsl 5-22: uniform semantics. 41-52: crop and transform. 59-63: transfer. 65-74: ColorMatrix. 76-97: Mask. 102-103: solid. 112-113: opacity and premultiply.
- output.wgsl 40-48: sequence ColorMatrix, clamp, sRGB.

**crates/render, other files**
- lib.rs 10-12: the Phase 1 scope doc. 65: re-exports `ColorParams`, `MaskParams`, `TransformParams`.
- source.rs 14, 19: transfer.
- Cargo.toml 13: `effects-registry` dependency.
- tests/render.rs: `EffectKind::{Transform, Crop, ColorMatrix, Mask}`, `MaskShape::Ellipse`, `TransitionKind::CrossDissolve`, `GeneratorKind::SolidColor`, and parameters `opacity`, `scale`, `position`, `left`, `exposure`, `saturation`, `shape`, `color` (30-32, 100-242, 274, 283, 336, 377, 463).
- tests/common/mod.rs 43-79 (via `FramePlan`).
- examples/frame_bench.rs 18-19, 73, 136-181.

**crates/playback**
- No effect, blend, transition or generator knowledge.
- frames.rs:543-551 maps the stream transfer (Linear stays Linear, everything else becomes Bt709).
- frames.rs:1535-1541 handles only video asset clips; generators return `None`.
- Reverse reading is detected from the pattern of requests (frames.rs:948-959), not from TimeRemap.

**Callers outside scope, for completeness**
- export.rs:570, 596-609 (`layers_at` + `submit_plan`/`collect_into`, default background);
- export still.rs:84;
- the app's player.rs:691-692;
- cli main.rs:1027-1036.

---

## 6. Professional judgement: one pass per effect over rgba16float

I compared the probe variants with today's pipeline. The figures are differing bytes, the share of pixels affected, and the maximum 8-bit difference, on NVIDIA Vulkan (DX12 and GL agreed) and on WARP.

| Variant | Result |
|---|---|
| V1: source decoded to a linear `rgba16f` texture at its own size, then resampled by the transform pass | **different**: 1:1 is exact; scaled or rotated noise **up to 72 levels, ~100% of pixels**; smooth content ±1. Using `rgba32float` (V1Lin32) gives the same picture, so the cause is **filtering in linear instead of encoded light**, not precision. |
| V1Enc16: source stored as encoded `rgba16f` (n/255 is not exact in f16), filtered as today | ±1 on 2–15% of pixels |
| V2: decode, grade, mask, each a texture-space pass in `rgba16f`, then the transform pass | as V1, plus mask differences at feathered edges |
| V3: layer placed into an output-size `rgba16f` intermediate (straight alpha), colour as a separate pass that composites | ±1 on up to ~7% of pixels, even when the grade is the identity |
| V4: colour stage skipped when the layer has no ColorMatrix | **0 on all 320×180 scenes, but 2–8 bytes per 1080p frame differ**. The identity `grade()` is not bit-neutral (pow and exp2 rounding). |
| V6: layer uv computed per pixel by inverse mapping instead of the interpolated vertex uv | ±2 on rotated layers (368k bytes at 1080p) |
| V12: forward placement as an f64 CPU matrix | 0 on NVIDIA, **±2 on WARP** for rotated layers |
| V8Inter: layer to an `rgba16f` intermediate, then fixed-function blend | 0 on NVIDIA, **±1 on WARP** for translucent layers |
| V8Prog / V8c: Normal blending done in the shader (read the destination, `s + d*(1-s.a)`) | **±1 on NVIDIA** for translucent layers; 0 on WARP (V8c) |
| B0Split (a render pass per layer), V7 (same arithmetic as chained per-block WGSL functions), V7Reorder, V11 (different module), V10 (`textureSampleLevel(…, 0)`), V9 (output via `textureLoad`) | **0 everywhere** |

**Semantic differences** were computed on the CPU in f64 (`composition_check`): running two ColorMatrix nodes in sequence instead of composing them differs by up to:
- 53 levels for saturation 1.5 then 0.5;
- 36 for contrast 1.2 then 0.9;
- 35 for contrast 1.5 then gain 0.8;
- 19 for saturation 0 then temperature 30;
- 7 for gamma then exposure;
- 6 for lift then gain.

Composition and sequence agree only for exposure + exposure, exposure then contrast, gain then lift, and temperature then saturation.

**Where a pass-per-effect design would therefore differ:**
1. **Resampling space and count.** Today there is exactly one bilinear fetch of the **8-bit encoded** texture, at its final placement. Decoding first, or resampling a second time (for example a separate crop or transform block), changes pixels by large amounts on detailed footage.
2. **Precision.** Every extra `rgba16f` storage rounds the value, including source copies, per-layer intermediates and straight-alpha colour. Shader-side Normal blending differs from the hardware blend on NVIDIA, and an intermediate before a fixed-function blend differs on WARP. No intermediate-based Normal blend matched on both.
3. **Order and semantics.**
   - ColorMatrix nodes are **folded** (`compose`), not chained.
   - Transform, Crop and Mask are **last-wins**, and track nodes are appended to clip nodes and evaluated at clip-local time.
   - The mask is evaluated per output pixel in layer space.
   - Transitions are opacity factors ("B over A"), not a mix.
   - Every layer runs the identity grade.
   - Placement uses f32 `radians`/`cos`/`sin` per vertex with interpolated uv.
   - A generic stack-order runner breaks each of these.
4. **Adapter-dependent maths.** `pow`, `exp2` and `length` already differ between backends today. "The same pixels" can only be defined per adapter, driver and compiler. Pin wgpu and naga for the migration, and verify by in-process A/B.

**A structure that keeps the bytes identical, supported by the zero-difference variants:**
- **Keep the core-owned geometry exactly as it is.** Today's 6-vertex quad and `vs_main`, unchanged, including crop and the last-wins folding of Transform. This fits "placement may stay in the core".
- **Generate one fragment shader per layer signature.** Sample the source with `textureSample` or `textureSampleLevel` on the `Rgba8Unorm` 8-bit texture with today's sampler. Then call, in a canonical order, block functions with the exact arithmetic:
  - linearise;
  - colour blocks: the built-in ColorMatrix receives the **CPU-folded** parameters, and an identity ColorMatrix stage is **always** emitted for migrated v2 projects;
  - alpha blocks: `src.a * opacity * mask(uv)`, with the mask at the interpolated layer uv;
  - premultiply.
  
  Plug-in pixel blocks become WGSL functions concatenated into this shader. Their parameters live in a uniform or storage buffer. Splitting code into functions or modules did not change bytes.
- **Keep fixed-function `PREMULTIPLIED_ALPHA_BLENDING`** into the `rgba16f` composite for Normal. Non-normal blend modes, which draw nothing today, may use shader blending with a destination read, because no existing pixels depend on them.
- **Give neighbourhood blocks their own pass** (blur, LUT, text, plug-ins that need it), inserted **only when such a block is present**. No existing project uses one, so v2 projects stay exact.
- **Transitions:** keep the opacity-factor dissolve for every existing `TransitionKind`, or migrate them explicitly to "dissolve".
- **Keep the output pass as it is.** Fold the sequence ColorMatrix into it, then clamp and apply the sRGB curve.
- **Verify** with the legacy-crate A/B on every machine, using noise and smooth content, plus a WARP golden set in CI.