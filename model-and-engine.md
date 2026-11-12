# How effects live in ChadFX's project model and editing engine today

All paths are relative to `crates/` unless they start with `ui/` or `docs/`. Nothing in the project was changed. The JSON and behaviour samples come from a throwaway program in the scratchpad, `docs/blocks-rewrite\probe\src\main.rs`, which uses the real crates and has its own build folder.

---

## 1. The current data model

### 1.1 `EffectKind`: a closed enum
`core-model/src/effect.rs:9-30`. The comment at lines 9-10 says: "The closed set of primitives the renderer understands… Adding a variant here is a renderer change." It is written to JSON in snake_case (`effect.rs:12`). A separate dotted id comes from `id()` (`effect.rs:62-81`) and is read back by `from_id()` (`effect.rs:83-85`). The dotted id is not what the file stores: the file stores the snake_case name.

| Variant | Stored in JSON as | `id()` | `domain()` (`effect.rs:87-97`) |
|---|---|---|---|
| Transform | `"transform"` | `core.transform` | Video |
| Crop | `"crop"` | `core.crop` | Video |
| Composite | `"composite"` | `core.composite` | Video |
| Blur | `"blur"` | `core.blur` | Video |
| ColorMatrix | `"color_matrix"` | `core.color_matrix` | Video |
| Curves | `"curves"` | `core.curves` | Video |
| Lut | `"lut"` | `core.lut` | Video |
| TimeRemap | `"time_remap"` | `core.time_remap` | **Any** |
| Mask | `"mask"` | `core.mask` | Video |
| Text | `"text"` | `core.text` | Video |
| Shader | `"shader"` | `core.shader` | Video |
| AudioGain | `"audio_gain"` | `core.audio_gain` | Audio |
| AudioEq | `"audio_eq"` | `core.audio_eq` | Audio |
| AudioCompressor | `"audio_compressor"` | `core.audio_compressor` | Audio |
| AudioPan | `"audio_pan"` | `core.audio_pan` | Audio |
| AudioTimeStretch | `"audio_time_stretch"` | `core.audio_time_stretch` | Audio |

- `EffectKind::ALL` lists the variants in the order above (`effect.rs:42-59`).
- `EffectDomain { Video, Audio, Any }` is at `effect.rs:32-39`. `Any` is documented as "time remapping" (line 37) and is used only by TimeRemap. Every variant not named in `domain()` falls into Video through the catch-all `_` (line 95).

### 1.2 `ParamKind` and `ParamValue`
- `ParamKind` (`effect.rs:209-224`) has twelve values: `float, int, bool, vec2, vec3, color, text, enum, curve, shape, dense_curve, artifact`.
- `ParamValue` (`effect.rs:229-244`) is stored as `{"type": …, "value": …}` (line 230). Every shape below was printed by the probe:

| Variant | JSON |
|---|---|
| `Float(f64)` | `{"type":"float","value":1.5}` |
| `Int(i64)` | `{"type":"int","value":3}` (no catalog parameter uses Int) |
| `Bool` | `{"type":"bool","value":true}` |
| `Vec2([f64;2])` | `{"type":"vec2","value":[1.0,2.0]}` |
| `Vec3([f64;3])` | `{"type":"vec3","value":[1.0,2.0,3.0]}` |
| `Color(Rgba)` | `{"type":"color","value":{"r":1.0,"g":1.0,"b":1.0,"a":1.0}}`. `Rgba` holds **f32** values (`effect.rs:189-201`). |
| `Text(String)` | `{"type":"text","value":"hi"}` |
| `Enum(String)` | `{"type":"enum","value":"box"}` |
| `Curve(Vec<CurvePoint>)` | `{"type":"curve","value":[{"x":0.0,"y":0.0},{"x":1.0,"y":1.0}]}` (`CurvePoint` is at `effect.rs:203-207`) |
| `Shape(MaskShape)` | `{"type":"shape","value":{"type":"rect","x":0.0,"y":0.0,"width":1.0,"height":1.0,"corner_radius":0.0}}` |
| `DenseCurve(BlobKey)` | `{"type":"dense_curve","value":"b3:abcd"}`: per-frame data kept in the project blob table |
| `Artifact(ArtifactKey)` | `{"type":"artifact","value":"a:ef01"}`: points into the machine cache |

Helpers on `ParamValue`:
- `kind()` and `as_f64()` (Float or Int), `as_bool()`, `as_str()` (Text or Enum): `effect.rs:246-284`.
- `is_interpolable()` is true for Float, Int, Vec2, Vec3 and Color: `effect.rs:287-292`.
- `is_finite()`: `effect.rs:295-305`.

### 1.3 `Param`, `Keyframe`, `Interp`
- **`Param { value, keyframes: im::Vector<Keyframe> }`** (`effect.rs:359-365`). The keyframes are serde-defaulted to empty but always written out (`"keyframes":[]`). Once keyframes exist, `value` is ignored for evaluation but kept as the "reset" value (comment at lines 357-358).
- **`Keyframe { time: RationalTime, value: ParamValue, interp: Interp }`** (`effect.rs:321-328`). `time` is **clip-local, where 0 is the clip's start on the timeline** (lines 323-324). `interp` defaults to Linear (line 326).
- **`Interp`** (`effect.rs:312-319`) is stored as `{"type":"hold"}`, `{"type":"linear"}` or `{"type":"bezier","out_tangent":[x,y],"in_tangent":[x,y]}`. The default is Linear. The tangents are the two inner control points of a CSS-style cubic-bezier in normalised segment space (lines 308-311).
- **Evaluation** is `Param::value_at` (`effect.rs:384-417`):
  - Before the first keyframe it returns the first value; after the last it returns the last.
  - Hold keeps the left value; Linear and Bezier interpolate with `lerp_value` (`effect.rs:431-446`).
  - Kinds that cannot be interpolated hold the left value (line 444).
  - `bezier_ease` is at `effect.rs:450-484`; it clamps the x control values to 0..1.

