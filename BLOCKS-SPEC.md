# ChadFX plug-in blocks: the technical contract

Status: approved by the owner on 2026-10-29. This is the shared contract every stage builds to. Where a stage finds a rule here that cannot work, it must say so in its report with evidence and propose the smallest change; it must not silently diverge. Research behind these decisions (with file:line evidence) lives beside this file in `maps/*.md`; read the map for your area.

Owner decisions that are settled (do not reopen):
- D1. Byte-exact rendering of old projects: the migration adds a neutral colour node to every picture clip and every sequence of an old project (see 9.4).
- D2. Keyframed (animated) sound levels and pan become smooth ramps, identical in preview and export. Today's stepped behaviour is not reproduced.
- D3. Text and captions are the NEXT task, not this one. Leave room (a host "text" capability), build nothing for it.
- D4. The ten unfinished brief checks run alongside Stage 0; bug-fixing reviewers come after this work.

---

## 1. Vocabulary

- **Block**: an effect building block. Every effect, transition, blend mode and generator is a block. A block is described by a manifest and implemented by WGSL (picture) or WebAssembly (sound).
- **Package**: a block on disk. A folder (for development) or a single zip archive with the extension `.chadfx-block`. Both have the same content.
- **Node**: one use of a block in a project: an `EffectNode` in a clip, track or sequence effect list, a clip's source, or a clip's transition.
- **Host**: the app (renderer, mixer, engine, interface). Host code never knows any specific block. It knows stages, parameter types, roles and fold rules.
- **Core node**: a node the engine itself interprets. There is exactly one: the time remap (`core.time_remap`, section 9.2). It is not a block, cannot be installed, removed or replaced.

## 2. Package format

```
acme-glow/                      (folder or the root of the zip)
  block.json                    required: the manifest (section 3)
  block.wgsl                    picture blocks: the code (any name; the manifest names it)
  block.wasm                    sound blocks: the module
  README.md, LICENSE, icon.png  optional, shown in the block manager (icon <= 256 KiB, PNG)
  tests/                        optional, ignored by the app; used by `vep blocks test` later
```

Reading rules (the package reader in `crates/blocks`):
- A zip is read entirely in memory and NEVER extracted to disk. Only whitelisted names are read (`block.json`, the files the manifest names, README.md, LICENSE, icon.png).
- Limits: archive <= 16 MiB; <= 64 entries (checked before iterating); each file <= 4 MiB declared and actually read through `take(limit+1)`; compression ratio <= 100:1; only Stored and Deflate; reject encrypted entries, symlinks, absolute paths, `..`, NUL, overlapping entries, case-insensitive duplicate names, names longer than 255 bytes.
- Identity of a package = BLAKE3 of its bytes (folders: of a canonical zip built in memory).
- `zip` crate 9.x with `default-features = false, features = ["deflate-flate2-zlib-rs"]` (writing is needed to pack folders on install).

## 3. The manifest (`block.json`)

JSON. Unknown fields are an error (so typos are caught). Every type is defined in `crates/blocks` with serde + schemars, and a JSON Schema for it is published so authors get editor checking.

```jsonc
{
  "manifest": 1,                       // manifest format version (this document)
  "id": "acme.glow",                   // [a-z0-9][a-z0-9_-]* "." [a-z0-9][a-z0-9_.-]*, <= 64 bytes.
                                       // "chadfx." is reserved for built-in blocks, "core." for core nodes.
  "version": "1.2.0",                  // semver, for people
  "format": 2,                         // integer >= 1: version of the PARAMETER format. Nodes store this.
  "name": "Glow",                      // LocalizedText: a string (English) or {"en": "...", "fr": "..."}
  "description": "Soft light that spreads from the bright parts of the picture.",   // LocalizedText; the AI planner reads it
  "category": "light",                 // free text for grouping in menus
  "kind": "video.filter",              // section 4: video.filter | video.transition | video.blend | video.source
                                       //            audio.filter | audio.transition | audio.source
  "params": [ /* ParamDecl, section 5 */ ],
  "fold": "none",                      // section 6: none | last | params
  "video": { /* section 7, picture blocks only */ },
  "audio": { /* section 8, sound blocks only */ },
  "transition": { /* section 7.6 / 8.5, transitions only */ },
  "migrations": [ /* section 10 */ ],
  "author": "Acme", "license": "MIT", "homepage": "..."   // optional, display only
}
```

## 4. Kinds

| kind | inputs | what it is |
|---|---|---|
| `video.filter` | the layer | changes a clip's picture: one or more stages (section 7): `region`, `place`, `pixel`, `pass` |
| `video.transition` | outgoing, incoming | how one clip turns into the next at a cut; style `fade` or `mix` |
| `video.blend` | layer, what is beneath | how a layer combines with the picture beneath it |
| `video.source` | none | makes a picture from nothing (solid colour; later text) |
| `audio.filter` | the clip's sound | changes sound |
| `audio.transition` | outgoing, incoming | crossfades at a cut |
| `audio.source` | none | makes sound from nothing (tone) |

Where nodes may live: filters in clip, track and sequence effect lists of the matching media kind; blends in clip and track lists (video); transitions only in a clip's `transition_in`/`transition_out`; sources only as a clip's source. The engine does NOT enforce these (it cannot see manifests); the edges do (section 11).

## 5. Parameters (`ParamDecl`)

```jsonc
{
  "id": "radius",                  // [a-z][a-z0-9_]*, not a WGSL keyword, <= 32 bytes, unique in the block. NEVER renamed
                                   // without a migration (it is stored in projects).
  "type": "float",                 // float | int | bool | enum | vec2 | vec3 | color | shape | curve | text | blob | artifact
  "default": 12.0,                 // a value of that type (same JSON shape as core_model::ParamValue's "value")
  "min": 0.0, "max": 200.0,        // optional; numbers or per-component arrays for vectors
  "step": 0.5,                     // optional; the inspector's step
  "unit": "px",                    // optional: px | degrees | percent | fraction | db | ms | seconds | hz | stops | multiplier | none
  "options": [ {"value": "gaussian", "label": "Gaussian"} ],   // enum only; stored as the value string
  "keyframable": true,             // default: true for float/vec2/vec3/color/shape/curve, false otherwise
  "label": "Radius",               // LocalizedText
  "description": "How far the glow spreads, in sequence pixels.",   // LocalizedText; the AI reads it
  "advanced": false,               // simple mode hides advanced parameters
  "hidden": false,                 // never shown; kept on the node (used for inert legacy parameters)
  "role": "opacity",               // optional host role (section 6.2)
  "fold": "add",                   // only with block fold "params": add | multiply | last
  "components": ["R", "G", "B"]    // optional labels for vector components (default X,Y / X,Y,Z)
}
```