### 1.4 `MaskShape` and `MaskRef`
- **`MaskShape`** (`effect.rs:116-123`) is internally tagged by `"type"`:
  - `rect {x, y, width, height, corner_radius}`, where `corner_radius` defaults to 0;
  - `ellipse {cx, cy, rx, ry}`;
  - `path {points: [[x,y]…], closed}`, where `closed` defaults to true.
  - Coordinates are normalised 0..1 (line 116).
  - `is_finite` and `is_well_formed` (sizes and radii at least 0, a path needs at least 2 points) are at `effect.rs:129-150`.
- **`MaskRef`** (`effect.rs:152-171`) is internally tagged by `"type"`:
  - `shape {shape, invert, feather}`, where `invert` and `feather` default;
  - `artifact {key: ArtifactKey, invert, feather}`.
  - Validity rules (`effect.rs:173-187`): feather is finite and at least 0, and an artifact key must not be empty.

### 1.5 `BlendMode`
`effect.rs:100-114`: `normal` (the default), `add, multiply, screen, overlay, soft_light, hard_light, darken, lighten, difference`, written in snake_case.

### 1.6 `EffectNode`
`effect.rs:499-515`. Fields in declaration order, which is also their order in the file:

| Field | Type | serde |
|---|---|---|
| `id` | `EffectId` (a ULID string) | required |
| `kind` | `EffectKind` | required; an unknown value is an error |
| `enabled` | bool | `default = true` (line 504) |
| `params` | `im::OrdMap<ParamId, Param>`, written as a JSON object with keys sorted bytewise | `default` |
| `mask` | `Option<MaskRef>` | `default`; written as `null` when absent |
| `blend` | `Option<BlendMode>` | `default`; `null` when absent |
| `provenance` | `Option<Provenance>` | `default`; `null` when absent |

There is no `metadata` field. `SetMetadata` on an effect is refused with "does not carry metadata" (`core-commands/src/ops/misc.rs:64-69`). The builder and accessor methods are at `effect.rs:517-559`. `set_param` keeps existing keyframes (lines 550-558).

### 1.7 `EffectTarget`: where the effect lists live
- `EffectTarget` (`effect.rs:561-568`) is stored as `{"kind":"clip"|"track"|"sequence","id":"<ulid>"}`.
- The three lists:
  - `Clip.effects` (`timeline.rs:190-192`);
  - `Track.effects` (`timeline.rs:270-272`);
  - `Sequence.effects` (`timeline.rs:378-380`).
- All three are `im::Vector<EffectNode>`, and the vector order is the stack order.
- Lookup is `ProjectState::locate_effect` (`project.rs:142-177`), which searches sequence effects, then for each track its effects followed by its clips' effects. The result type `EffectLocation` is at `project.rs:75-82`. `all_effects()` is at `project.rs:299-317`.
- Effect ids must be unique across every kind of entity (`project.rs:224-244`; enforced at `core-commands/src/ops/helpers.rs:83-97`).
- Which kind of media each list is checked against:
  - clip and track lists are checked against the track kind (`validate.rs:248` and `:333`; `ops/effects.rs:25-39`);
  - **sequence lists are checked against nothing** (`validate.rs:236`; `ops/effects.rs:21-24` passes `None`). The probe confirmed that an AudioGain node carrying a blend and a mask is accepted on a sequence.

### 1.8 `Transition` and `TransitionKind`
- **`TransitionKind`** (`timeline.rs:130-140`): `cross_dissolve, dip_to_black, dip_to_white, wipe, slide, zoom, Custom(String)`. It is externally tagged, so `Custom` is stored as `{"custom":"iris"}`, which makes it an existing open-ended escape hatch.
- **`Transition { kind, duration: RationalTime, params: BTreeMap<ParamId, ParamValue> }`** (`timeline.rs:142-150`). The params are plain values: no `Param` and no keyframes. There is no id, no `enabled` flag and no spec or registry for transitions.
- A clip holds `transition_in` and `transition_out` (`timeline.rs:193-196`).
- Validation checks only that the duration is positive and no longer than the clip, and that every parameter is finite (`ops/helpers.rs:193-204`; `validate.rs:316-323`).
- `kind` is never examined anywhere in the core crates.

### 1.9 `GeneratorKind` and `ClipSource::Generator`
- `GeneratorKind` (`timeline.rs:89-99`): `solid_color, color_bars, text, adjustment` ("Applies its effect stack to everything beneath it", line 95), `silence, tone`.
- `track_kind()` (`timeline.rs:101-108`): Silence and Tone go on audio tracks, everything else on video tracks.
- `ClipSource` is internally tagged `"type"` (`timeline.rs:110-115`) and stored as `{"type":"generator","kind":"solid_color"}`.

### 1.10 `Provenance`
- `effect.rs:486-497`: `{ skill_id: SkillId (string), skill_version: String, input_hash: String, plan_step: Option<StepId> }`.
- It exists on `EffectNode` (`effect.rs:513-514`) and on `Clip` (`timeline.rs:205-206`).
- Only a clip's provenance can be changed later (`SetClipProvenance`, `core-commands/src/command.rs:126`). An effect's provenance is set only when the node is created.

### 1.11 A real serialised clip: two effects, a mask, keyframes and two transitions
This came from `serde_json::to_string`, the same call `to_json` uses, and the clip was accepted by `apply_checked`. The ids are short numeric ULIDs (`Ulid::from_u128(n)`) so they are easy to read. This is the byte-exact compact form the file stores:

```
{"id":"00000000000000000000000005","source":{"type":"asset","asset_id":"00000000000000000000000002","stream_index":0},"timeline_range":{"start":{"num":2,"den":1},"duration":{"num":5,"den":1}},"source_in":{"num":10,"den":1},"effects":[{"id":"00000000000000000000000006","kind":"transform","enabled":true,"params":{"opacity":{"value":{"type":"float","value":0.8},"keyframes":[]},"position":{"value":{"type":"vec2","value":[0.0,0.0]},"keyframes":[{"time":{"num":0,"den":1},"value":{"type":"vec2","value":[0.0,0.0]},"interp":{"type":"linear"}},{"time":{"num":1,"den":1},"value":{"type":"vec2","value":[100.0,0.0]},"interp":{"type":"bezier","out_tangent":[0.42,0.0],"in_tangent":[0.58,1.0]}},{"time":{"num":5,"den":2},"value":{"type":"vec2","value":[100.0,50.0]},"interp":{"type":"hold"}}]}},"mask":null,"blend":null,"provenance":null},{"id":"00000000000000000000000007","kind":"blur","enabled":true,"params":{"kind":{"value":{"type":"enum","value":"gaussian"},"keyframes":[]},"radius":{"value":{"type":"float","value":12.0},"keyframes":[]}},"mask":{"type":"shape","shape":{"type":"ellipse","cx":0.5,"cy":0.5,"rx":0.3,"ry":0.2},"invert":true,"feather":8.0},"blend":"screen","provenance":{"skill_id":"video.softFocus","skill_version":"1.0.0","input_hash":"ab12","plan_step":null}}],"transition_in":{"kind":"cross_dissolve","duration":{"num":1,"den":1},"params":{"softness":{"type":"float","value":0.5}}},"transition_out":{"kind":{"custom":"iris"},"duration":{"num":1,"den":2},"params":{}},"link_group":null,"enabled":true,"label":null,"color":null,"provenance":null,"metadata":{}}
```

Things to notice:
- Parameters may be sparse: the Blur node has no `angle`.
- Optional fields are always written as `null`.
- `params` keys are sorted (`opacity` before `position`).
- Struct fields appear in declaration order.

For reference, a TimeRemap node as `time_map::constant_speed_node` builds it:
`{"id":"…08","kind":"time_remap","enabled":true,"params":{"reverse":{"value":{"type":"bool","value":true},"keyframes":[]},"speed":{"value":{"type":"float","value":0.5},"keyframes":[]}},"mask":null,"blend":null,"provenance":null}`. There is no `frame_blend` parameter in it.

---

## 2. The time remap today (it is an `EffectKind`)

### 2.1 Where it is stored and how it is chosen
- It is a node in `Clip.effects`. The doc comment at `timeline.rs:180-182` says: "Speed changes are a `TimeRemap` effect, never a separate field, so there is one mechanism."
- **The active remap is the first enabled TimeRemap node** in `clip.effects` (`core-model/src/time_map.rs:59-61`). A disabled remap means normal speed (`time_map.rs` test `disabled_remap_is_ignored`).
- Placement rules: one per clip, and clips only. Disabled remaps count towards the limit:
  - `validate.rs:153,166-171` reports `TimeRemapMisplaced` (`validate.rs:50,119-121`);
  - `ops/effects.rs:63-76` (`check_time_remap_placement`) runs on AddEffect (line 83);
  - `ops/helpers.rs:224,231-237` runs on InsertClip;
  - AddTrack and CreateSequence catch it only through the full `validate` (`ops/tracks.rs:22-27`; `ops/sequences.rs:49-52`). The probe confirmed AddTrack is refused for a remap on a track.
- `EffectDomain::Any` (`effect.rs:37-38,89`) exists only for the remap.

### 2.2 How its parameters are read (the meaning a clip-level field must keep)

| Rule | Location |
|---|---|
| Parameter names `SPEED_PARAM = "speed"` and `REVERSE_PARAM = "reverse"` | `time_map.rs:51-52` |
| No speed parameter means 1x | `time_map.rs:73,103,198` |
| A speed that is not a number means 1x; a negative speed means 0 | `factor_of`, `time_map.rs:92-95` |
| A constant speed becomes a fraction via `approx_seconds(s, MAX_APPROX_DEN = 1_000_000)` | `time_map.rs:56,94` |
| Exactly 1x takes an identity fast path | `time_map.rs:201` |
| A missing or non-bool reverse means false | `time_map.rs:68,80-85` |
| Keyframed speed is a ramp: integrated in floating point, rounded to microseconds | `time_map.rs:203-204,311-357` (bezier stretches in 64 straight pieces, `EASE_CELLS`, lines 275 and 334-353; negative values clamped by `speed_of`, lines 280-282) |
| Derived queries `is_identity`, `is_reversed`, `constant_speed`, `source_offset`, `source_span`, `source_end`, `source_time_at` (reversed rule at lines 236-247), `source_range` | `time_map.rs:64-78, 80-85, 101-107, 196-205, 208-210, 216-218, 234-248, 257-271` |
| Builder `constant_speed_node` | `time_map.rs:360-364` |
| `frame_blend` is defined only in the registry and **read nowhere** | `effects-registry/src/lib.rs:189` |
| The registry range for speed, 0.001..1000, is applied only through registry validation | `effects-registry/src/lib.rs:187` |

### 2.3 Every place that reads or enforces it

**core-model**
- `validate.rs:300`: `ClipSourceOutOfBounds` uses `time_map::source_end`.
- `validate.rs:328`: `TimeOutOfRange` uses `source_end`.
- `codec.rs:161-170`: the v1→v2 migration finds the first `"kind":"time_remap"` whose `enabled != false`, then reads `params.speed.value.value`, `params.speed.keyframes` and `params.reverse.value.value` from the raw JSON.
- `codec.rs:94`: `SPEED_DENOMINATOR`, a deliberate copy of `MAX_APPROX_DEN` (comment at lines 91-93).

**core-commands**