Types visible to shaders (packed by the host into the block's `Params` struct, field name = param id): `float` f32, `int` i32, `bool` u32 (0/1), `enum` u32 (index into options), `vec2` vec2<f32>, `vec3` vec3<f32>, `color` vec4<f32> (r,g,b,a as stored), `shape` `VepShape` (section 7.2). `curve`, `text`, `blob`, `artifact` are NOT visible to shaders in manifest version 1: a picture block may declare them only as `hidden` data. Sound blocks receive every numeric parameter flattened to f32 (section 8.3); `text`, `curve`, `shape`, `blob`, `artifact` are not passed to sound blocks in version 1.

Values are evaluated by the host exactly as today: `Param::value_at(clip-local time)` in f64 (hold, linear, bezier), then converted to f32 for the GPU or the WASM module. A parameter missing from a node takes the manifest default.

## 6. Roles and fold rules

### 6.1 Fold rules (block level)
How several nodes of the same block in one stack combine. The stack of a clip is its own enabled effect nodes followed by its track's enabled nodes (today's order, kept).
- `none` (default for plug-ins): every node runs, in stack order.
- `last`: only the last enabled node of this block in the stack runs.
- `params`: all enabled nodes of this block fold into ONE run, at the position of the first. Each parameter folds with its own `fold`: `add`, `multiply` or `last`. Folding starts from the manifest defaults and is done in f32 in stack order. (This is exactly today's ColorMatrix compose; it is commutative for add/multiply.)

### 6.2 Roles (parameter level)
Roles tell the host what a parameter means, so host features need no block knowledge.
- `opacity`: the host multiplies every opacity-role value of every enabled node in the stack, in stack order, in f32, starting from 1.0, then multiplies by the transition factor (7.6). That product is the layer opacity, applied once to the source alpha before the pixel stages (7.3). This reproduces today's arithmetic exactly. Roles apply to all enabled nodes, regardless of the block's fold rule.
- `scale`, `position`, `rotation`, `anchor`: placement parameters, so host features such as "fit to frame" (ui/src-tauri/src/service/framing.rs) find them without knowing the block. At most one block in a stack should own placement roles; fit-to-frame uses the last block in the stack that declares a `scale` role (or adds the catalogue's default placement block, the one whose manifest declares `"default_for": "placement"`; the built-in transform).
- `gain_db`, `pan`: sound levels, for later automatic mixing by the AI.
- Roles are a closed list in manifest version 1; unknown roles are a manifest error.

## 7. Picture blocks: stages and the WGSL contract

### 7.1 Principle
Each layer is drawn in ONE draw call, exactly as today: a quad positioned by the vertex stage, a fragment stage that samples the source once, converts to linear light, runs the pixel stages, applies alpha, premultiplies; the hardware blends it onto the `rgba16float` composite with premultiplied "over". Region, place and pixel stages of every block in the stack are WGSL functions the host joins into that one draw (fused). Only `pass` stages, non-default blends and `mix` transitions add draws, and only when such a block is present. Measured basis: maps/renderer.md section 6 (one fused draw per layer with today's arithmetic reproduces today's bytes; splitting into passes changes them by up to 72 levels).

A block supplies FUNCTIONS ONLY. No entry points, no bindings, no module-scope variables, no `override`, no `enable`, no storage buffers, no atomics, no workgroup memory, no barriers, no derivatives (`dpdx`, `fwidth`, `textureSample` with implicit level), no `discard`. The host owns every binding, every entry point, the parameter struct, the uniform buffer and all textures. Names beginning with `vep_` and `Vep` are reserved for the host. The host renames every module-scope name of a block with a unique prefix when composing.

### 7.2 Host prelude visible to blocks
```wgsl
struct VepCtx {
  seq_size: vec2<f32>,     // sequence frame size in sequence pixels
  layer_size: vec2<f32>,   // the layer's displayed size in sequence pixels (render::displayed_size; sources: sequence size)
  time: f32,               // seconds since the clip's start (clip-local), the time parameters were evaluated at
  frame_rate: f32,
  progress: f32,           // transitions: 0..1; otherwise 0
  output_scale: f32,       // output pixels per sequence pixel (preview at reduced size < 1)
};
struct VepShape {          // a shape parameter
  kind: u32,               // 0 rect, 1 ellipse, 2 path (version 1 gives a path as its bounding rect, kind 0 in `a`)
  a: vec4<f32>,            // rect: x, y, width, height; ellipse: cx, cy, rx, ry (normalised 0..1 of the layer)
  b: vec4<f32>,            // rect: corner_radius, 0, 0, 0
};
fn vep_srgb_to_linear(c: vec3<f32>) -> vec3<f32>;
fn vep_linear_to_srgb(c: vec3<f32>) -> vec3<f32>;
fn vep_luma(c: vec3<f32>) -> f32;                       // BT.709 weights
// pass, blend and mix-transition stages only:
fn vep_sample(input: u32, uv: vec2<f32>) -> vec4<f32>;  // explicit level 0, bilinear, clamp to edge
fn vep_load(input: u32, px: vec2<i32>) -> vec4<f32>;
fn vep_input_size(input: u32) -> vec2<f32>;
```
The block's parameter struct is generated by the host from the manifest and is always called `Params` inside the block's source.

### 7.3 Stage signatures
All colours are LINEAR light. Pixel stages see straight (not premultiplied) alpha. Pass, blend and transition stages see PREMULTIPLIED colour (the composite's working form).

| stage | signature | runs in | notes |
|---|---|---|---|
| `region` | `fn region(r: vec4<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | vertex | `r` = visible part of the layer as x0, x1, y0, y1 in layer uv (starts 0,1,0,1). Crop. |
| `place` | `fn place(pos: vec2<f32>, luv: vec2<f32>, ctx: VepCtx, p: Params) -> vec2<f32>` | vertex | returns the corner's position in sequence pixels. `pos` is the position so far (the host's default placement, or the previous place stage); `luv` is the corner's layer uv. Transform computes from `luv` and ignores `pos`. |
| `pixel` | `fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | fragment | colour, alpha, masks. `uv` = interpolated layer uv. Also used at sequence level (7.7). |
| `source` | `fn source(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | fragment | sources only. Returns colour ENCODED like footage (sRGB/BT.709 curve), straight alpha; the host converts it like footage. |
| `pass` | `fn <name>(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | own draw | neighbourhood effects; declared passes in the manifest (7.5). |
| `blend` | `fn blend(src: vec4<f32>, dst: vec4<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | own draw | premultiplied layer over premultiplied beneath. |
| `transition` | `fn transition(a: vec4<f32>, b: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | own draw | mix style only; a = outgoing, b = incoming, premultiplied, at sequence uv. |

The manifest lists what the block provides:
```jsonc
"video": { "source": "block.wgsl", "stages": ["place", "pixel"],
           "passes": [ /* pass stages only, 7.5 */ ] }
```

### 7.4 The fused layer draw (host template)
Vertex: `r = (0,1,0,1)`; run every region stage in stack order (after fold); corner `luv = mix(r)`; `pos` = default placement computed with EXACTLY today's transform formula and default values (anchor 0.5, scale 1, rotation 0, position 0), then every place stage in stack order; to NDC with today's formula. The Stage 3 engineer must prove byte-exactness of the default placement against the goldens; if a syntactically different default formula is not byte-exact, the host uses today's full formula with constants.
Fragment: `c = source()` or `textureSample(tex, samp, uv)` with today's sampler on the `Rgba8Unorm` texture; `c.rgb = vep_srgb_to_linear(c.rgb)` unless the source is linear (today's transfer flag); `c.a = c.a * layer_opacity` (6.2); every pixel stage in stack order (after fold); `return vec4(c.rgb * c.a, c.a)`.
Blending: fixed-function `PREMULTIPLIED_ALPHA_BLENDING` onto the `rgba16float` composite (default "over"). Output pass: today's, with sequence-level pixel stages (7.7).

### 7.5 Pass stages (neighbourhood effects)
Declared in the manifest:
```jsonc
"passes": [
  { "name": "down", "input": "layer", "output": "half", "scale": 0.5 },
  { "name": "blur_h", "input": "half", "output": "tmp" },
  { "name": "blur_v", "input": "tmp", "output": "out" },
  { "name": "combine", "inputs": ["layer", "out"], "output": "result" }
]
```
- Version 1 runs pass stages in SEQUENCE space after placement: the host draws the layer (all fused stages) into a cleared transparent `rgba16float` intermediate of output size, runs the passes, and composites `result` (or the last output) with the layer's blend. A block with pass stages may also have pixel stages; those are fused into the layer draw (before the passes).
- Limits: <= 4 passes per block, <= 4 intermediates, each at scale 0.25..1 of output size, `rgba16float`. A pass reads at most 4 inputs. Passes run at output size, so a preview at reduced size costs less; parameters in sequence pixels are scaled by `ctx.output_scale` by the block itself.
- Sequence-level pass stages run on the composite before the output pass.

### 7.6 Transitions
```jsonc
"transition": { "style": "fade",
  "incoming": [[0,0],[1,1]], "outgoing": [[0,1],[1,1]],
  "outgoing_alone": [[0,1],[1,0]], "incoming_alone": [[0,0],[1,1]] }
```
- `fade`: no shader. The host draws both clips as today, multiplying each clip's layer opacity by the piecewise-linear curve evaluated at the transition progress (f64, then f32). `outgoing_alone`/`incoming_alone` apply when the neighbour does not exist (fade out / fade in). The cross dissolve is `incoming` linear 0..1 and `outgoing` held at 1, exactly today's "B over A at opacity u".
- `mix`: both clips are drawn into two intermediates; the `transition` stage combines them; the result is composited with normal over.
- Progress `u` is computed exactly as today: render/src/graph.rs:276-281 (f64 seconds, clamped).
- Which clip's transition wins at a cut, and the timing, is core and unchanged (graph.rs:301-327, mixer.rs:286-341).

### 7.7 Sequence level
Sequence effect lists hold `video.filter` nodes (pixel stages run in the output pass, on the PREMULTIPLIED composite, before clamp and encode: today's behaviour) and `audio.filter` nodes (8.6). Pass stages at sequence level run before the output pass.

### 7.8 Blends
A clip or track stack may hold `video.blend` nodes; fold is always `last`. No blend node means the host's default premultiplied over (fixed function). A blend node: the layer is drawn into an intermediate, then a draw reads the intermediate and a copy of the composite and writes the blended result. A blend manifest may declare `"fixed_function": "premultiplied_over"`; the host may then use hardware blending (only the normal blend may say this).

### 7.9 Validation and limits (picture)
1. Parse the block alone after a stub prelude (7.2) with `wgpu::naga` 30 (`front::wgsl::parse_str`), on a worker thread with an 8 MiB stack.
2. Validate with `naga::valid::Validator::new(ValidationFlags::all(), Capabilities::empty())` (a fixed capability set on every machine).
3. Check the IR: no global variables or entry points beyond the prelude's; the declared stage functions exist with the exact signatures; nothing forbidden in 7.1; source <= 64 KiB, <= 64 functions.
4. Insert the loop budget into every loop (maps/technology-research.md 1.6): a private `vep_budget` counter decremented by a static cost of the loop body; at zero the loop breaks. Budget 4096 cost units per invocation; a texture read costs 16.
5. Compose (prefix names, generate Params structs and the template), validate the composed module again, create the module with the CHECKED path only (never `create_shader_module_trusted`), pass it as `ShaderSource::Naga` (wgpu feature `naga-ir`).
6. Compile lazily on a worker thread with a time limit (2 s); cache pipelines in memory keyed by BLAKE3 of the composed source + entry points + target format; rebuild after device loss. Compile ahead at project open for the blocks the project uses. A slow compile counts as a refusal for that block.
7. Untrusted (non-built-in) picture blocks are refused on the GL backend.
8. Device loss safety: before each submission that includes non-built-in blocks, the host records the set of those blocks in a crash-safe "in flight" file in the user block folder; on device loss or at the next start after a crash, those blocks are QUARANTINED (disabled with a reason) BEFORE any retry. A slow-frame tripwire (250 ms at preview size; per-pass tiling for 4K export) quarantines a block whose frames are slow. Never retry a frame with an untrusted block automatically. Built-in blocks are trusted (shipped with the app) but go through the same validation.

## 8. Sound blocks: the WebAssembly contract

### 8.1 Runtime
`wasmi` 2.0 (`default-features = false`, features `std`, `validate`, `deterministic`; no `simd`). Config: `consume_fuel(true)`, `wasm_simd(false)`, `wasm_relaxed_simd(false)`, `wasm_memory64(false)`, `wasm_multi_memory(false)`, `allow_start_fn(false)`, `enforced_limits(EnforcedLimits::strict())`. Store limits: memory <= 16 MiB, tables <= 1000 elements, 1 instance per store. A module with ANY import is refused (no host functions at all; maths comes compiled into the module, e.g. Rust `libm`). Determinism: the same module and input give the same bits on every machine.

### 8.2 Exports (ABI version 1)
```
memory                                   exported linear memory
vep_abi() -> i32                         must return 1
vep_prepare(sample_rate: i32, channels: i32, max_frames: i32) -> i32     0 = ok; called once per instance
vep_io() -> i32                          byte offset of an f32 buffer [max_frames * channels], interleaved: input and output (in place)
vep_io2() -> i32                         transitions only: second buffer (incoming)
vep_params() -> i32                      byte offset of an f32 array [2 * P]: values at slice start, then values at slice end (8.3)
vep_reset()                              forget all state (after a jump in time)
filters:      vep_process(frames: i32) -> i32
transitions:  vep_transition(frames: i32, offset: i32, full: i32) -> i32   // offset = samples since the fade start, full = fade length in samples (today's apply_partial_fade arguments)
sources:      vep_generate(frames: i32, t0: f64) -> i32                     // t0 = clip-local time in seconds of the first frame
```
Return value 0 = ok; anything else disables the block for the session with that code in the message.

### 8.3 Parameters
P = the number of f32 values the numeric parameters flatten to, in manifest order (float, int, bool 0/1, enum index: 1; vec2: 2; vec3: 3; color: 4). The host writes the values at the slice start and at the slice end (evaluated at those exact times), so a block can ramp smoothly inside the slice (owner decision D2). A block that ignores the end values is stepped every 128 samples.

### 8.4 Slicing, determinism and state
- The mixer processes every block in fixed slices of 128 frames aligned to ABSOLUTE sample positions on the sequence's timeline (slice k covers samples [128k, 128k+128)), whatever the caller's chunk size. Preview and export therefore call blocks identically for the same span.
- One instance per (node, clip) per mixer session; state persists across contiguous chunks. After a discontinuity (seek, start of an export range, start of a clip) the host calls `vep_reset` and pre-rolls `warmup` samples (declared in the manifest, <= 4 s) from the earlier of the clip start and the new position minus warmup, discarding the pre-roll output. Blocks without memory declare warmup 0 and are bit-identical between preview and export; blocks with memory are identical once playback has run for their warmup after a jump. Document this honestly.
- Manifest: `"audio": { "module": "block.wasm", "warmup_ms": 0, "latency_samples": 0, "tail_ms": 0 }`. Latency and tail are recorded; version 1 does not yet compensate latency or extend clips by the tail (say so in the docs).
- Fuel: 2,000 fuel per frame per channel per call plus 100,000 per call; running out traps. A trap, a non-zero return, or a NaN/infinite output disables the block for the session with a message; the slice passes through unchanged (filters), silent (sources), or with the host's linear crossfade (transitions).
- Memory initial size <= 64 pages (4 MiB) at load, maximum 16 MiB.

### 8.5 Audio transitions
`vep_transition` gets the outgoing clip's slice in `io` and the incoming clip's slice in `io2` and writes the mix into `io`. `offset` and `full` are today's fade arguments (mixer.rs:373-383), so the built-in crossfade reproduces today's per-sample fade.

### 8.6 Where sound blocks run (mixer structure)
Per clip: source (asset decode with speed and reverse, or an `audio.source` block) -> the clip's filter nodes in stack order -> the track's filter nodes in stack order (track-level nodes keep today's meaning: applied to each clip, timed from the clip's start; a real track mix bus is left for later) -> transitions at cuts -> sum into the master -> sequence-level filter nodes on the master, timed in sequence time (new: today sequence sound effects are ignored) -> clamp to [-1, 1]. Mute and solo as today.

## 9. The project model (core-model) and schema version 3

### 9.1 Types
```rust
pub struct BlockId(String);                 // validated as in section 3; core-model knows nothing else about it
pub struct EffectNode {                     // field order is the file order: keep it
    pub id: EffectId,
    pub block: BlockId,
    pub version: u32,                       // the block's "format" this node's parameters follow
    pub enabled: bool,
    pub params: OrdMap<ParamId, Param>,
    pub mask: Option<MaskRef>,              // legacy from schema 2: never drawn (it was never drawn before either)
    pub blend: Option<String>,              // legacy from schema 2: never drawn
    pub note: Option<String>,               // migration notes ("was dip_to_black"); shown in the inspector
    pub provenance: Option<Provenance>,
}
pub struct Transition { pub block: BlockId, pub version: u32, pub duration: RationalTime,
                        pub params: BTreeMap<ParamId, ParamValue>, pub note: Option<String> }
pub enum ClipSource {                       // internally tagged "type"
    Asset { asset_id, stream_index },
    Block { node: EffectNode },             // generated content: a video.source or audio.source node
    Adjustment,                             // applies its effects to everything beneath (core; draws nothing itself today)
    Silence,                                // an empty sound clip (core)
}
```
`EffectKind`, `EffectDomain`, `BlendMode`, `TransitionKind` and `GeneratorKind` are removed from the live model (kept only inside the v2 migration as frozen string tables). `ParamValue`, `Param`, `Keyframe`, `Interp`, `MaskShape`, `MaskRef`, `Provenance` are unchanged.

### 9.2 The time remap stays a core node
The time remap stays an `EffectNode` in `Clip.effects`, at the SAME list position, with `block = "core.time_remap"`, `version = 1`, and the same `speed`/`reverse` parameters. Reason: saved journals identify effects partly by list position (`EffectId::derived(new_id, "split", i)`, core-commands ops/helpers.rs:379-381), so moving it out of the list would break replay of existing journals. `core-model/src/time_map.rs` matches on the reserved id instead of `EffectKind::TimeRemap`. The registry never lists `core.*` ids; the inspector shows speed in the clip section as today.

### 9.3 What the engine checks
`core-commands` keeps only STRUCTURAL checks that need no manifest: ids unique, numbers finite, keyframes strictly increasing with values of the same type as the parameter's value, masks well formed, transitions positive and not longer than the clip, block ids well formed, time-remap placement (one per clip, clips only). It no longer calls any registry (today it does: ops/effects.rs:82,103,207-212; ops/helpers.rs:178-191), and `SetKeyframes` on a parameter the node does not have creates it with the FIRST keyframe's value as its base value (no registry default). The domain check (a video block on an audio track) and every range, type and enum check move to the edges (section 11). Reason: replaying a journal must never depend on which blocks are installed (maps/model-and-engine.md section 5).

### 9.4 Migration from schema 2 to 3 (`core_model::codec::migrations`, on the JSON tree)
The house rule holds: migrations copy the constants they need (the old registry defaults, kind names) instead of importing live code.

Effect nodes (`"kind"` becomes `"block"` + `"version": 1`; unknown fields dropped as today; `blend` value kept as its string; `note` null unless stated):

| schema 2 kind | schema 3 block | notes |
|---|---|---|
| transform | chadfx.transform | |
| crop | chadfx.crop | `feather` kept as a hidden parameter (inert, as today) |
| composite | chadfx.opacity | `blend` parameter kept as a hidden parameter (inert, as today) |
| color_matrix | chadfx.color | |
| mask | chadfx.mask | |
| time_remap | core.time_remap | same position, same params |
| audio_gain | chadfx.gain | |
| audio_pan | chadfx.pan | |
| blur, curves, lut, text, shader, audio_eq, audio_compressor, audio_time_stretch | chadfx.v2.<kind> | not installed anywhere: shown as "not available yet (from an older version of the editor)"; draws or sounds nothing, exactly as today. The NEW blur is `chadfx.blur`, a different id, so old nodes do not start blurring. |

Transitions (`"kind"` becomes `"block"` + `"version": 1`; `params` kept):
- on a clip of a VIDEO track: every kind (cross_dissolve, dip_to_black, dip_to_white, wipe, slide, zoom, custom) becomes `chadfx.cross_dissolve`, because every kind draws as a dissolve today; for any kind other than cross_dissolve, `note` = "was <kind>" (custom: "was custom: <name>").
- on a clip of an AUDIO track: every kind becomes `chadfx.crossfade`, with the same note rule.

Generators:
- solid_color: `{"type":"block","node":{id: EffectId::derived(clip_id,"source",0), block:"chadfx.solid_color", version:1, params:{color: <clip.metadata.color clamped 0..1, alpha default 1, white if absent or malformed>}}}`. Today's graph.rs:185-200 rule, including its default; a malformed metadata value that panics today (graph.rs:193) becomes white. The clip's metadata is left untouched.
- color_bars: `chadfx.solid_color` with colour (0.5, 0.5, 0.5, 1) and note "was color_bars" (that is what is drawn today).
- text: `chadfx.v2.text` (not available: draws nothing, as today), note "was a text generator".
- tone: `chadfx.tone` with frequency 440 and amplitude 0.25 (today's constants, mixer.rs:197-204).
- adjustment: `{"type":"adjustment"}`; silence: `{"type":"silence"}`.

Neutral colour nodes (owner decision D1): every clip on a VIDEO track (asset clips and source-block clips; not adjustment clips) and every sequence gets ONE extra node APPENDED AT THE END of its effect list: `{id: EffectId::derived(<clip or sequence id>, "neutral_color", 0), block: "chadfx.color", version: 1, enabled: true, params: {}, note: "added by the 2026 upgrade so the picture stays exactly the same"}` — but only if the list has no enabled `chadfx.color` node already (folding makes extra neutral nodes harmless anyway, but do not add clutter). Appending (not inserting) keeps every existing node's list position, so position-derived ids in old journals still match. Reason: today's shaders run the colour maths on every layer and on the output even with no colour node, and skipping it changes a few pixels per HD frame (maps/renderer.md section 6, variant V4).

### 9.5 Old journals
`core-project` replays journal rows written under schema 2 through a matching TRANSACTION migration before deserialising: every embedded effect node, transition, clip source and new clip gets exactly the 9.4 treatment (including the neutral colour node for clips and sequences the transaction creates, with the same derived id). It must be proven on a real schema-2 project file with a journal made by today's build (Stage 0 fixture). Hash checks for old-schema journals stay disabled as today.

### 9.6 Things that must keep working
`referenced_blob_keys`/`referenced_artifact_keys` still see parameters of missing blocks and of transitions (today transition params are not scanned: add them). `shift_keyframes` on head trims still shifts every node's keyframes. Derived ids on cut and duplicate unchanged. Undo, the journal, crash recovery and exact rational time unchanged. Canonical JSON and state_hash: struct field order as declared above.

## 10. Versions and migrations of blocks
- A node stores the block's `format` it was written with (`version`). The registry holds ONE installed version per id (installing another version replaces it after confirmation).
- `migrations` in the manifest, applied in memory, in order, to bring an older node up to the installed `format`:
```jsonc
"migrations": [
  { "from": 1, "to": 2, "steps": [
      { "rename": { "from": "size", "to": "radius" } },
      { "scale":  { "param": "radius", "by": 2.0 } },             // float, int, vector components; keyframes too
      { "offset": { "param": "angle", "by": 90.0 } },
      { "enum":   { "param": "mode", "map": { "box": "fast" } } },
      { "default":{ "param": "quality", "value": 1.0 } },          // a new parameter's value for old nodes
      { "remove": "legacy_flag" } ] } ]
```
- A node whose `version` is greater than the installed `format` is "needs a newer version": treated as missing. A node whose version has no migration path is "cannot be converted": treated as missing.
- Migration happens in memory for rendering, mixing, the inspector and the AI. The project file changes only when the user (or later the AI) edits that node: the edge then writes the migrated parameters and the new version in the same transaction.

## 11. Edges: where manifests are enforced
`crates/blocks` provides `validate_node(&Catalog, &EffectNode, place)` (known block, kind allowed at that place and media kind, parameter ids known, types, ranges, enum values, keyframable) and `prepare_node` (fills nothing; manifests' defaults stay implicit). The app service validates every transaction and EditOp result against the catalogue BEFORE committing, and the command-line tool does the same. Nodes of missing blocks may be moved, deleted, enabled, disabled and reordered, but their parameters cannot be changed. The engine never sees the catalogue.

## 12. The registry
- Built-in packages: folder packages in `<resource dir>/blocks/builtin/` (Tauri `bundle.resources`; on Windows the exe's folder). Overridable with the environment variable `VEP_BUILTIN_BLOCKS` (tests, the end-to-end harness, the command-line tool in development). In the repository they live in `blocks/builtin/<id>/`.
- User packages: `<config dir>/blocks/` (the app's config dir rule, ui/src-tauri/src/settings.rs; `VEP_CONFIG_DIR` isolates it). Installed packages are stored as `.chadfx-block` zip files named `<id>.chadfx-block`; installing a folder packs it into a zip after validation. State (disabled and quarantined blocks with reasons, the in-flight file) in `<config dir>/blocks/state.json` and `<config dir>/blocks/in-flight.json`.
- Ids: `chadfx.*` only from the built-in folder; `core.*` never. A user package with a built-in id is refused.
- The catalogue is an immutable snapshot (Arc) with, per block: manifest, origin (built-in or user), package hash, status (`ok`, `invalid` with reasons, `disabled` by the user, `quarantined` with reason), and the compiled forms (validated WGSL, validated WASM module bytes). A change produces a new snapshot and an event.
- Missing-block status of a node (for the inspector, export and the AI): `available`, `missing` (not installed), `needs_newer`, `cannot_convert`, `disabled`, `quarantined`, `invalid`, `not_available_yet` (the `chadfx.v2.*` ids).

## 13. Missing blocks everywhere
- Renderer: a node that is not `available` is skipped (a missing source block draws nothing; a missing transition becomes a cut; a missing blend falls back to normal). Same in preview and export.
- Mixer: skipped (missing source: silence; missing transition: hard cut).
- Export: `ExportReport.missing_blocks` (static list of nodes and their status for the exported range) and runtime block problems (`BlockProblem`: block, node, kind, count, first/last time, message), like `missing_media` and `MediaProblem` today; the window shows them as warnings before and after export.
- Inspector: a section titled with the node's stored name if known, else the id, saying why it is unavailable, with enable/disable/remove/move still working; parameters shown read-only from the stored values.
- AI view (core-query): `EffectView` carries `block`, `version`, `title` (from the catalogue if available) and `status`; `describe_timeline` takes a catalogue view (a trait object `BlockNames`) and prints missing blocks as such.
- Saving: the node is written back exactly as read.

## 14. Built-in blocks in this task
Manifests copy today's registry values exactly (effects-registry/src/lib.rs:143-237) for defaults and ranges; the renderer falls back to those defaults today, so they affect pixels.

| id | kind | stages / notes |
|---|---|---|
| chadfx.transform | video.filter, fold last | place (today's vertex maths), pixel: none; `opacity` role; scale/position/rotation/anchor roles; `default_for: placement` |
| chadfx.crop | video.filter, fold last | region; `feather` hidden (inert) |
| chadfx.opacity | video.filter, fold none | `opacity` role only (no stage code needed: a block may have zero stages); `blend` hidden (inert) |
| chadfx.color | video.filter, fold params | pixel: today's `grade()` exactly; exposure add, contrast multiply, saturation multiply, temperature add, tint add, lift add, gamma multiply, gain multiply (graph.rs:57-68) |
| chadfx.mask | video.filter, fold last | pixel: today's `mask_value()` exactly, path as bounding rect, corner radius ignored, feather normalised by layer width |
| chadfx.cross_dissolve | video.transition, fade | today's dissolve |
| chadfx.solid_color | video.source | returns the colour parameter |
| chadfx.gain | audio.filter | `gain_db` role; today's `10f32.powf(db/20)` |
| chadfx.pan | audio.filter | `pan` role; today's equal-power pan (stereo only, 1.0 per channel otherwise) |
| chadfx.crossfade | audio.transition | today's per-sample linear fade |
| chadfx.tone | audio.source | today's `sin((t0 + i/sr) * 440 * TAU) * 0.25` in f64, with frequency and amplitude parameters |
| NEW chadfx.blur | video.filter | pass stages: separable Gaussian, radius in sequence pixels |
| NEW chadfx.glow | video.filter | pass stages: threshold, downsampled blur, add back |
| NEW chadfx.wipe | video.transition, mix | a soft-edged directional wipe (angle, softness) |
| NEW chadfx.blend | video.blend | mode enum: add, multiply, screen, overlay, soft_light, hard_light, darken, lighten, difference |
| NEW chadfx.eq3 | audio.filter | three-band EQ (low shelf, peak, high shelf) with today's AudioEq parameter names; warmup declared |
| (normal blend) | video.blend | `chadfx.normal`, `fixed_function: premultiplied_over`; the default when no blend node exists is the host's over, not this node |

Audio built-ins are written in Rust (`#![no_std]`, `libm`) in a separate cargo workspace at `blocks/src/` (excluded from the main workspace), compiled to `wasm32v1-none` with a small stack (`-C link-arg=-zstack-size=65536`), and copied to `blocks/builtin/<id>/block.wasm`. The compiled files are checked in; a test compares a recipe hash (BLAKE3 of sources, Cargo.lock, toolchain version and flags) so stale files are caught, and rebuilds and compares bytes when the pinned toolchain is present.

## 15. Interface, command line, AI
- Block manager (new feature folder in ui/src): install from file (dialog and file drop), list (built-in and installed; status), remove (user blocks only; warn when the open project uses it), errors, quarantine reasons with "enable again", missing blocks of the open project. Fully keyboard reachable.
- Inspector built only from manifests: types, units, steps, component labels, advanced (simple mode), hidden, roles; transitions edited with their parameters; no per-block lists anywhere in the window (the hand-written GESTURE_ONLY/STANDARD/DRAWN lists and unit guessing go: maps/app-interface-and-tools.md 2.2).
- All NEW interface text goes into one central strings file (ui/src/lib/strings.ts): PRODUCT.md asks that from now on. Manifest text is shown in English for now (LocalizedText `en`).
- Command line: `vep blocks list | info <id> | install <file|folder> | remove <id> | validate <file|folder> | pack <folder> <out.chadfx-block>`.
- The AI: a runtime query (`blocks_catalog`) returns every available block with its description, parameters and roles; the build-time schemas no longer list effect kinds (they are strings).

## 16. Stages and acceptance (each ends with: zero warnings in `cargo build --workspace --all-targets`, `cargo test --workspace` green, and in ui `npm run build`, `npm run lint`, `npm test`, `npm run e2e` green)
0. Backup (done). Golden pictures and sounds from TODAY's code (crates/golden). A schema-2 project file with a journal, made by today's build.
1. `crates/blocks`: manifest types and JSON Schema, package reader, WGSL validator with loop budget, WASM validator, registry (folders, install, remove, state), migrations engine, `vep blocks list|info|validate|pack`. Built-in manifests and WGSL for the picture blocks of section 14 (code may be finished in Stage 3). The `blocks/src/` WASM workspace and the gain/pan/crossfade/tone/eq3 modules with the staleness test.
2. Model, engine and journal: section 9; core-query and core-edit adapted; the edges validate (11). Renderer and mixer adapted with the smallest change (match on the new ids) so everything still renders and mixes exactly as before: prove with the goldens.
3. Renderer runs picture blocks generically (section 7). All id-specific code in render goes. Goldens byte-exact on this machine (NVIDIA Vulkan and WARP). New picture blocks: blur, glow, wipe, blend.
4. Mixer runs sound blocks generically (section 8). All id-specific code in audio goes. Golden sounds identical for static cases (bit-exact or within the rounding statement: about one part in ten million, measured and reported); keyframed gain and pan smooth and identical in preview and export. New sound block: eq3.
5. App backend (registry service, commands, events, edges, missing-block reporting, export warnings), command-line install/remove, interface (block manager, inspector from manifests, transitions with parameters, central strings).
6. Safety tests (malicious and broken blocks of every kind are contained), the end-to-end test that installs a new block from a package file into the running app, uses it, exports and checks the result, docs/BLOCKS.md and a template folder, README, ARCHITECTURE (new implementation notes), PRODUCT.

## 17. Notes from the brief review (docs/REVIEW-2026-10-29.md, 2026-10-29)
- Stage 2: when a clip is cut or duplicated, its new SOURCE node (ClipSource::Block) gets a fresh id `EffectId::derived(new_clip_id, "source", 0)`; source nodes count in every id check and scan: `locate_effect`, `contains_ulid`, `all_effects`, uniqueness checks, `referenced_blob_keys`, `referenced_artifact_keys`, `shift_keyframes` (crates/core-model/src/project.rs:142-317). Write a checkpoint before any new journal row lands on a checkpoint migrated from schema 2, so one journal never mixes formats.
- Stage 3: reuse the intermediate pictures pass stages draw into (keep them across frames like today's layer slots). Contract line: colour values above 1.0 are legal in the working space and are never clamped before the output pass.
- Stage 5: one shared simple-or-pro flag in the existing preference store (ui/src/state/prefs.ts) used for "advanced" parameters; whole messages (not fragments) in ui/src/lib/strings.ts; package README and manifest text shown as PLAIN TEXT only (never as HTML or rendered markdown); set a strict content security policy in ui/src-tauri/tauri.conf.json (today "csp": null).
- Kept as written after the skeptics: the slow-frame tripwire is an absolute time (the graphics driver's watchdog is the same on every machine), and sound blocks are limited by deterministic fuel only, never by wall-clock time.

## 18. Clarifications from the Stage 0 checker (2026-10-29)
- Sequence-level nodes: today only an ENABLED ColorMatrix acts at sequence level (render/src/graph.rs:331-333) and the mixer reads no sequence node at all. So the 9.4 migration writes every sequence-level node OTHER than color_matrix (picture: transform, crop, composite, mask; sound: audio_gain, audio_pan; any other kind) with `"enabled": false` and the note "was ignored at sequence level before the upgrade". Their block ids follow the normal table. The schema-2 journal treatment (9.5) applies the same rule to sequence-level nodes added or enabled by old journal commands (an old SetEffectEnabled(true) on such a node stays a no-op for rendering: write the node disabled and keep it disabled).
- Neutral colour nodes go to asset clips and source-block clips on VIDEO tracks only: not adjustment clips, not silence clips, not clips on sound tracks.
- A picture generator on a sound track (only possible in hand-edited files) becomes a video.source node on an audio track: the mixer keeps it silent but it still counts as a neighbour under transitions, exactly as today.
- `VepCtx.layer_size` is the DISPLAYED size (render::displayed_size), never the decoded texture size: today's mask feather is divided by the displayed width.
- The built-in color and mask blocks must copy today's shader limits exactly: gamma clamped to at least 0.01, the normalised feather to at least 1e-5, and mask opacity NOT clamped (opacity 1.5 gives alpha -0.5 outside the mask).
- The Stage 0 data must never be regenerated: the fixture generator ran the `vep` binary, which now validates differently.

## 19. Changes from Stage 1 (2026-10-30; details in stages/s1-*.md)
New rules from the security review, binding from now on:
- WGSL: no statement or declaration may hold more than 2048 operators (`wgsl_expression_too_long`; naga recursed without limit and a 20,000-term chain aborted the process). The 16 KiB per-value limit covers module-scope constants. Loop-budget unit: 1 per expression evaluated and per other statement, +16 per picture read, +1 per 16 bytes of an array or struct value loaded, stored, built or indexed by value with a computed index. A stage's straight-line cost (calls included) is at most 4096 units. ASCII only outside comments. No module-scope name may hide a WGSL built-in. At most 64 KiB of uniforms per composed draw: if a layer's blocks would exceed it, Stage 3 skips the block that does not fit (with a status), never fails the frame.
- The checker runs on its own 8 MiB thread (naga overflows small stacks). Every guard relies on panics unwinding: no build profile may ever set `panic = "abort"`.
- WASM: no function may declare more than 1024 locals (`wasm_too_many_locals`). wasmi runs with `portable-dispatch`, eager compilation, custom sections ignored, one engine per validated module. Validation ends with a trial run on silence at 48 kHz stereo. Memory: initial at most 64 pages; growth capped at 16 MiB by the store limiter.
- DECIDED (2026-10-30): keep `portable-dispatch`. The faster `auto-dispatch` (about 3 times faster) can overflow the native stack in non-optimised builds and abort the process, which breaks rule 2. The cost is small: eq3 uses about 1.7% of one core per stereo stream.
- Registry: an id is never used as a path; the in-flight record is deleted only after the quarantines it causes are saved; refresh writes the in-memory state back when state.json has become unreadable. Manifest version at most 64 bytes. The command line escapes control characters in every human-readable line.
- VepShape has a 4th member `bounds: vec4<f32>` (the host's f64 bounding rectangle), needed for byte-exact masks.
- Sound rounding statement, corrected: for static cases, every sample is bit-exact or within 3 units in the last place of a 32-bit float (under 3 parts in ten million of the sample's value). "About one part in ten million" holds only for a single gain block. Blocks with memory (eq3) agree with uninterrupted playback to about 1e-12 after their warm-up, not bit for bit.
- chadfx.tone: `frequency` is not keyframable (today's formula has no memory). Past about 10 minutes of clip time at 440 Hz it uses a cheaper phase path that differs from today's formula by at most 1.5e-8.
- eq3: RBJ low shelf and high shelf, and a peak at sqrt(low x high) with Q = 1/sqrt(2); gains clamped to plus or minus 24 dB; exact pass-through at 0/0/0 dB.
- Known, accepted: folder packages could contain hard links (not detectable safely on Windows); fused blocks in one layer share one loop budget per pixel (a hostile block can only spoil its own layer's picture); Unicode bidirectional characters in manifest text: Stage 5 must show names so that they cannot disguise themselves (for example isolate them with `unicode-bidi: isolate` and show the block id next to the name).

## 20. The picture proof that needs no graphics card (2026-11-01; details in stages/ng-*.md)
The owner asked: "Find a way to do this picture reference test without relying on anything graphics card or operating system native; this should be possible and compatible with Windows, Mac, and Linux." Binding from now on:
- **Replaces Stage 3's acceptance line** "goldens byte-exact on NVIDIA Vulkan and WARP". Stored card pictures can only be compared on the card that made them, and most computers have none of those cards. Stage 3 is accepted when: `crates/render/tests/no_gpu_pictures.rs` draws all 351 scenes and the known panic through today's code and the software renderer, byte-exact with `crates/golden/data/render-portable`, no tolerance and no skip; `same_card.rs` gives the same bytes as the frozen old shaders on the computer's own card, both the way the preview draws and the way the export draws; and `card_vs_cpu.rs` stays within its bound. `no_gpu_recipes.rs` is retired by Stage 3 as its header says, with the reason in the report; never regenerate `recipes.json` or teach it new recipes.
- **What stays.** `crates/golden/data/render` and its tests stay untouched. `golden_pictures_software.rs` on WARP remains a check: on this PC it reproduces all 351 scenes. A card with no stored set prints that it compared nothing, which proves nothing.
- **New crates.** `crates/render-recipe` is one frame's drawing instructions as plain data. `crates/cpu-render` is a pure-Rust software renderer that interprets the real WGSL, with its arithmetic defined in its README. It is a development dependency only and never ships, and tests fail if a shipped crate links it. `crates/golden/data/render-portable` holds the old code's frozen recipes and shaders and the pictures drawn from them; `recipes.json`, `references.json` and the capture record are pinned by BLAKE3. `make_portable_references --write` is only for a change to the arithmetic that has been proven and reported.
- **Not yet proven.** No Mac, no Linux, no ARM processor and no AMD, Intel or Apple card has run this. The promise rests on Rust's exact float operations, the pinned `libm` and the pinned hashes. WebAssembly under Node matches but runs on the same processor. The check on a new machine is `cargo test -p cpu-render -p golden` plus `no_gpu_recipes` and `no_gpu_pictures` in `render`.
- **DECIDED, Stage 3: the block checker refuses what the software renderer refuses.** A block that works on a card but cannot be proven here must not be installable. `crates/blocks` adds: calls the parser folds from constants (`sin` through `fma`, `min`, `max`; `radians` and `degrees` except a literal zero; `clamp` and `saturate` except plain numbers with no minus sign), because naga's folded value depends on how it was built; and a loop whose `continuing` block reads a value a `continue` skipped, which WGSL forbids, naga accepts, and which gave different bytes from run to run. Keep the same tokenising rules as `crates/cpu-render/src/front.rs` (naga also ends a comment at VT, FF, CR, U+0085, U+2028 and U+2029). The built-in blocks already pass.
- **Also for Stage 3.** `frame_recipe` requires exactly today's bindings, so pass modules need it widened. Add `crates/blocks` to the maths search in `no_gpu_pictures.rs`. Keep the parse at `crates/blocks/src/wgsl/compose.rs` behind a guard, because naga 30.0.1 panics on some constant `select` expressions. Any block that reads the composite's alpha needs its own proof, because no picture shows that alpha.
- **DECIDED: the two sampler differences stay, written down.** With a nearest sampler, a card's `textureSampleBaseClampToEdge` uses nearest and the software renderer uses linear. With different minification and magnification filters, a card uses the minification filter at an explicit level or large gradient and the software renderer does not. No block can reach either: blocks have no samplers of their own, and every sampler the host binds is linear with equal filters. The proofs are kept as ignored tests in `crates/cpu-render/src/wgsl_spec_tests.rs`. Change this if the host ever binds another kind of sampler.
- **ACCEPTED: `card_vs_cpu`'s bound is not the anchor test's rule.** Which bytes differ by one level says only which way a card rounds, so the anchor's rule would fail an honest card. Its limits on bytes off by 2 and by 4 levels are what catch the misdraws listed in its header, no alpha byte may differ, and the limit on any difference over a run is 11.276%, from a one-off measurement of simulated rounding rules whose numbers are in the test's comment and whose scratch copy is not kept. It misses a final display step 1 part in 1,000 too bright; `same_card` catches that on a card. Widen it only with a new measurement.
- **Graphics errors are not "no card".** Only `RenderError::NoAdapter` may skip a test. Any other error from `Renderer::new` fails it, in every crate.
- **Every wait on FFmpeg or ffprobe has a limit.** Product code and test code go through `media::ToolWatch` or `OutputWithLimit`, which stop a tool that stays silent for 5 minutes, or 120 seconds in tests. `nothing_waits_for_a_tool_except_through_a_watch` enforces it. A stuck decoder can hold the preview for up to those 5 minutes.
- **Undo and redo.** `Project::jump` writes at most one checkpoint however far a history jump goes past the save point.

## 21. Rules settled by the Stage 2 review (2026-11-02; details in stages/s2-review-*.md and s2-acceptance.md)
The reviewers found cases the sections above do not settle. Binding from now on:
- **The time remap's limits live at the edges.** It has no manifest and the engine checks structure only, so `crates/blocks/src/node.rs` holds the old catalogue's rules, frozen: `speed` a float from 0.001 to 1000 that can be animated, `reverse` a switch, `frame_blend` one of none, blend, flow, nothing else, and format 1 only. A test compares them with the old catalogue. A node already outside these limits can still be read, edited within them, copied and replayed. The engine and `validate` refuse a transition that names the time remap.
- **A move across track kinds is the edges' job.** A clip moved to the other kind of track takes its source, effects and transitions with it, and each is checked at its new place. Only the place is judged. What the catalogue cannot judge (a block it does not have) may move, and that includes the old `chadfx.v2.text`, which draws nothing either way.
- **A node whose block cannot be used is locked whole.** Besides its parameters (section 11), its legacy mask and blend are refused too, unless the command changes nothing. Moving, removing, switching and reordering it stay allowed.
- **A node made earlier in the same transaction is checked like any other,** such as the right-hand part of a cut edited in the same transaction.
- **A new node takes the installed format of its block,** in `vep effect add` and in fit to frame. The window's `raw.addEffect` and `dissolve()` still send none, so format 1 is assumed: Stage 5 must fix that before any built-in moves to a later format.
- **The copy rule stays.** An exact copy of an existing node, including one whose block cannot be used, may be written. A copied node with different keyframes counts as new (Stage 5 decides whether that stays).
- **Reports name what is in the wrong place.** `vep validate` and `vep info` report `block_wrong_place`, and `vep validate` then exits 1. The app's report on opening a project names such blocks too. Blocks the catalogue does not have are not reported. The check exists twice (`crates/cli/src/catalogue.rs`, `ui/src-tauri/src/service/project.rs`); moving it into `blocks::edges` is the tidier home.
- **The window shows every unusable node.** Source, track and sequence nodes appear as read-only notes (`node-unavailable`), and a transition as a "cannot be used" chip (`transition-unavailable`). Stage 5 adds the controls to switch or remove them.
- **Upgraded files.** Replay keeps "replay equals migration": when an old row moves a clip to the other kind of track, its transition takes that track's block. One stated exception: an old `SetKeyframes` or `UpdateEffect` mixing value types, possible only in a hand-edited file, replays with the base value taking the keyframes' type or with the wrongly typed keyframes dropped, because the mix cannot be represented. A colour component too large for single precision reads as the largest there is, because infinity cannot be written back to JSON. `validate` counts the project's own id among the unique ids.
- **New permanent data.** `crates/core-project/tests/fixtures/v2_random` holds 8 projects made by the real old code, with the generator's source. A pinned state hash in `old_build_journals.rs` catches any change to a field's place or spelling, because hashes in people's journals depend on it. Never regenerate either.
- **Reference runs must really compare.** A picture run may compare nothing only if no graphics card on the computer has a stored set. The counts are exact (17 documents, 351 scenes, 1 known panic, 447 source requests; 118 sound cases, 590 references) and both index files are pinned by BLAKE3.
- **The export's missing list follows what the output uses:** for pictures, the frames the export really has, the incoming fade alone where a clip's two fades overlap, and only blocks of the media being exported. For sound, the whole range.
- **Until Stages 3 and 4 the list says more than happens.** A missing transition still dissolves and a turned-off built-in still draws or plays, though the list, the window and the AI's text describe the later behaviour (section 13). The new built-ins (blur, glow, wipe, blend, eq3) do nothing and are not offered. Stages 3 and 4 must make all of this true.
- **Judging sound from a damaged file.** FFmpeg's decoder is repeatable on damaged data, but its gap filling (`aresample=async=1`, which `crates/media/src/audio.rs` uses for every sound) is not when frames are missing: 64 runs with 24 at once gave 9 different results, and 1 without that filter. So an export of a damaged file can differ slightly between runs, and the tests compare it within 2 hundredths of a second's audible share against up to 32 plain decodes and up to 8 exports. A second really lost disagrees every time. Undamaged files gave one result in 64 runs.
- **Left open, recorded.** A hand-edited file whose derived id collides with an id already in it opens and does not crash, but `validate` reports `DuplicateId`, and adding a track or creating a sequence is refused until it is fixed. No real file can reach this, because the id is a hash of its owner. The neutral colour node prints as "Color" on every upgraded clip in the AI's text: true but noisy; the AI work decides.

## 22. Rules settled by Stage 3 (2026-11-03; details in stages/s3-*.md)
The builders, the three reviewers and the main session settled these. Binding from now on; where they differ from sections 7, 13 and 19, this section wins.
- **The checker refuses what the software renderer refuses.** `crates/blocks/src/wgsl/portable.rs` repeats cpu-render's parse-time rules (cpu-render must never ship, so they are copied, with a test that fails if the two drift), and `crates/blocks/tests/wgsl_parity.rs` feeds one list of sources to both. New refusals: `wgsl_no_return` (a function that reaches its end through a `switch` `break`, which naga accepts), `wgsl_parser_stopped` (naga panicked; the parse is guarded), `wgsl_too_many_blocks` (more than 64 blocks with code in one draw), `wgsl_draw_too_costly` (a draw's whole code over 2^18 units), and cpu-render's text and data limits. At most 256 `else if` in one function: naga's validator and SPIR-V writer overflow a 1 MiB stack at 1,024. Operators are counted exactly as cpu-render counts them, so a table of more than about 2,000 negative numbers must be split.
- **The unit of cost (amends 19).** On top of 1 per expression and statement, 16 per picture read and 1 per 16 bytes of an array or struct moved: a call or a nested loop costs its whole code (a loop that never runs is not free), by-value arguments and results and every local and constant are charged by their bytes, 1 per `switch` case, and matrix arguments by their bytes. One unit is at most 33.5 steps of the software renderer (measured).
- **A stack too large for one draw loses blocks, never the frame.** The renderer composes with `compose_layer_fitting` and `compose_output_fitting`; what they leave out, and at most 64 blocks with pass stages per stack, is listed by `render::left_out`. A stack the composer refuses for any other reason is drawn without its nodes.
- **The renderer draws only what the catalogue has.** `crates/render/src/builtin.rs` and the old shaders are gone; the renderer depends on `blocks` and reads a replaceable catalogue snapshot (`CatalogSlot`). Programs are cached by the stack and by their full text, not by `Composed::key`; pipelines by BLAKE3 of the text with entry points, format, blend and binding shape.
- **Fade curves are evaluated in f32 (amends 7.6),** from the progress made f32. Evaluated in f64 a fade-out differs from today's `1.0f32 - u` in the last bit, and every dissolve curve reproduces today's factor exactly in f32.
- **Reading hand-edited values.** A whole number where a float is declared is read as that number. A node holding something else where its block declares a shape is skipped whole (the old mask reader kept the previous mask). Every other mistyped value takes the manifest's default.
- **Where pass stages run.** In a layer, every pixel stage runs in the layer's one draw before any block with pass stages, whatever the stack order, so `[Blur, Mask]` draws exactly as `[Mask, Blur]` and a mask's edge is always blurred. At sequence level all pass stages run on the composite first and the pixel stages in the output pass. The window's "applied later" wording is therefore wrong next to a pass block: Stage 5 decides between a second draw for pixel stages after a pass block and saying so in the window.
- **Which blend applies.** The last usable blend node decides; an unusable last one gives way to an earlier usable one. A blend whose program cannot be made, and a block dropped by `safety::without`, give the normal blend. A sequence-level blend is ignored (nothing is beneath it).
- **A mix transition lays its result down with the blend both sides share (amends 7.6).** When every side that has a clip has the same blend (same block, same code, same values: a track's blend, typically), the transition mixes the two into an intermediate, which is laid down with that blend like a layer. A side with no clip agrees with any blend, because a blend leaves what is beneath unchanged where nothing is drawn. Otherwise the result is laid over normally, and a clip's own blend waits until the transition is over. A transition whose block cannot be used, or whose program cannot be made, is a cut: its own clip, with progress 0. The context a mix transition sees has the sequence's size as the layer size and the owning clip's local time.
- **Built-in blur and glow read every texel of their reach,** two texels per bilinear sample, placed between them so the pair is weighed as the Gaussian weighs it. The blur has four passes: `blur_h` and `blur_v` blur exactly up to 120 texels each side; `wide_h` and `wide_v` add the rest of a larger blur on the picture the first two have already smoothed (sigma 40 texels), in steps of at least 5 texels and at most 60 samples a side, which leaves no fine detail behind up to about 3,600 texels; they only copy for a smaller blur. Together the four passes are the Gaussian of sigma a third of the radius to within 0.0018 (under half a level), mostly the rounding of the 16-bit pictures between them (test `a_blur_beyond_the_sharp_reach_is_still_the_gaussian_of_its_radius`). Measured on the RTX A4000 at 1080p, against the earlier two-pass blur that skipped texels: radius 8 +0.38 ms a frame (was +0.35), radius 32 +0.64 (was +0.90), radius 200 +2.45 (was +1.85). The glow's two blur passes are exact up to 120 half-size texels, which covers its whole range in any export up to 1.2 times the sequence's width; beyond that, very fine bright detail glows a little unevenly. A card places a bilinear sample to 1/256 of a texel (the software renderer does too), so such a pair may be off by up to 1/512 of the step between its two texels, and tests allow that.
- **Infinities and NaN are not defined.** Extreme colour values, or 16-bit intermediates overflowing, can make them; WGSL lets a card assume there are none, so the card and the software renderer may disagree, and a blur spreads one such pixel. Old pictures match the old code exactly. Whether the host should keep values finite is left open.
- **Runtime safety (7.9 items 5 to 8), as built and reviewed.**
  - A program that runs a block of the user is made and compiled on a worker, each step held to 2 seconds; one that takes longer, or fails, is refused for the rest of the run (not quarantined) and the frame goes on without it. Built-in programs are waited for, and a built-in that fails to compile is an error, as before.
  - Pipelines are compiled ahead when a project opens.
  - Blocks of the user are not drawn on OpenGL, nor from a catalogue with no registry link, which has nowhere to keep a crash record.
  - The crash record is written before each submission and before each compile of a program that runs a block of the user, and an entry is cleared when that submission or compile ends.
  - A crash record that names several packages of one block quarantines the block in every package.
  - A quarantine that cannot be saved holds for the run, and the block stays named in the crash record, so the next start quarantines it again.
  - The tripwire judges a block's share of a frame: the time of every draw that runs it, added up and scaled to a full-HD band's worth on larger outputs. A share over 250 ms is a strike, and three in a row quarantine. A share, or one band, over 1 second quarantines at once, and the frame is drawn again without the block.
  - **On a software adapter (WARP) the tripwire turns a block off for the run only, and saves nothing:** every frame is slow there, and there is no card to reset. Crash and lost-device quarantines are saved there as anywhere. Test: `crates/render/tests/block_safety_software.rs`.
  - After a lost device, every renderer that opens a replacement first quarantines the crash record of every registry the process has written to.
  - Export reports carry `block_problems`; the preview keeps its own (`RegistryLink::problems`).
- **Messages escape what disguises the text after it.** Besides control characters, the bidirectional formatting characters (U+061C, U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069) and the line and paragraph separators (U+2028, U+2029) are written as escapes: in `blocks`, in `core-model`'s block-id messages and in everything the command-line tool prints. Outside text (driver and system errors) is quoted as one plain line of at most 600 characters.
- **Proofs.** `no_gpu_recipes` is retired, because a uniform's meaning is set by the program that reads it and the programs are now composed. `recipes.json` is kept, never regenerated. `no_gpu_pictures` adds a check that no old frame's recipe changed shape. The contact sheets of the new blocks are pinned by hash in `crates/render/tests/block_pictures.rs` (outside golden data). Blur and glow were pinned anew on 2026-11-03, after their sampling changed, and looked at side by side with the earlier sheets.
- **Left open, recorded.**
  - **Packaging, before any release (Stage 5):** `ui/src-tauri/tauri.conf.json` bundles no built-in blocks, and release builds have no fallback to the repository's folder. Since this stage a packaged app or a release `vep` would draw old projects wrongly. The ignored test `ui/src-tauri/tests/blocks.rs` `a_packaged_app_ships_its_built_in_blocks` must pass first.
  - **Not shown to the person yet (Stage 5):** blocks left out of a draw and the preview's runtime problems reach no window, export list or AI text.
  - **The timeline (Stage 5):** it still paints an unusable cross dissolve with its wedge and the word "Dissolve", although it is drawn as a cut.
  - **Uniform size on smaller cards (Stage 6):** the composer's 64 KiB cap is not compared with the card's own limit (Vulkan guarantees only 16 KiB). On such a card a stack of user blocks between the two would fail rather than leave a block out. Built-in stacks are far below any limit.
  - **Smaller items:**
    - Many different hostile blocks, each under the tripwire, can still slow a frame together.
    - Programs for user stacks are made on one thread per stack, with no cap.
    - A frame planned just before another thread's quarantine can still submit the block once (not a crash path).
    - A compile that outlives its limit while the app closes normally leaves its record entry, and the next start quarantines that block (a false positive, on the safe side).
    - The software renderer's step limit on a whole draw depends on the frame's size and is not checked.
    - A helper function named like an undeclared stage is accepted silently (harmless).
    - On a track with a multiply blend, a cross dissolve is darker in its middle than either clip.
    - Where two sides of a wipe have different blends, the picture can still jump at the wipe's ends.
  - **Hardware:** everything was proven only on the RTX A4000 (Vulkan, Direct3D 12, OpenGL) and on WARP.

## 23. Rules settled by Stage 4 (2026-11-04; details in stages/s4-*.md)
The builders, the three reviewers, the acceptance checker and the main session settled these. Binding from now on; where they differ from sections 8, 13, 14 and 19, this section wins.
- **The mixer plays only what the catalogue has.** `crates/audio/src/builtin.rs` is gone; `crates/audio` depends on `blocks`, and a test fails if a block id appears in its code. A mixer session (`audio::Mixer`) reads a replaceable catalogue snapshot (`audio::CatalogSlot`, the same shape as `render::CatalogSlot`; Stage 5 should give the two one home in `blocks`) at every call. `audio::mix_range` is a one-call session on the built-in catalogue. Sound roles (`gain_db`, `pan`) do nothing in the mixer: 6.2 keeps them for later automatic mixing.
- **The grid.** Frame j of a call is absolute sample `ceil(start x rate) + j` of the sequence's timeline, and a clip spans `[ceil(start x rate), ceil(end x rate))`. Blocks run in slices `[128k, 128k + 128)` cut to each clip's extent, with parameter values at each slice's exact start and end (8.3, D2). A call reads whole slices, so it may read up to 127 frames beyond its range on each side and keeps the last slice's extra as the session's tail.
- **Speed-changed clips** are placed at their exact source positions in whole-number arithmetic. Wherever today's calls were aligned this reproduces today's samples; the chunk-dependent speeds (0.5, 0.7003, 1.0001) now give one answer however a range is cut.
- **Warm-up (replaces 8.4's sentence on pre-roll, which cannot be read literally).** After a jump to P the host calls `vep_reset` on every instance and starts the mixer at the first slice at or before P minus (longest chain warm-up + longest transition warm-up + sequence warm-up). The sequence's nodes, every transition (not before its fade's start) and every chain whose clip's sound began before that point start there, and that pre-roll is their warm-up. Every other chain that starts afresh at a sample p is reset and first run over the clip's own sound from p minus its warm-up, then its output before p is discarded. That covers a clip's start, the start of a neighbour's fade that holds it, and a chain put under the playhead during playback. The warm-up is the longest of the chain's source's and nodes' warm-ups. Before the clip's start it reads the media before the in-point through the clip's time map (silence where the file has none), or runs a source block at earlier clip-local times. All output before P is discarded. So a clip cut in two with an EQ on it, or with an EQ on its track, exports bit-identical to the uncut clip. Test: `crates/export/tests/sound_cuts.rs` and `crates/audio/tests/state.rs`.
  - Inside one chain the longest warm-up counts, not the sum.
  - A transition that declares a warm-up still starts cold at its fade's start in continuous playback.
  - The streaming source keeps the last 4 seconds and two slices it delivered, so the clip after a cut warms up on the earlier clip's decoder, not on a new FFmpeg process.
- **What is heard when.** A new catalogue snapshot, or an edit, between two calls that follow on is heard from the next slice: up to 127 frames already mixed with the old one still play. Instances of unchanged nodes keep running bit for bit.
- **The rounding statement (amends 19).** Every sample is bit-exact or within 3 units in the last place of its own value, except:
  - **sums**, where a sample adds several contributions (a transition, or several tracks): within 3 units in the last place of the largest magnitude added up (the product of a clip's levels, summed over tracks). The golden test accepts its three crossfade cases this way, at full scale. Machine-made projects with gains above 0 dB needed the larger magnitude;
  - **the tone**, at zero crossings: within 1.8e-12, and within 1.5e-8 beyond about 10 minutes at 440 Hz.

  Blocks with memory (eq3) agree with uninterrupted playback to about 1e-12 after their warm-up, not bit for bit. The golden proof checks every excused reference against the samples, with six named explanations (`crates/audio/tests/golden_sounds.rs`).
- **Values reaching a block.** Parameters reach a block as f32 (8.3), so a tone frequency that is not an exact f32 sounds slightly different from the old f64 tone (old projects hold 440 Hz, which is exact). The host makes every value finite (NaN becomes 0, an infinity the largest f32) and every input sample finite and within plus or minus 2^20 (NaN becomes 0). The built-in gain no longer clamps to its manifest's -96 to 24 dB, because the old mixer applied any value: NaN means 0 dB and the value is capped at +380 dB so the multiplier stays finite.
- **Clips that sound nothing** (silence, adjustment, a picture or unusable source, a speed ramp, speed 0, a missing asset) get no chain: their filters do not run, they give silence to a transition, and they still count as neighbours.
- **Overlapping fades.** When a clip's two fades overlap, both always play (the old code skipped one when a call ended inside the incoming fade).
- **Memory (amends 8.1).** One mixer session holds at most 512 MiB for all its instances. That counts what each instance really holds: its memory as the interpreter reserves it (up to twice its size after growing in small steps), the interpreter's records of functions, tables, globals and element items, and the runner's one-slice buffer. An instance that does not fit is not turned off: it plays its fallback, is reported as not started (`SoundNotStarted`), and starts at a later call when memory is free. A growth beyond what is left fails as at the per-instance limit. A module's element segments hold at most 1,000 items in all (`wasm_too_many_elements`), and each of its tables at most 1,000 elements.
- **A fault turns the block off for the session, on every node,** with that slice and the rest playing the fallback (8.4). In the preview a session is the open project, so a block that faulted plays again when the project is opened again; an export is a session of its own. A block that faults in every session is not quarantined, sound keeps no crash record, and user sound blocks play from a catalogue with no registry link (pictures do not). The reason: no fault of a valid module reaches past the interpreter, and a record per call would cost about 1.2 ms of disk writing on the player's thread. The residual risk is a soundness bug inside wasmi's own executor that crashes the process instead of panicking.
- **Problems.** Sound faults and refusals are reported as `BlockProblem` through `RegistryLink` (preview) and `ExportReport.block_problems` (export), with five sound kinds (`SoundTrapped`, `SoundOutOfFuel`, `SoundErrorCode`, `SoundNotFinite`, `SoundNotStarted`); a sound problem's count is in 128-frame slices, the pre-roll included. A block that faults during a warm-up is off, but its fault time is reported at the first slice the mix uses, and slices of fallback during a warm-up are not counted.
- **Preview and export.** They give the same samples when the sound device runs at the sequence's sample rate: the player mixes the export's two channels and puts them on the device's front left and right (a mono device gets their middle). A device at another rate makes the player mix at that rate (open). The player limits one pass to about 50 ms and mixes smaller calls when mixing runs slower than real time, which changes when sound is mixed, never what is mixed. Scrubbing makes no sound, as before.
- **The export list follows the samples the export holds** for sound, which can reach one sample past its range or start one sample into it (amends 21's "the whole range"). A sound transition holds its neighbour only if it can play (`audio::transition_plays`).
- **The window** offers eq3; the "not played yet" chip is gone, because every usable sound block is played.
- **Left open, recorded.**
  - **A sound thread of its own for the player (Stage 5).** After a seek, all of a project's warm-up is mixed in one call on the player's thread, which also draws the picture. With the costliest valid blocks declaring 4-second warm-ups on eight tracks, that held picture and transport for 41 seconds (the window stays responsive).
  - **Cost.** Eight tracks with gain, pan and eq3 take about 167 ms of one i9-13900 core per second of sound (about 185 ms when the clips are cut every two seconds), against 5.8 ms before Stage 4, when eq3 was not played. On a mid-range laptop this is estimated, not measured, at 35 to 50% of one core. Most of it is the interpreter with portable dispatch (19). Tracks are not mixed in parallel.
  - **Fuel and time.** Per unit of fuel, arithmetic on denormal numbers is about 9 times slower than plain instructions, and calls of a function with 1,024 locals about 5 times. Pricing floating-point operators higher (`Config::operator_cost`) or lowering the locals limit to 256 would need the built-ins' fuel measured again: owner's decision.
  - **Smaller items:**
    - After a hostile block faults it is off from the next call, so preview and export can differ for it by up to one call.
    - Near the session budget, which instances may grow depends on which are alive, so preview and export can differ there.
    - A block refused growth by the session budget may fail and then stays off for the session.
    - wasmi's interpreter stack is outside the budget (bounded per installed module).
    - Hand-edited transitions with a negative duration were not tested.
    - The EQ's corner-frequency slider covers only part of its range (Stage 5 inspector).
    - Latency and tail are recorded and not used (version 1).
  - **Hardware:** proven only on the RTX A4000 computer (Windows); golden sounds depend on Windows maths.

## 24. A block in one .wgsl file (the owner's decision, 2026-11-05)
The owner chose, of the two forms README-STAGES offered: a picture block may also be installed from one `.wgsl` file whose manifest sits in a comment at its top. A pack file holding several blocks is not wanted now. Binding from now on; it adds to sections 2, 3, 12 and 15.
- **The form.** The file is UTF-8 WGSL text (a byte order mark is allowed and ignored). Its first text, after any blank lines, is a block comment that opens with `/*` followed on the same line by the marker word `chadfx-block`. Everything after the marker up to that comment's closing `*/` is the manifest, as JSON: the same object as `block.json` (section 3), with the same rules and the same refusal of unknown fields. Example:

  ```wgsl
  /* chadfx-block
  {
    "manifest": 1, "id": "acme.warm", "version": "1.0.0", "format": 1,
    "name": "Warm", "description": "Makes the picture a little warmer.",
    "kind": "video.filter", "author": "Acme",
    "params": [ { "id": "amount", "type": "float", "default": 0.5, "min": 0, "max": 1, "label": "Amount" } ],
    "video": { "stages": ["pixel"] }
  }
  */
  fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
      return vec4<f32>(c.r + 0.05 * p.amount * c.a, c.g, c.b - 0.05 * p.amount * c.a, c.a);
  }
  ```
- **What differs from a package.**
  - `video.source` is left out: the shader is this file. A header that names a source, or any other file (a module, a README, a licence file, an icon), is refused, because a lone file has nothing beside it.
  - Only picture kinds (`video.*`): a sound block needs a WebAssembly module, which a text file cannot hold. A sound kind is refused with that reason.
  - The manifest's JSON may not contain `/*` or `*/`: in WGSL the first would open a nested comment and the second ends the header. A text that needs them is written as a package instead; the refusal says so.
  - A file that does not start with the header is refused in plain words: it says that a block's `.wgsl` file starts with a `/* chadfx-block` comment holding its description, and that a shader without one is not a block.
- **Limits.** The file is read through `take(limit + 1)` with a package file's limit (section 2: at most 4 MiB; refused as `lone_file_too_large`), and the header's JSON is held to the manifest's limits. Because the file is the block's shader, the shader's own limit (section 7: at most 64 KiB of source) covers the whole file, header included, so in practice a lone file is at most 64 KiB (corrected in section 25).
- **Checking and keeping.** Installing checks the file exactly as a package with that manifest and that shader would be checked (sections 3, 7 and 19, the shader checker included); nothing of it runs before that. It is then kept as the canonical package archive `<id>.chadfx-block` that `vep blocks pack` writes for a folder holding `block.json` (the header's manifest with `video.source` set to `block.wgsl`, written the same way every time) and `block.wgsl` (the file's bytes unchanged, header included). So everything after installing (identity by BLAKE3 of the archive, quarantine, disable, removal, replacing, the catalogue) is that of any user package, and installing the same file twice gives the same package.
- **Where it can be installed from.** `vep blocks install FILE.wgsl`, the block manager's "Install from a file" (its file chooser offers `.wgsl` beside `.chadfx-block`), a `.wgsl` file dropped on the window, and ChadFX started with the file's path (the person chose "Open with" ChadFX), which asks the same question as for a `.chadfx-block` file. `vep blocks validate FILE.wgsl` checks one, and `vep blocks pack FILE.wgsl` writes the `.chadfx-block` file a person can share.
- **No file association for `.wgsl`.** ChadFX registers only `.chadfx-block`. People edit `.wgsl` files in code editors, and installing ChadFX must not take those files over; "Open with", where the person picks ChadFX, still works.
- **Not in this form:** an icon, a README or a licence file, and several blocks in one file. Use a package for those.

## 25. Rules settled by Stage 5 (2026-11-06; details in stages/s5-*.md and s5f-*.md)
The builders, the reviewers, the two acceptance checks and the main session settled these. Binding from now on; where they differ from sections 10, 12, 13, 15 and 24, this section wins.
- **One catalogue in the app.** `blocks::CatalogSlot` (`crates/blocks/src/slot.rs`) is the one home of the replaceable catalogue snapshot; the renderer, the player's mixer and the export read it. The app keeps one `Registry`. Every change (install, turn off or on, enable again, remove, a package another program put in the folder) publishes one new snapshot, with one conversion and check per change, and the window hears `BlocksChanged`. A package is checked without holding the registry's lock (`registry::check_for_install`, then `Registry::install_checked`), and two installs of one block at once end with exactly one of them.
- **When state.json cannot be written,** memory, the snapshot and the disk stay in agreement. Turning a block off or on and enabling it again change nothing and say so. A quarantine is published and then reported. A removal is done (the package is gone), and the catalogue problem `state_unwritable` says which flags state.json still holds.
- **Converted settings are what everyone sees (amends 10).** The views (`EffectView`, `TransitionView`), the AI's timeline text and `vep effect list` show a node's values and keyframes converted to the installed block's format by `blocks::migrate_params`, the same code the renderer and the mixer use, with a `converted` note naming the format the project stores. The project itself is converted only when one of the node's settings changes (`migrate_on_edit`): turning an effect off or on, or moving it, keeps the stored format. Every change the window makes to a transition sets it again, so that converts it.
- **A node whose settings cannot be converted** is `cannot_convert` and treated as missing (10, 13), and so is a transition of either style: a fade whose settings cannot be converted is a cut in the renderer, in the mixer and in every list (`render::transition_draws`, `audio::transition_plays`).
- **A block in the wrong place** (a sound effect on a picture clip, a picture source on a sound track) has the status `wrong_place` with a plain reason, from the one rule `blocks::edges::wrong_place`. The edges' refusal, `misplaced`, `status_at`, the block manager, the timeline view, the AI text, `vep effect list`, the export's list and the report on opening a project all use it. A node is judged in this order: the catalogue's status, then the place, then whether its settings convert.
- **The default transition** is the block whose manifest says `"default_for": "transition"`; only a transition may say it (rule `default_for_kind`). `Catalog::default_transition(media)` picks among usable blocks, built-in before installed, then by id; several claims are the catalogue problem `default_transition_duplicate`. `CatalogDescription.default_transitions` gives `{video, audio}`. The built-in cross dissolve and crossfade claim it, and the window's dissolve gesture takes it from there: no window code names a block id (`blockNames.test.ts`).
- **The catalogue says how a picture block runs:** `BlockDescription.stages` (the manifest's `video.stages`) and `passes` (pass names in run order).
- **Icons are decoded in the backend only (amends 12).** `blocks::icon::decode_icon` takes at most 256 KiB and 512 by 512 pixels (checked before decoding), holds the PNG decoder's memory to 2 MiB, verifies checksums, ignores text and colour-profile chunks and catches a panic; an animated PNG gives its first picture. `blocks_icon` answers width as a little-endian u32, height as a little-endian u32, then RGBA rows top to bottom, alpha not premultiplied. The window checks the length against the stated size and draws it with `putImageData`; it decodes no image from a package.
- **Where a node is, by name.** `BlockUse` and an export's `MissingBlock` carry `clip_name` (the clip's label, else its media's name, else its source block's name) and `track_name`, by one rule. The window shows them isolated (19, 22).
- **Replacing a block the open project uses warns first,** before the person confirms: how many effects use it and, when the new package has a lower settings format than some, that those need a newer version and are not drawn or played until one is installed (refusal detail `new_format`, `nodes`, `needs_newer`). `vep blocks install --project FILE` says the same of the project it was given ("The project has ...").
- **A refusal names what was offered:** "This package cannot be installed ..." for a package file or folder, "This .wgsl file cannot be installed ..." for a block in one file (`blocks::manage::refusal_from`).
- **The built-in folder.** A packaged app ships `blocks/builtin` beside the program and never falls back to the repository's. `vep render-frame`, `vep export` and `vep export-still` say when the folder is missing before they start.
- **The player's sound thread.** Sound is mixed on a thread of its own (`vep-player-sound`), never on the thread that draws the picture. It keeps up to `SOUND_LEAD` (300 ms) queued ahead of the device, mixing in bursts when the queue falls below half; so an edit or a block turned off is heard at most that much later than it is seen. After play or a seek, the playhead waits up to `SOUND_LEASH` (250 ms) for the first sound; a project whose warm-up takes longer starts its picture, and its sound joins in place when it comes. The preview plays the export's samples, converted when the device runs at another rate (`playback::Resampler`).
  - A lost device is noticed at once through the output's error callback, with `DEVICE_STALL` (500 ms) of silence as a backstop. The thread tries to open the device again every `REOPEN_EVERY` (1 s), and the sound comes back from the playhead. With no device at all, playback goes on silent by the wall clock.
  - While sound plays, the thread checks every `DEFAULT_CHECK` (2 s) whether the system's default device changed, and follows it.
  - An edit that makes the sequence longer after its end was already queued moves nothing (`Resampler::tail`).
- **Focus after a keyboard action (amends 15).** When a control pressed with the keyboard disappears, moves, or goes out of use while its action runs, the focus goes back to it when it can be used again, otherwise to the next sensible control (`lib/focus.ts`, `keepKeyboardPlace`). A mouse click leaves the focus where the browser puts it, so the panels' shortcuts keep working.
- **The window's texts** all live in `ui/src/lib/strings.ts` (`lib/windowText.test.ts` reads every source file and allows none elsewhere). A number field keeps the value typed, with no snapping to a step it was not given.
- **Correction to 24, limits.** The shader's 64 KiB source limit covers a lone `.wgsl` file whole, header included.
- **Where a block's file may come from.** A block's file or folder is read only from a drive or a network share. Never through the Windows device namespace (`\\.\`, `\\?\GLOBALROOT`, volume paths) or a server's `pipe` share, because reading those can wait without end (`opened::on_a_drive_or_share`). This holds for the block manager, a drop, "Open with" and a hand-off.
- **Opening a block file with ChadFX.** The installer registers `.chadfx-block` only, never `.wgsl` (24): named "ChadFX Block", with the program's icon, and an open command that quotes both the program's path and the file's. ChadFX started with block files asks about each in turn: "Install <name> by <maker>?" with the id beside the name, the version and the kind, every text isolated; refusals use the block manager's words. Installing from the question installs only the package that was shown (its identity is checked again). The question waits while another question of the window is open.
- **The hand-off to a running ChadFX.** The running app holds an exclusive lock on `<config folder>/opened/app.lock`. A start that names block files and finds the lock taken writes a request there (JSON `{"format": 1, "sent_ms", "paths"}`, at most 256 KiB and 64 paths, renamed into place) and ends within a second without a window. The running app looks every 200 ms and trusts nothing in a request: each path is checked as a start's argument is, and only ever leads to the question. It drops requests older than 30 s or from the future, and hands each request over at most once; one it cannot delete is only deleted on later looks.
- **The installer.** Only the NSIS installer is built (`ui/src-tauri/tauri.windows.conf.json`: `"targets": ["nsis"]`; the owner allowed its tools on 2026-11-05, not WiX's), for the current user, with `blocks/builtin` beside the program. `ui/src-tauri/windows/installer-hooks.nsh` quotes the program's path in the open command after installing, and removes the `.chadfx-block` key Tauri's uninstaller leaves empty. `productName` is ChadFX; the window's title still says Vep (the owner's choice).
- **Left open, recorded.**
  - **Editing a whole track's or sequence's effects:** the inspector shows such nodes only when they cannot be used; usable ones have no editing section yet (Stage 6).
  - **A mistyped shape value** makes the renderer skip a node whole (22), while `judge` still calls it available, so the lists do not name it.
  - **Not tried for real:**
    - a sound device unplugged or switched: the recovery rests on test doubles and cpal's own source;
    - the Mac path (`RunEvent::Opened`, Finder's file type, the Mac bundle), which was never built;
    - the file association on the real desktop, because Claude's app container redirects registry writes. The owner checks it by running the installer outside Claude.
  - **Smaller items:**
    - a notice can cover the block manager's list;
    - a very long description in the block's question cannot be scrolled with the keyboard;
    - no end-to-end test presses Move down with the keyboard;
    - the focus rule acts only within a time window (up to 10 minutes while an action runs).
  - **Carried from 23:** the composer's uniform cap against the card's own limit (Stage 6); sound cost and fuel pricing (the owner's decision). The AI reading package text as untrusted belongs to a later phase.

## 26. Rules settled by Stage 6 (2026-11-06; details in stages/s6-*.md and s6f-*.md)
The builders, the three reviewers, the acceptance check and the main session's fix round settled these. Binding from now on; where they differ from earlier sections, this section wins.
- **The uniform cap is the card's (amends 19 and 22).** A composed draw holds at most the smaller of 64 KiB and the device's own `max_uniform_buffer_binding_size` of uniforms (`blocks::wgsl::compose::uniform_limit`, `render::programs::uniform_limit`). A recipe made before any device is opened uses 64 KiB. On a smaller card, the blocks that do not fit are left out ("... at most N fit in one uniform buffer on this graphics card"), and the frame is drawn without them instead of failing.
- **A mistyped shape value (replaces 25's open item).** One rule, `blocks::edges::broken_shape`, judges a node whose stored value or any keyframe holds something else where its block declares a shape. It covers every picture kind (filter, transition, blend, source), not only filters, and does not depend on time. The renderer skips such a node whole, its opacity role included. `judge` calls it `cannot_convert` with a plain reason, so the block manager, the timeline view, the AI's text, `vep effect list` and the export's list all name it. No new status.
- **The command line names what it leaves out.** `vep render-frame` prints a `left out: ...` line for each node it does not draw (missing, turned off, quarantined, mistyped, wrong place, or too much for one draw). `vep export-still` and `vep export` also name nodes their draws could not hold.
- **A node left out of its draw is a runtime problem (amends 22).** When a stack is too much for one composed draw (more than 64 blocks with code, more uniforms than the card holds, or too much code), the renderer reports each node it leaves out as the problem kind `left_out_of_draw` (`BlockProblemKind::LeftOutOfDraw`, `Renderer::report_left_out`, from the same `render::left_out_within` list the block manager and `vep` use).
  - It is counted per frame and carries the composer's reason, so it says which limit.
  - It reaches `ExportReport.block_problems`, `StillReport`, the preview's problem list and the inspector's notes on the node, and the window says it in one sentence (`strings.ts`).
  - It is never a quarantine and is not a sound kind: it is not the block's fault.
  - `vep` names such nodes in its `left out` lines and lists them in `--json` reports, without saying them twice.
- **Any transition length the engine accepts is drawn (amends 9.3).** The engine accepts any positive exact length no longer than the clip, including one that lies on no frame or sample grid. The renderer measures an outgoing fade so that no accepted length can stop a frame (`render::graph::outgoing_progress`): when the exact arithmetic fits, it is unchanged, so no picture moved; otherwise the instant is measured back from the clip's end.
- **At most 1,000 packages per folder (amends 12).** The registry reads at most `MAX_PACKAGES_PER_FOLDER` (1,000) packages per folder, in name order. Past that, the catalogue problem `user_folder_too_many` or `builtin_folder_too_many` says how many the folder holds and that only the first 1,000 are read. Installing a new block into a full user folder is refused with that code, and nothing is written; replacing an installed block still works.
- **Every library's words are outside text (amends 22).** Error text from the JSON reader, wasmparser and naga that a refusal quotes is escaped and cut to one plain line of at most 600 characters (`blocks::ids::one_plain_line`), as driver and system errors already were. This includes a field or export name a package chose.
- **Plainer refusals.**
  - A stage written as an entry point (`@fragment fn pixel`, `@vertex`, `@compute`) gets `wgsl_entry_point` alone, without a false `wgsl_stage_missing` or naga's entry-point text.
  - A lone `.wgsl` file of a sound kind gets `lone_sound_kind`, whatever its audio section holds.
  - `wasm_memory_too_large` says that the 4 MiB covers the stack and all static data together, and that a module may grow its memory in `vep_prepare` up to 16 MiB.
  - The block manager's list under a refusal is headed "What is wrong with it:", true of packages, `.wgsl` files and installed blocks alike.
- **`vep` forms (amends 15 and 25).**
  - `vep effect add/set --set` reads a colour as `[r,g,b,a]` or in block.json's `{"r":..,"g":..,"b":..,"a":..}` form, and a curve only as `[[x,y],...]`.
  - `vep blocks info` prints colour defaults in the object form.
  - `vep blocks pack .` works.
  - `vep blocks enable-again ID` lifts only a quarantine whose hash is none or the installed package's. A remembered quarantine of another package gives the "not turned off by the editor" answer.
- **Where a block's file may come from, for `vep` too (amends 25).** The one rule is `blocks::package::on_a_drive_or_share`, and the app's `opened::on_a_drive_or_share` is that same function. `vep blocks install`, `validate` and `pack` first make a typed path whole as Windows opens it (`std::path::absolute`, which reads nothing), then apply the rule. So a device name such as `NUL`, `C:\x\NUL` or `CONIN$` is refused as well, at once, with exit code 1 and one sentence. `--project` files and `pack`'s output path are not checked.
- **A description keeps its line breaks.** Where the window shows a block's description, or a setting's description, as running text (the block manager's details, the question when a block file is opened), a line break is shown as a line break (`visibleLines`). Every other control and direction-changing character, tab and carriage return included, stays a visible code. Names, list rows and tooltips are one line and show `\n` as a code (`visibleText`).
- **Sound blocks are contained at every layer.** The checker, the mixer, the app's service and player, `vep` and the running window each have tests in which blocks of every sound kind trap, run out of fuel, write outside their memory, fail in the middle of a crossfade, lie about their warm-up and latency, or cost as much as the rules allow. Each test checks the fallback: a filter gives the clip's own sound, a source gives silence, a transition gives the plain crossfade. The table is `stages/s6-sound-inventory.md`. The fuel price and the limit of 1,024 locals are unchanged, pending the owner's decision (measured in `stages/s6-sound.md`).
- **Editing a whole track's and the sequence's effects (replaces 25's open item).** The inspector edits track and sequence nodes as it edits a clip's: switch, reset, settings, keyframes, move, remove and undo.
  - A selected clip shows its own effects, then its track's list.
  - Clicking a track's name selects the track, so a track with no clips can get effects.
  - With nothing selected, the sequence's list sits under its settings.
  - Which kinds go in which list is `blocks::Place::allows`. The window repeats it in `catalog.ts` `fitsList`, and a unit test reads `crates/blocks/src/node.rs` and fails if the two drift.
  - The sequence's add menu offers only picture filters with a pixel stage or pass stages, because only those run at sequence level (7.7, 22). There, effects with pass stages run first, and the order note says so (`order.ts` `appliedBeforeOnSequence`).
  - An add menu that offers nothing says why (blocks turned off or broken, or none installed) and offers the block manager.
- **Where notices go (amends 25).** Notices never lie over a dialog's content or over the panels above the timeline:
  - while a dialog is open, in the dialog's own band (every dialog has one);
  - otherwise, in the timeline panel's bottom right corner, below its header and never taller than the panel, scrolling with the newest in view;
  - with no project open, in the window's corner, at most 40% of its height.
- **Focus after a menu (amends 25's focus rule).** A menu closed with the keyboard gives the focus back to the control that opened it, before the chosen item runs, so a dialog the item opens still takes the focus. A mouse click leaves the focus on the page.
- **The template folder.** `blocks/template` holds five working examples, each with a read-me: a picture filter (`example.warm`), the same filter as one `.wgsl` file (`example.warm-one-file`, its own id so both can be installed side by side), a transition of the mix style (`example.iris`), a source (`example.gradient`) and a sound filter in Rust (`example.width`).
  - The sound template is one crate, with `#![deny(unsafe_code)]`; only its six exports carry `#[allow(unsafe_code)]`, as `#[unsafe(no_mangle)]` needs.
  - It sets no panic strategy (wasm32v1-none's default is already abort) and pins its toolchain to 1.99.0.
  - Tests rebuild its module and the built-in sound blocks and compare the bytes.
- **The guide for outsiders is `docs/BLOCKS.md`.** `crates/blocks/tests/docs.rs` reads the rule codes, statuses, problem kinds, manifest choices and `vep blocks` commands from the code and the schema, and fails when the guide misses one or names one that does not exist. Section 10 of the guide draws the line between the core and blocks (brief rule 6).
- **The end-to-end harness builds offline,** with `cargo build --offline`, and `ui/src/lib/offline.test.ts` fails on any cargo call in the harness without `--offline`.
- **Left open, recorded.**
  - **The owner's decisions:**
    - the sound fuel price and the locals limit;
    - the EQ's defaults;
    - whether `vep` ships with the installer;
    - whether the uninstaller also deletes `HKEY_CURRENT_USER\Software\vep\ChadFX` (Tauri's remembered install folder, kept unless "delete the application data" is ticked);
    - whether notices move to a list in the status bar;
    - forcing every colour to a finite value;
    - turning a faulting sound block off for good instead of for the session;
    - the window's title (still "Vep");
    - adding generator clips and keyframes from the window.
  - **Not tried for real:**
    - Mac, Linux and ARM;
    - AMD, Intel and Apple graphics;
    - a mid-range laptop (estimated at 35 to 50% of one core for 8 EQ tracks);
    - a real card with a uniform limit under 64 KiB (simulated with a 16 KiB device);
    - the file association on the real desktop;
    - a sound device unplugged or switched.
  - **Smaller items:**
    - Delete with only a whole track selected does nothing and says nothing.
    - Tooltips that hold information are mouse-only.
    - A removed block is named by its id in the export dialog, because the catalogue no longer knows its name.
    - No end-to-end test installs, replaces or removes a sound block while it plays (the player's tests do).
    - The registry re-checks every installed package after each install.
    - A runtime problem (`BlockProblem`) names its block and node but not its clip or track, so two left-out nodes of one block read as two identical sentences in the export summary. The inspector places each note on its node.
    - A left-out node's reason is the composer's first refusal (66 blocks read "this draw has 65 blocks with code"): true, but not the stack's whole size.