| Location | What it does |
|---|---|
| `ops/effects.rs:52-61` `recheck_clip` | Re-checks source bounds after AddEffect (91), UpdateEffect (115), RemoveEffect (127), SetEffectEnabled (155) and SetKeyframes (230). Not after Reorder, Mask or Blend. |
| `ops/helpers.rs:66-72` `check_clip_times_representable` | Used by AddTrack (`ops/tracks.rs:12-14`) and CreateSequence (`ops/sequences.rs:42-44`) |
| `ops/helpers.rs:144-154` `check_source_bounds`; `:167-175` `check_cut_part` | Source bounds after edits and cuts |
| `ops/helpers.rs:296-305` `consume_head` | Advances the in-point by `source_offset(delta)` unless the clip is reversed, **then shifts every keyframe** (line 304) |
| `ops/helpers.rs:310-339` `trim_head` and `trim_head_in_place`; `:343-365` `trim_tail` | The tail trim has a separate path for reversed clips (349-361) |
| `ops/helpers.rs:370-385` `split_at` | |
| `ops/clips.rs` | Insert in ripple or overwrite mode (77-137), all trim modes (251-349), slip (351-369, uses `add_lossy` at 360), slide (371-417), split (419-453) |

**core-edit** (`timing.rs` unless noted)
- `454-463` `speed_patch` writes `SPEED_PARAM` and `REVERSE_PARAM`.
- `466-472` `SpeedPlan`.
- `482-554` `change_speed` issues UpdateEffect, SetEffectEnabled (resetting a disabled remap to 1x forward first, 503-506) or `AddEffect(constant_speed_node)` (525).
- `557-687` `set_clip_speed`:
  - range 0.001..=1000 (558), repeated in `error.rs:85-86`;
  - finds the first TimeRemap **whether or not it is enabled** (568);
  - refuses ramps (569-571, `error.rs:88-89`);
  - reads speed and reverse from the active node (572-574);
  - uses `source_span` (592).
- `select.rs:161-175` `sync_shift` uses `is_identity`.
- `op.rs:229-245` defines `EditOp::SetClipSpeed`; `expand.rs:84` dispatches it.

**core-query** (`lib.rs`)
- `ClipView.speed` and `reversed` (105-107, computed at 190-194, 208).
- `source_range` (206, 449).
- `source_time_at` (381).
- `describe_timeline` speed text (457-462).

**Other crates** (pointers only)
- `render/src/graph.rs:206`.
- `audio/src/mixer.rs:212-228`.
- `ui/src-tauri/src/service/framing.rs:113` inserts a Transform "before everything but a speed change".
- `cli/src/main.rs:1355`.

### 2.4 Hazards when moving the remap into a clip-level field
- **The remap's own `EffectId` is in journals.** `set_clip_speed` writes `UpdateEffect`, `SetEffectEnabled` and `AddEffect` (`timing.rs:505-546`), and users can write `SetKeyframes`, `RemoveEffect` or `ReorderEffect` against it. A v2 journal replayed after migration needs those commands translated.
- **Derived ids on a cut depend on list position.** The right-hand part's effects get `EffectId::derived(new_id, "split", i)`, where `i` is the index in `clip.effects` (`ops/helpers.rs:379-381`; the insert cut uses the same rule, `ops/clips.rs:40,58-60`). Take the remap out of the list and every node after it gets a different derived id. A v2 journal entry that targets such a node (for example UpdateEffect after SplitClip) would then fail with "not found" when replayed.
- **Speed keyframes are clip-local.** They shift with head trims (`ops/helpers.rs:304` calls `shift_keyframes`, `246-261`), after the offset has been computed from the old keyframes (line 298).
- **It shows up as an ordinary effect.** It appears in `EffectView` lists and in the `describe_timeline` effects list (`core-query/src/lib.rs:215, 464-467`), in `ChangeSet.effects`, and in `contains_ulid`.
- **AudioTimeStretch is meant to work with the remap.** Its description is "How audio follows a time remap" (`effects-registry/src/lib.rs:236-238`), but nothing reads it.
- **Every remap in a v1 fixture.** `core-project/tests/fixtures/project_before_exact_in_points.json` holds 13 remap nodes (schema 1). The tests at `core-project/tests/compat.rs:78,124` would exercise a v1→v2→v3 chain.

---

## 3. The registry, and everything that depends on a registry entry or a specific kind

### 3.1 What `effects-registry` describes
- `ParamSpec` (`lib.rs:16-29`): `id, title, description, kind: ParamKind, default: ParamValue, min: Option<f64>, max: Option<f64>, keyframable, enum_values`.
- `EffectSpec` (`lib.rs:31-45`): `kind: EffectKind, id, title, description, domain, params`.
- **What it does not describe:**
  - no units field (units appear only in the description text, such as "in pixels" or "dB");
  - no step, grouping, version or required flag;
  - no flag for whether a mask or blend is allowed;
  - no per-component ranges: `min` and `max` apply to every component (line 23).
- The catalog is a static `OnceLock` built at compile time (`lib.rs:242-247`).
- `spec()` **panics** for a kind without an entry (`lib.rs:249-251`).
- Whether a parameter can be keyframed is set by its type (`lib.rs:72-130`):
  - float, vec2, vec3, color, curve and shape: yes;
  - bool, text, enum and artifact: no.
- No Int or DenseCurve parameter exists in the catalog.

The catalog itself is `build()` at `lib.rs:132-240`. "kf" means the parameter can be keyframed.

| Kind (lines) | Params: type, default, range, kf, enum values |
|---|---|
| Transform (143-149) | position vec2 [0,0] unbounded kf; scale vec2 [1,1] 0..100 kf; rotation float 0 unbounded kf; anchor vec2 [.5,.5] -1..2 kf; opacity float 1 0..1 kf |
| Crop (150-156) | left, right, top, bottom: float 0, 0..1, kf; feather float 0 0..1000 kf |
| Composite (157-160) | blend enum "normal" {normal, add, multiply, screen, overlay, soft_light, hard_light, darken, lighten, difference}, not kf (**duplicates the `BlendMode` names**); opacity float 1 0..1 kf |
| Blur (161-165) | radius float 0 0..1000 kf; kind enum "gaussian" {gaussian, box, directional}; angle float 0 unbounded kf |
| ColorMatrix (166-175) | exposure 0 -10..10; contrast 1 0..4; saturation 1 0..4; temperature 0 -100..100; tint 0 -100..100; lift vec3 0 -1..1; gamma vec3 1 0.1..4; gain vec3 1 0..4 (all kf) |
| Curves (176-181) | master, red, green, blue: curve [(0,0),(1,1)], 0..1 on x and y, kf |
| Lut (182-185) | lut artifact "" (an empty key is allowed); intensity float 1 0..1 kf |
| TimeRemap (186-190) | speed float 1 0.001..1000 kf; reverse bool false; frame_blend enum "none" {none, blend, flow} |
| Mask (191-196) | shape: Shape Rect(0,0,1,1,0) kf; feather float 0 0..1000 kf; invert bool false; opacity float 1 0..1 kf |
| Text (197-208) | text ""; font_family "Inter"; font_size 48 1..2000; color white; position vec2 [.5,.5] -1..2; align enum "center" {left, center, right}; stroke_width 0 0..200; stroke_color black; box_width .8 0..1; line_height 1.2 .5..4 |
| Shader (209-215) | source text ""; u0..u3 float 0, unbounded |
| AudioGain (216-218) | gain_db 0 -96..24 |
| AudioEq (219-225) | low, mid, high _gain_db 0 -24..24; low_freq 200 20..20000; high_freq 4000 20..20000 |
| AudioCompressor (226-232) | threshold_db -18 -60..0; ratio 3 1..20; attack_ms 10 0..500; release_ms 100 0..5000; makeup_db 0 -24..24 |
| AudioPan (233-235) | pan 0 -1..1 |
| AudioTimeStretch (236-238) | preserve_pitch bool true |

The functions:
- `default_node` (254-260) and `fill_defaults` (263-269) are **unused outside the registry's own tests**.
- `default_value` (272-274).
- `check_range` (276-298) checks Float, Int, Vec2, Vec3 and Curve points. **Color is not range-checked.**
- `validate_value` (301-332) checks: the parameter is known, the type matches, the value is finite, an enum value is allowed, a shape is well formed, a curve has at least 2 points with strictly rising x, and the range.
- `validate_mask` (335-341) is purely structural.
- `validate_keyframes` (345-365) checks that the parameter can be keyframed, times strictly increase, every value passes `validate_value`, and tangents are finite.
- `validate_node` (369-379).
- `describe` (382-384) produces JSON for the UI and the AI. It goes through `to_value`, so its keys come out alphabetically sorted.
- The error type `ParamError` is at 47-70.

### 3.2 Behaviour that depends on the registry or on a specific kind (core crates)

I use three labels:
- **Structural**: still valid if the kind became an opaque block id.
- **Spec-driven**: generic over any entry, so it works for a plug-in block if the block's manifest supplies the same fields.
- **Kind-specific**.

| Where | What | Label |
|---|---|---|
| `core-commands/src/ops/helpers.rs:178-191` `validate_effect_for` | `validate_node` plus a check of `node.kind.domain()` against the track kind | spec-driven, plus kind-specific for the domain (it comes from the enum, not the spec) |
| `ops/effects.rs:82` (AddEffect), `ops/helpers.rs:230` (InsertClip, via `ops/clips.rs:37`) | Call `validate_effect_for` | spec-driven |
| `ops/effects.rs:103` (UpdateEffect `set`) | `validate_value` per value. `clear_keyframes` and `remove` are unchecked (106-113): an unknown name is ignored, and a removed parameter falls back to its default | spec-driven |
| `ops/effects.rs:207-212` (SetKeyframes) | The parameter must be in the spec (`UnknownParam`), and a missing parameter is **materialised with the registry default** | spec-driven, and it affects state contents |
| `ops/effects.rs:209` | `validate_keyframes` | spec-driven |
| `ops/effects.rs:163` (SetEffectMask) | `validate_mask` | structural |
| `ops/effects.rs:173-180` (SetEffectBlend) | No validation and no check that the kind supports blending | structural (gap) |
| `ops/tracks.rs:22-27` (AddTrack), `ops/sequences.rs:49-52` (CreateSequence) | Only the core-model `validate`. **No registry check.** The probe: AddTrack accepted opacity = 5.0, an unknown parameter `not_a_param` and Blur `kind = "triangle"`, both on track effects and on a clip inside the track. InsertClip refused the same clip. | gap |
| `core-model/src/validate.rs:146-201` | Id uniqueness (155), finite params (178), keyframes sorted (182-188), keyframe type matches the value type (190-192), keyframes finite (194-196), mask well formed (172-176) | structural |
| `validate.rs:156-165` | Domain check through `kind.domain()` | kind-specific mapping |
| `validate.rs:166-171` | TimeRemap placement | kind-specific |
| `core-model/src/project.rs:332-347, 350-373` | `referenced_blob_keys` and `referenced_artifact_keys` scan parameters and keyframes by `ParamValue` type, and `node.mask`. **Transition params are not scanned.** Blob clean-up relies on them (`core-project/src/lib.rs:359-366, 397-418`) | structural (an opaque node must still expose these references, or its blobs get deleted on save) |
| `ops/helpers.rs:246-261` `shift_keyframes` | Shifts every keyframe of every effect on a head trim | structural (needs keyframes visible even for unknown blocks) |
| `ops/helpers.rs:379-381`; `ops/clips.rs:56-65`; `core-edit/src/timing.rs:444-446` | Derived or fresh effect ids on cut and duplicate | structural |
| `core-query/src/lib.rs:166-186` `effect_view` | `spec(kind).title`; `kind_id = kind.id()` | spec-driven (panics if there is no entry) |
| `core-query/src/lib.rs:465` | `spec(kind).title` in `describe_timeline` | spec-driven (panics if there is no entry) |
| `ops/helpers.rs:100-111`; `core-model/src/project.rs:264-272`; `validate.rs:309-313` | `GeneratorKind::track_kind` | kind-specific (generators) |
| Every row in section 2.3 | Time remap | kind-specific |

**Pointers outside the core crates.** These matter for "renders pixel for pixel the same" and belong to another researcher's area:
- The renderer falls back to registry defaults for missing parameters (`render/src/graph.rs:116-136, 243`). Defaults therefore affect the image, because nodes may be sparse.
- The renderer combines effects as accumulated settings, not as a stack: Transform, Crop, Composite (opacity only), ColorMatrix and Mask (shape parameter only) (`graph.rs:228-272`). Sequence effects use ColorMatrix only (`graph.rs:331-333`).
- **`EffectNode.mask` and `EffectNode.blend` are read by nothing in render, audio or export.**
- **Blur, Curves, Lut, Text, Shader, AudioEq, AudioCompressor and AudioTimeStretch are read by no renderer or mixer.**
- Transitions ignore `kind` and always cross-fade (`graph.rs:302-326`; `audio/src/mixer.rs:286-315`).
- The mixer reads AudioGain and AudioPan with hard-coded defaults (`mixer.rs:112-133`).
- The UI gets the catalog from `ui/src-tauri/src/commands.rs:133`. The generated TypeScript types use closed unions (`ui/src/bindings/types.ts:313, 1852`).
- The CLI parses kinds with `EffectKind::from_id` and `ALL` (`cli/src/util.rs:302-314`).

---

## 4. Document versioning, canonical JSON and how serde treats unknown data

- **`SCHEMA_VERSION = 2`** (`core-model/src/project.rs:16-23`, with a changelog comment):
  - 1 is the first format;
  - 2 has the same fields, but the in-point of a speed-changed clip is now exact.
  - It is stored as the field `ProjectState.schema_version` (`project.rs:42`). `validate` reports a mismatch (`validate.rs:208-210`).
- **Migrations** (`codec.rs:77-330`):
  - The type is `Migration = fn(&mut Value) -> Result<(), String>` (86).
  - `MIGRATIONS[i]` upgrades version i+1 to i+2; today it holds only `exact_in_points` (89).
  - They work on the **untyped JSON tree before typed parsing** (`from_json_value`, `codec.rs:61-70`).
  - `migrate` (313-329) runs the steps one after another and rewrites `schema_version` after each.
  - A document newer than the build is refused with `NewerSchema` (63-65). A missing or invalid version is `MissingVersion` (54-59).
  - House rule: a migration describes how the *old* build read the file, so constants are copied rather than imported (91-94).
- **How migrations are tested:**
  - `migration_chain_matches_schema_version` (`codec.rs:343-346`).
  - Schema-1 documents are made by writing a current state and setting `schema_version = 1` (`codec.rs:364-394`).
  - The old build's frame rule is written out again by hand (`frame_then`, 401-410) and compared with the new `time_map` over 600 random cases (419-469).
  - A test that untouched cases stay untouched (472-508), and an end-of-media case (513-562).
  - A real fixture written by the old build: `core-project/tests/fixtures/project_before_exact_in_points.json`, used by `core-project/tests/compat.rs:78` (same frames shown) and `:124` (old journal replayed, not lost).
- **Canonical JSON:**
  - `to_json` is `serde_json::to_string(state)`: compact (`codec.rs:24-27`).
  - `state_hash` is BLAKE3 hex of those bytes (`codec.rs:72-75`).
  - The header comment says "sorted object keys" (`codec.rs:3-4`), but in fact **struct fields come out in Rust declaration order and only map keys are sorted** (`OrdMap` and `BTreeMap` by key order). That covers `params`, `assets`, `sequences`, `metadata` and transition `params`.
  - `None` is written as `null`; empty collections are always written.
  - f64 values use serde_json's shortest exact form (`12.0`). `float_roundtrip` is on (`Cargo.toml` workspace dependencies), so reading back gives the identical number.
  - `RationalTime` and `TimeRange` are normalised when read (`core-time/src/rational.rs:364-374`, `range.rs:122-132`), so `{"num":2,"den":4}` is written back as `{"num":1,"den":2}`.
  - serde_json's `preserve_order` is **not** enabled anywhere in the workspace (checked with `cargo tree`). A node kept as `serde_json::Value` therefore **comes back with its keys sorted alphabetically**. The probe showed `{"blend":…,"enabled":…,"id":…,"kind":…,"mask":…,"params":…,"provenance":…}` instead of `id, kind, enabled, params, mask, blend, provenance`. Byte-for-byte preservation of unknown nodes needs either the struct field order or `RawValue`. The `raw_value` feature is on only through Tauri's feature unification, not in `core-model`'s own manifest.
  - Hashes are recorded for every checkpoint (`core-project/src/lib.rs:157-158, 404-405`) and every journal entry (`lib.rs:298-302`).
- **Unknown fields and unknown values:**
  - There is **no `deny_unknown_fields`, no `flatten` and no `untagged`** in the core crates or the registry (grep).
  - An unknown field is silently dropped on read and lost on save (probe: `"future_field":42` disappeared).
  - An unknown **enum value** (an `EffectKind`, a `ParamValue` type, `BlendMode`, `MaskShape`, `Interp`, `TransitionKind`, `GeneratorKind`, a `ClipSource` type) makes **the whole document unreadable**. The probe got `invalid project document: unknown variant 'glow', expected one of 'transform', …` and the same for the transition `"spin"`.
  - Missing optional fields are filled with defaults, so a sparse hand-written node gains all seven keys when saved.

---

## 5. Journal replay

**What runs.**
- `Project::open` reads the newest checkpoint with `from_json_versioned` (`core-project/src/lib.rs:196`). It refuses only violations that make arithmetic unsafe (`lib.rs:212-216`). **The registry is never run against a loaded document**: a stored out-of-range parameter opens without complaint.
- For each journal row it deserialises a `Transaction` with the *current* types (`lib.rs:236`) and calls `store.commit` (`lib.rs:237`). That goes to `apply_transaction` (`core-commands/src/store.rs:67-84`, `apply.rs:47-54`), which runs the same `ops::dispatch` as live editing.
- There is no separate "trusted replay" path and no full `validate` afterwards (`apply_checked`, `apply.rs:59-66`, is not used).
- After each entry the state hash is compared when the stored schema equals the current one (`lib.rs:202, 247-252`).
- Undo and redo rows hold only a hash (`lib.rs:239-244, 314-345`).

**So the registry is re-checked on replay** for AddEffect and InsertClip (`validate_node`), UpdateEffect `set` (`validate_value`), and SetKeyframes (`default_value` and `validate_keyframes`); SetEffectMask uses the structural `validate_mask`. It is **not** re-checked for AddTrack or CreateSequence (core `validate` only), or for Remove, Reorder, Enabled and Blend.

**If the installed blocks change between writing the journal and replaying it:**
1. **Today**, an unknown kind string fails to parse. In a checkpoint that is a `Codec` error; in a journal row it is `ProjectError::Json` at `lib.rs:236` (`lib.rs:69-76`). Either way **the whole project fails to open**. A checkpoint containing it cannot be opened even with `open_discarding_journal`.
2. If kinds become strings but replay still calls the registry, a missing block makes `spec()` panic today (`effects-registry/src/lib.rs:250`), or as a `Result` produces `ProjectError::Replay` (`lib.rs:237`, `87-88`). Open still fails.
3. If a block is updated with a narrower range, a removed or renamed parameter, a different type, keyframing switched off, or a different domain, commands that were accepted when written are refused on replay. That is `Replay`, and open fails.
4. If a block's **default changes**, SetKeyframes on a parameter the node did not yet have stores the new default (`ops/effects.rs:210-212`). The state changes, `JournalMismatch` follows (`lib.rs:249-251`), and the project can only be opened by discarding unsaved work.
5. A v2→v3 format change disables hash checks for the old journal (`lib.rs:202`), but **journal transactions are never migrated**: only checkpoints go through `codec`. Old commands that carry `"kind":"blur"` nodes, BlendMode values, remap effect ids, or split-derived ids that depend on list position (section 2.4) must still parse and still apply under the new model.

---

## 6. Inventory: every non-test location in `core-*` and `effects-registry` that names a specific effect kind, parameter name, blend mode, transition kind or generator kind

Labels: **S** structural, **SD** spec-driven, **KS** kind-specific. Test code is excluded (it starts at `effect.rs:570`, `timeline.rs:478`, `validate.rs:342`, `codec.rs:331`, `time_map.rs:366`, `effects-registry/src/lib.rs:386`, `core-query/src/lib.rs:496`, `core-edit/src/expand.rs:313`, `core-project/src/lib.rs:484`).

| file:line | What it does | Label |
|---|---|---|
| core-model/src/effect.rs:9-30 | The closed `EffectKind` enum (16 variants) | KS |
| effect.rs:32-39 | `EffectDomain` (Any exists only for TimeRemap) | S concept, KS use |
| effect.rs:42-59 | `EffectKind::ALL` | KS |
| effect.rs:62-81 | The `core.*` id strings | KS |
| effect.rs:83-85 | `from_id` over `ALL` | KS |
| effect.rs:87-97 | Kind-to-domain mapping | KS |
| effect.rs:100-114 | The closed `BlendMode` set | KS (closed list of values) |
| effect.rs:116-150 | `MaskShape` and its validity rules | S (parameter type) |
| effect.rs:152-187 | `MaskRef` and its validity rules (the doc comment names `EffectKind::Mask`) | S |
| effect.rs:209-306 | `ParamKind` and `ParamValue` | S (closed set of parameter types) |
| effect.rs:500-515 | `EffectNode.kind: EffectKind`, `blend: Option<BlendMode>` | KS (the field types) |
| core-model/src/timeline.rs:89-99 | `GeneratorKind` | KS |
| timeline.rs:101-108 | Generator-to-track-kind mapping (Silence and Tone are audio) | KS |
| timeline.rs:110-115 | `ClipSource::Generator { kind }` | KS |
| timeline.rs:130-140 | `TransitionKind`, with `Custom(String)` | KS (with an escape hatch) |
| timeline.rs:180-182 | Doc: speed is a TimeRemap effect | KS (doc) |
| core-model/src/project.rs:264-272 | `clip_track_kind` through `GeneratorKind::track_kind` | KS |
| project.rs:332-373 | Blob and artifact references by parameter type and `MaskRef::Artifact` | S |
| core-model/src/validate.rs:45,110 | `EffectDomainMismatch` | KS through domain |
| validate.rs:50,119-121 | `TimeRemapMisplaced` | KS |
| validate.rs:156-165 | Domain check | KS mapping |
| validate.rs:166-171 | `EffectKind::TimeRemap` placement and count | KS |
| validate.rs:300, 328 | Bounds and time limit via `time_map::source_end` (depends on the remap) | KS |
| validate.rs:309-313 | Generator track kind | KS |
| core-model/src/codec.rs:94 | `SPEED_DENOMINATOR` | KS |
| codec.rs:161, 163, 169-170 | `"time_remap"`, `"speed"`, `"reverse"` in the migration | KS |
| core-model/src/time_map.rs:51-52 | `"speed"`, `"reverse"` | KS |
| time_map.rs:56 | `MAX_APPROX_DEN` (speed quantisation) | KS |
| time_map.rs:59-61 | Finds the first enabled `EffectKind::TimeRemap` | KS |
| time_map.rs:64-107, 196-271, 280-282, 311-357 | The speed and reverse maths | KS |
| time_map.rs:360-364 | `constant_speed_node` | KS |
| effects-registry/src/lib.rs:31-39 | `EffectSpec` keyed by `EffectKind` | KS key, SD content |
| lib.rs:51 | `ParamError::UnknownParam { kind: EffectKind }` | KS type |
| lib.rs:72-130 | Keyframing policy per parameter type | SD |
| lib.rs:132-240 | Every kind and parameter literal (table in 3.1) | KS (the catalog) |
| lib.rs:158 | Composite blend enum repeating the `BlendMode` names | KS |
| lib.rs:186-190 | TimeRemap spec, including the speed range and `frame_blend` | KS |
| lib.rs:249-251 | `spec()` panics on a missing entry | KS assumption |
| lib.rs:254-384 | defaults and validation functions, `describe` | SD (validate_mask: S) |
| core-commands/src/command.rs:144-158 | Effect commands (BlendMode and MaskRef payloads) | S |
| command.rs:128 | `SetTransition` | S |
| core-commands/src/error.rs:48-51, 62-66 | `EffectDomainMismatch`, `Param(ParamError)` | S/SD |
| ops/effects.rs:52-61 | `recheck_clip` (exists because of the remap) | KS |
| ops/effects.rs:63-76, 83 | TimeRemap placement | KS |
| ops/effects.rs:82, 103, 163, 207-212 | Registry calls | SD (163: S) |
| ops/effects.rs:173-180 | Blend set without any check | S |
| ops/helpers.rs:66-72, 144-175 | Bounds via `time_map` | KS |
| ops/helpers.rs:100-111, 119 | Generator kind and source rate | KS |
| ops/helpers.rs:178-191 | `validate_effect_for` (registry plus domain) | SD + KS |
| ops/helpers.rs:193-204 | Transition validation (kind ignored) | S |
| ops/helpers.rs:224-237 | One TimeRemap per new clip | KS |
| ops/helpers.rs:246-261 | `shift_keyframes` | S |
| ops/helpers.rs:296-305, 343-365 | Head and tail trims through `is_reversed` and `source_offset` | KS |
| ops/helpers.rs:379-383 | Derived effect ids on a cut; transitions dropped | S (positional ids) |
| ops/clips.rs:56-65 | Derived ids reserved for an insert cut | S |
| ops/clips.rs:360 | Slip via `add_lossy` | S |
| ops/clips.rs:496-508 | `set_transition` | S |
| ops/tracks.rs:12-14, 22-27; ops/sequences.rs:42-44, 49-52 | Remap-aware time checks; core `validate` only, no registry | KS / gap |
| core-edit/src/timing.rs:8-9 | Imports `SPEED_PARAM`, `REVERSE_PARAM`, `EffectKind` | KS |
| timing.rs:444-446 | Fresh effect ids on duplicate | S |
| timing.rs:454-463, 482-554, 557-687 | Speed patch, `change_speed`, `set_clip_speed` (see 2.3) | KS |
| core-edit/src/select.rs:165 | `is_identity` for sync | KS |
| select.rs:188-194 | Filler clip uses `GeneratorKind::SolidColor` or `Silence` | KS |
| core-edit/src/error.rs:85-89 | `InvalidSpeed` (0.001..1000), `SpeedRamp` | KS |
| core-edit/src/op.rs:229-245; expand.rs:84 | `SetClipSpeed` | KS |
| core-edit/Cargo.toml:13 | Depends on `effects-registry` but never uses it | — |
| core-query/src/lib.rs:105-107, 190-194, 206, 208, 381, 449, 457-462 | Speed and reversed views via `time_map` and `SPEED_PARAM` | KS |
| core-query/src/lib.rs:118-122, 213-214 | `TransitionView` (kind and duration only, no params) | S |
| core-query/src/lib.rs:135-140, 167, 172, 465 | `EffectView.kind`, `kind_id`, title from `spec()`, blend | SD (panics on a missing entry) |
| core-query/src/lib.rs:446, 469, 472 | Generator and transition kinds printed with Rust's internal names (`SolidColor`, `CrossDissolve`, `Custom("iris")`), not the stored names | KS |
| core-query/src/lib.rs:464-467 | `describe_timeline` names clip effects only, not track or sequence effects | S (gap) |
| core-project/src/lib.rs:236-252 | Journal replay re-runs operations, then checks the hash | S/SD |
| core-project/src/lib.rs:359-366 | Blob clean-up through `referenced_blob_keys` | S |

---

## 7. Where the design documents differ from the code

`docs/ARCHITECTURE.md` §11 and §12 have drifted from the code:
- `Param.keyframes` is written as an `Option` there; the code uses a plain vector.
- §11 lists the bezier fields as `in_tangent`, then `out_tangent`. The code, and therefore the file, has `out_tangent` first.
- §11's `ParamValue` list has no `Shape`.
- `Provenance` is shown with `Version` and `Hash` types; the code uses plain strings.
- §11 lists a `PutBlob` command, which does not exist. Blobs go through `Project::put_blob` (`core-project/src/lib.rs:436`).
- §12 describes TimeRemap as "DenseCurve or keyframes mapping clip time → source time". The code integrates a keyframable `speed` instead.
- §12 describes ColorMatrix as a 4x4 matrix, Text as "rich text runs" and Shader as "declared uniforms". The catalog has named parameters, plain text, and four fixed float uniforms `u0`–`u3`.