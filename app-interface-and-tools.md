# Effects outside the core crates: what the app backend, the typed contract, the window, the command-line tool and the end-to-end tests know about effects

This covers effect knowledge outside `crates/core-*`, `crates/render`, `crates/audio` and `crates/export`. Nothing was changed. All paths are relative to `C:/Users/Mauro/Documents/CLAUDESTART`.

## 0. Key findings

1. **The catalogue is untyped and fixed for the whole run.**
   - `effects_catalog` is a free function with no app state. It returns `serde_json::Value` from the static `effects_registry::describe()` (`ui/src-tauri/src/commands.rs:131-134`), which is backed by a one-time global, `static REGISTRY: OnceLock<Vec<EffectSpec>>` (`crates/effects-registry/src/lib.rs:242-247`).
   - In TypeScript its type is `unknown` (`ui/src/bindings/commands.ts:297-300`). It is not a root of the schema generator.
   - The window asks for it once and stores it. The comment says it "never changes while the app runs" (`ui/src/features/inspector/live.ts:1-3,22-37`). No event tells the window that the catalogue changed.
2. **`EffectKind` is a closed list on every surface.**
   - Generated TypeScript union: `ui/src/bindings/types.ts:1848-1852`.
   - Checked-in JSON schemas: for example `schemas/Command.schema.json:358-379`. The same list is in `Transaction`, `ClipView`, `TimelineView`, `EffectView` and `ScratchRequest`.
   - Command-line parsing: `crates/cli/src/util.rs:299-306`.
   - The AI is meant to receive `schemas/EditOp|Command|Transaction` (`ui/src-tauri/src/schema.rs:5-7`). Those files are generated at build time, so a block installed at runtime can never appear in them. The AI will need the catalogue at runtime.
3. **The window holds hand-written effect knowledge**, despite the "no per-effect code" claim in its own comments:
   - `GESTURE_ONLY_KINDS`, `STANDARD_KINDS` and `DRAWN_KINDS` (`ui/src/features/inspector/catalog.ts:117-147`).
   - Units guessed from parameter names and descriptions (`controls.ts:57-83, 217`).
   - A rule that every 3-number parameter is R, G, B (`controls.ts:169`).
   - Transition names and the hard-coded `cross_dissolve` (`Sections.tsx:26`; `timeline/actions.ts:326-346`; `timeline/paint.ts:225`).
   - The "fx" badge filter on the timeline (`paint.ts:97`).
4. **Two paths would not keep an effect or transition intact:**
   - `TransitionView` has no `params`, and the inspector rewrites a transition as `{ kind, duration }` when its length is changed (`Sections.tsx:49`). Transition parameters are dropped.
   - The command-line effect subcommands go through `effects_registry::spec(kind)`, which panics for an unknown kind (`crates/effects-registry/src/lib.rs:249-251`; `crates/cli/src/main.rs:1255`).
5. **Folders:**
   - Settings live in `VEP_CONFIG_DIR`, otherwise `%APPDATA%\vep` (`ui/src-tauri/src/settings.rs:280-299`). The command-line tool has a second copy of that logic (`crates/cli/src/export_cmd.rs:91-103`).
   - The cache lives in `VEP_CACHE_DIR`, otherwise `%LOCALAPPDATA%\vep\cache` (`crates/cache/src/layout.rs:83-91`).
   - `tauri.conf.json` has no `bundle.resources`.
   - The end-to-end harness copies **only the exe** into a temporary `bin` folder (`ui/e2e/harness/app.ts:750-754`). Built-in blocks found next to the exe would therefore be missing in end-to-end runs.

---

## 1. The typed contract

### 1.1 Commands that carry effect information

All commands are thin wrappers registered in `ui/src-tauri/src/lib.rs:91-162`. The window calls them through `ui/src/bindings/commands.ts`.

| Command | Rust | TypeScript wrapper | Arguments → answer | Effect content |
|---|---|---|---|---|
| `effects_catalog` | `commands.rs:131-134` (no `State`, calls `effects_registry::describe()`) | `commands.ts:297-300` → `Promise<unknown>` | () → JSON array | The whole catalogue, shape in §1.4 |
| `timeline` | `commands.rs:116-119` | `commands.ts:279-281` | `sequence?` → `TimelineView \| null` | Sequence `effects`, `tracks[].effects`, `tracks[].clips[].effects/speed/reversed/transition_in/out` |
| `clip` | `commands.rs:121-124` | `commands.ts:284-286` | `clip` → `ClipView \| null` | Same clip fields |
| `edit_apply` | `commands.rs:138-141` | `commands.ts:306-324` (doc mentions "speed", line 308) | `op: EditOp` → `EditOutcome` | The `set_clip_speed` gesture. Errors `invalid_speed`, `speed_ramp` |
| `edit_preview` | `commands.rs:143-146` | `commands.ts:326-332` | `op` → `EditPreview` | Speed changes show as `trimmed`/`modified` (`ClipChange`, `types.ts:562-582`) |
| `apply` | `commands.rs:148-151` | `commands.ts:334-345` ("an effect parameter") | `transaction: Transaction` → `ApplyOutcome` | Every effect, transition and keyframe `Command` (§1.3). `ApplyOutcome.changes.effects` |
| `fit_to_frame` | `commands.rs:153-156` → `service/framing.rs:143-156` | `commands.ts:347-366` | `clips, fill` → `EditOutcome` | The backend adds or updates a **Transform** `scale` (§2.3) |
| `scratch_set` | `commands.rs:203-206` → `service/edit.rs:392-414` | `commands.ts:466-484` | `request: ScratchRequest` → `ScratchOutcome` | `{kind:'transaction'}` is how slider previews of effect parameters reach the preview monitor |
| `scratch_clear` | `commands.rs:208-211` | `commands.ts:486-496` | () | Ends a preview |
| `drag_update` / `drag_end` | `commands.rs:193-201` | — | `EditOp` inside `DragUpdate`/`DragFrame` | The type includes `set_clip_speed`; drags do not use it |
| `project_info` | `commands.rs:111-114` | — | → `ProjectInfo` | `summary.counts.effects` (`ProjectCounts`, `types.ts:3422-3429`) |
| `project_open` | `commands.rs:96-99` → `service/project.rs:293-300` | — | → `ProjectOpened` | `report.violations: string[]` (`contract.rs:253-268`). Validation problems with effects, as sentences. The natural place to report missing blocks |
| `new_ids` | `commands.rs:178-181` | `commands.ts:407` | `n` → ids | The window makes `EffectId`s through this |

`export_start` / `export_still` (`service/exporting.rs:236, 280`) and the player (`ui/src-tauri/src/player.rs:655, 692, 848`) pass the whole `ProjectState` to `export::`, `audio::mix_range` and `render::Renderer`. They contain no effect knowledge, but any block registry would have to travel through these calls.

### 1.2 Events

- **`project:changed` → `ProjectChanged`** (`ui/src-tauri/src/events.rs:49-64`; `types.ts:3394-3420`):
  - `changes: ChangeSet` carries `effects: EffectId[]` and `removed: EntityRef[]`, which may be `{kind:'effect', id}` (`types.ts:519-534`, `1989-2011`).
  - The window does not use `changes.effects`. Every change re-reads the summary and the timeline (`ui/src/state/sync.ts:116-139`).
- There is **no catalogue or block event**. The event list is a fixed-size `AppEvent::NAMES: [&str; 9]` (`events.rs:166`). `tests/bindings.rs:139-162` requires each name to be in `events.ts` (`EVENT_NAMES` at `ui/src/bindings/events.ts:30`) with an `on('<name>', handler)` function and a doc line `` * `<name>`: ``, and in the generated `AppEvent` union.

### 1.3 Types: exact shapes (generated `ui/src/bindings/types.ts`)

```ts
// 313
export type BlendMode = 'normal' | 'add' | 'multiply' | 'screen' | 'overlay' | 'soft_light' | 'hard_light' | 'darken' | 'lighten' | 'difference'
// 1846-1852
export type EffectId = string
/** The closed set of primitives the renderer understands. Nothing else is
 * ever stored in a project. Adding a variant here is a renderer change. */
export type EffectKind = 'transform' | 'crop' | 'composite' | 'blur' | 'color_matrix' | 'curves' | 'lut' | 'time_remap' | 'mask' | 'text' | 'shader' | 'audio_gain' | 'audio_eq' | 'audio_compressor' | 'audio_pan' | 'audio_time_stretch'
// 1857-1865
export interface EffectNode { blend?: BlendMode | null; enabled?: boolean; id: EffectId; kind: EffectKind; mask?: MaskRef | null; params?: Record<string, Param>; provenance?: Provenance | null }
// 1870-1879
export type EffectTarget = { kind: 'clip'; id: ClipId } | { kind: 'track'; id: TrackId } | { kind: 'sequence'; id: SequenceId }
// 3075-3078 / 3083 / 3088-3101
export interface Param { keyframes?: Keyframe[]; value: ParamValue }
export type ParamId = string
export interface ParamPatch { clear_keyframes?: ParamId[]; remove?: ParamId[]; set?: Record<string, ParamValue> }
// 3108-3144
export type ParamValue = {type:'float';value:number} | {type:'int';value:number} | {type:'bool';value:boolean}
  | {type:'vec2';value:[number,number]} | {type:'vec3';value:[number,number,number]} | {type:'color';value:Rgba}
  | {type:'text';value:string} | {type:'enum';value:string} | {type:'curve';value:CurvePoint[]}
  | {type:'shape';value:MaskShape} | {type:'dense_curve';value:BlobKey} | {type:'artifact';value:ArtifactKey}
// 2757-2764, 2548-2559
export interface Keyframe { interp?: Interp; time: RationalTime /* clip-local */; value: ParamValue }
export type Interp = {type:'hold'} | {type:'linear'} | {type:'bezier'; in_tangent:[number,number]; out_tangent:[number,number]}
// 2803-2840
export type MaskRef = { type:'shape'; feather?: number; invert?: boolean; shape: MaskShape } | { type:'artifact'; feather?: number; invert?: boolean; key: ArtifactKey }
export type MaskShape = {type:'rect';corner_radius?:number;height:number;width:number;x:number;y:number} | {type:'ellipse';cx:number;cy:number;rx:number;ry:number} | {type:'path';closed?:boolean;points:[number,number][]}
// 4348-4361
export interface Transition { duration: RationalTime; kind: TransitionKind; params?: Record<string, ParamValue> }
export type TransitionKind = 'cross_dissolve' | 'dip_to_black' | 'dip_to_white' | 'wipe' | 'slide' | 'zoom' | { custom: string }
export interface TransitionView { duration: RationalTime; kind: TransitionKind }   // NOTE: no params
// 3572-3580
export interface Provenance { input_hash: string; plan_step?: StepId | null; skill_id: SkillId; skill_version: string }
// 1881-1892, 3146-3150
export interface EffectView { blend: BlendMode | null; enabled: boolean; has_mask: boolean; id: EffectId; kind: EffectKind; kind_id: string; params: Record<string, ParamView>; provenance: Provenance | null; target: EffectTarget; title: string }
export interface ParamView { animated: boolean; keyframe_count: number; value: ParamValue }
```

- **The read views lose information.** `EffectView` gives `has_mask` but not the mask, and a keyframe count but not the keyframes. `kind_id` is `"core.<kind>"` and `title` comes from the registry (`crates/core-query/src/lib.rs:132-143, 167-173`).
- **`Clip`** (`types.ts:536-557`): `effects?: EffectNode[]`, `transition_in?/transition_out?: Transition | null`. Its doc says speed changes "are a `TimeRemap` effect, never a separate field" (`types.ts:539-542`).
- **`ClipView`** (`types.ts:634-663`): `effects: EffectView[]`, `reversed: boolean`, `speed: number | null` (`null` means a keyframed ramp), `transition_in/out: TransitionView | null`. The speed fields are computed from `time_remap` in core-query (`crates/core-query/src/lib.rs:190-208`).
- **`TrackView.effects` and `TimelineView.effects`** (`types.ts:4313-4322`, `4276-4288`): the window never reads them (no track or sequence effect UI).
- **`GeneratorKind`** (`types.ts:2396-2405`) includes `'text'` and `'adjustment'` ("Applies its effect stack to everything beneath it").
- **`Command` effect variants** (`types.ts`):

  ```ts
  { type: 'set_transition'; clip; edge; transition?: Transition | null }   // 859-864
  { type: 'add_effect'; effect: EffectNode; index?: number | null; target }  // 885-890
  { type: 'update_effect'; id; patch: ParamPatch }                           // 891-895
  { type: 'remove_effect'; id }                                              // 896-899
  { type: 'reorder_effect'; id; index }                                      // 900-904
  { type: 'set_effect_enabled'; enabled; id }                                // 905-909
  { type: 'set_effect_mask'; id; mask?: MaskRef | null }                     // 910-914
  { type: 'set_effect_blend'; blend?: BlendMode | null; id }                 // 915-919
  { type: 'set_keyframes'; effect; keyframes; param; range? }                // 920-931
  ```

- **`EditOp` `set_clip_speed`** (`types.ts:1770-1792`): `{ clips; include_linked?; reverse?; ripple?; ripple_scope?; speed: number }`.
- **`EditErrorDetail`**: `invalid_speed {speed}` and `speed_ramp {clip}` (`types.ts:1477-1490`; Rust `ui/src-tauri/src/error.rs:192-195, 228-229`).
- **`CommandErrorDetail`** (`types.ts:972-981`; `error.rs:242-257`): `{ type: string; detail: unknown }`. This is `core_commands::CommandError` serialised. The effect-related variants are `effect_domain_mismatch {effect}`, `param` (a nested `ParamError` that carries `kind: EffectKind` for `unknown_param`) and `invalid_transition {clip, reason}` (`crates/core-commands/src/error.rs:48-53`).
- **`ScratchRequest`** (`contract.rs:437-443`; `types.ts:3847-3860`): `{kind:'op'; op: EditOp} | {kind:'transaction'; transaction: Transaction}`. The doc names "an effect parameter while a slider is dragged".

### 1.4 How `effects_catalog` reaches the window

1. `effects_registry::describe()` serialises the static registry (`crates/effects-registry/src/lib.rs:382-384`). `EffectSpec`/`ParamSpec` derive `Serialize` only, not `JsonSchema` (`lib.rs:16-39`), which is why the catalogue is not in the typed contract.
2. Shape, taken from the checked-in copy `ui/src/features/inspector/catalog.fixture.ts:9-…`:
   ```json
   [{"kind":"transform","id":"core.transform","title":"Transform","description":"…","domain":"video|audio|any",
     "params":[{"id":"opacity","title":"Opacity","description":"Layer opacity.","kind":"float",
                "default":{"type":"float","value":1},"min":0,"max":1,"keyframable":true,"enum_values":[]}]}]
   ```
3. `commands.rs:131-134` → handler list `lib.rs:112` → `effectsCatalog()` (`commands.ts:298-300`).
4. `ensureCatalog()` (`ui/src/features/inspector/live.ts:25-37`) is called once (an `asking` flag at line 22). It stores the result in the zustand `useCatalog` store (lines 13-20). An empty parse becomes the error "The list of effects could not be read." (line 31).
5. `parseCatalog` (`catalog.ts:102-105`) checks each field: a missing `domain` defaults to `'video'` (line 90) and the kind is cast with `raw.kind as EffectKind` (line 92).
6. `useEffectsCatalog()` (`live.ts:40-44`) → `InspectorPanel` (`ui/src/features/inspector/index.tsx:40`) → `EffectGroup` (`EffectSection.tsx:106`). **The inspector is the only consumer.**
7. Because of the window's rule that features do not import each other (`ui/src/architecture.test.ts:46-55`), the timeline cannot read this store unless it moves to `ui/src/state`.
8. Other readers: the backend test `ui/src-tauri/tests/ipc.rs:152` (only checks "is an object or an array") and the end-to-end check `ui/e2e/panels-inspector.e2e.ts:69`, `expect(await editor.invoke('effects_catalog')).toEqual(CATALOG_FIXTURE)`.

### 1.5 The schema generator and the staleness test

- **Generator** (`ui/src-tauri/src/schema.rs`):
  - `roots()` (87-180) lists the root types. The engine roots are lines 93-107: `EditOp`, `EditPreview`, `SnapPoint`, `SnapResult`, `Command`, `Transaction`, `ChangeSet`, `ProjectSummary`, `TimelineView`, `ClipView`, `AssetSummary`, **`EffectView`** (104) and the time types.
  - Each root is marked `sent` (as read, so fields with defaults are optional) or `root`/received (every field present, absent values are `null`) (30-83).
  - `generate()` (184-193) writes `schemas/<Name>.schema.json` per root plus `ui/src/bindings/types.ts`.
  - `definitions()` (201-213) merges the definitions of both directions. The sent form wins for types that travel both ways.
  - `write()` (227-245) also deletes orphaned schema files.
  - `stale()` (250-255) compares with line ends normalised (257-260).
  - The TypeScript is produced by Rust code in the same file (`typescript()`, 283-310).
- **Running it:** `cargo run -p vep-app --example gen_bindings` (`ui/src-tauri/examples/gen_bindings.rs`, `--check` at 16-27), or `npm run bindings` / `bindings:check` (`ui/package.json` scripts).
- **Staleness test:** `ui/src-tauri/tests/bindings.rs:25-29` fails if any generated file differs. Other tests in that file:
  - Engine schemas exist and list the gestures, including `set_clip_speed` (31-44).
  - The command lists in `commands.rs`, `lib.rs` and `commands.ts` are equal and each function has a doc comment (48-75).
  - Argument names agree, with `app` excluded (119-137).
  - Event names, and `JobResults` in `commands.ts:696-…` against `service::kinds::ALL: [&str; 13]` (`ui/src-tauri/src/service/mod.rs:107-137`) (139-162).
  - Unit tests in `schema.rs:573-608` check that no two types share a name.
- **What must be regenerated when core types change.** Any change, including a change to a doc comment (descriptions are copied into both outputs), to any type reachable from the roots:
  - `core_model` types: EffectNode/EffectKind/Param/ParamValue/Keyframe/Interp/MaskRef/MaskShape/BlendMode/Transition/TransitionKind/Provenance/Rgba/CurvePoint.
  - `core_commands` Command/Transaction/ChangeSet, `core_edit` EditOp, `core_query` views, and the `error.rs` details.

  These rewrite `types.ts` and these schema files: `Command`, `Transaction`, `ClipView`, `TimelineView`, `EffectView`, `ScratchRequest`, `ChangeSet`, `EditOp`, `EditErrorDetail`, `AppError`, `DragFrame`, `DragUpdate` (checked by grep over `schemas/`).
- **Not generated, maintained by hand:**
  - `catalog.fixture.ts`. Only the end-to-end test catches drift.
  - The unit table of every `kind.param` → unit (`ui/src/features/inspector/controls.test.ts:73-128`) and the addable-kinds lists (59-60).
  - The command-line `vep schema` output, which is built at runtime (`crates/cli/src/main.rs:979-993`). Its test list is `crates/cli/tests/cli.rs:152-158`.

---

## 2. The inspector (`ui/src/features/inspector`)

### 2.1 What comes from the catalogue

- One section per effect: title, description tooltip, parameters (`EffectSection.tsx:36-92`).
- The control type per parameter kind (`controls.ts:252-273`), bounds and defaults (`numberParts` 189-210), enum choices (262), keyframable is parsed but unused.
- The "add effect" menu: `addableKinds` filters by `domain` against the track kind (`catalog.ts:113-115, 145-147`).
- Undo labels are made from `spec.title` and `param.title` (`ParamRow.tsx:242`; `EffectSection.tsx:48, 72-74, 117`).
- Test ids embed the kind and parameter: `param-${effect.kind}-${param.id}` (`ParamRow.tsx:252`), `add-effect-${spec.kind}` (`EffectSection.tsx:114`).
- Saved interface preferences embed them too: `inspector.collapsed.${spec.kind}` (`EffectSection.tsx:37`) and `inspector.linked.${testId}` (`ParamRow.tsx:36`). They are persisted in `Settings.ui` through `useUiValue` (`ui/src/state/prefs.ts:199-203`). Renaming kind ids would orphan these keys.

### 2.2 Effect knowledge held in the window, quoted

**Lists per kind** (`catalog.ts`):

```ts
export const GESTURE_ONLY_KINDS: readonly EffectKind[] = ['time_remap']                         // 123
export const STANDARD_KINDS: Record<TrackKind, readonly EffectKind[]> = {                        // 130-133
  video: ['transform', 'crop', 'color_matrix'],
  audio: ['audio_gain', 'audio_pan'],
}
/** The kinds the renderer and the mixer act on today. … (This list is the one thing here that is not in
 * the catalogue: see the panels engineer's request for a flag in `EffectSpec`.) */
export const DRAWN_KINDS: readonly EffectKind[] = ['transform', 'crop', 'composite', 'color_matrix', 'mask', 'audio_gain', 'audio_pan', 'time_remap'] // 142
```

- `GESTURE_ONLY_KINDS` is used in `commands.ts:27, 49, 75` and `catalog.ts:146`.
- `STANDARD_KINDS` drives "shown before the clip has it" sections and the insertion order: `insertIndex` (`commands.ts:44-54`), `sectionOrder` (70-81), `EffectSection.tsx:38, 134`.
- `DRAWN_KINDS` drives the "not drawn yet" chip (`EffectSection.tsx:60-66`) and the menu label suffix (113).
- `ui/src/app/README.md:129` lists this as a known gap.

**Units guessed from names and descriptions** (`controls.ts:8-10`: "Units are not a field of the catalogue"):

```ts
export function unitOf(param: ParamSpec): string {                                   // 58-73
  const id = param.id.toLowerCase()
  if (id.endsWith('_db')) return 'dB'
  if (id.endsWith('_ms')) return 'ms'
  if (id.endsWith('_freq')) return 'Hz'
  const text = param.description.toLowerCase()
  if (/\bpixels?\b/.test(text)) return 'px'
  if (/\bdegrees?\b/.test(text)) return '°'
  if (/\bstops?\b/.test(text)) return 'stops'
  …
  if (isFraction(param)) return '%'
  if (/\bmultiplier\b|\bfactors?\b|\bratio\b/.test(text)) return '×'
```

Related rules in `controls.ts`:
- `isFraction`: min 0 and max 1, or a description containing "normalized"/"fraction", is shown ×100 as % (79-83, 193).
- `FIELD_STEPS` by unit (217) and `fieldStep` (224-230).
- `sliderRange`: caps the slider at 4× the default's distance when the range is more than 10× (180-186).
- `linkMode`: "ratio" when the unit is ×, "offset" for any 3-number parameter (127-131).
- `PART_LABELS = { 1: [''], 2: ['X', 'Y'], 3: ['R', 'G', 'B'] }` (169). Every 3-number parameter is assumed to be a colour triple.
- Text controls become multi-line when the description mentions "source" or "code", aimed at shader source (265-267).
- `choiceLabel`: `soft_light` → "Soft light" (245-249).
- `describeValue` (401-413) and the shape editor (343-398) know `MaskShape` rect, ellipse and path. A path is read-only (`ParamRow.tsx:210`).

**Parameter kinds:** `ParamKind` is a fixed list in the window (`catalog.ts:14-16`). `ParamRow.tsx:270-288` switches over the control types.

**Stored values of the wrong kind** are shown as the default and never written back by just looking: `const stored = view && isParamValue(view.value, param.kind) ? view.value : param.default` (`ParamRow.tsx:240`). An enum value the catalogue no longer lists is still shown (88-89).

**Animated parameters** are read-only ("Animated, N keys" and "Make it one value") (`ParamRow.tsx:261-269`). There is no keyframe editing (`ui/src/app/README.md:75`).

**Add and update rules** (`commands.ts`):
- `paramCommands` (108-122) adds the missing effect with only the touched parameters: `raw.addEffect({kind:'clip',id}, id, kind, {param:{value}}, insertAt)` (119). Untouched parameters are left for the catalogue's defaults.
- Reset is `{ remove: [...] }` (125-128).
- Moving skips hidden kinds (158-166).

**An unknown kind already has a fallback** (`EffectSection.tsx:137-147`): "This version of the editor has no controls for this effect." No unit test covers it (`inspector.test.tsx` only tests time_remap and "not drawn").

### 2.3 Transforms, masks, blend, transitions and speed

- **Transform / Fit:**
  - The window sends only clip ids (`Sections.tsx:96, 130-141` → `ui/src/lib/edit.ts:59-71` → `fit_to_frame`).
  - The backend holds the Transform knowledge in `ui/src-tauri/src/service/framing.rs:32-33` (`const SCALE: &str = "scale"`) and 81-129. It finds `EffectKind::Transform` (99), refuses if the scale has keyframes (101-103), and otherwise updates the scale or adds a Transform "before everything but a speed change": `position(|effect| effect.kind != EffectKind::TimeRemap)` (112-115).
  - Tests: `ui/src-tauri/tests/edit.rs:317-382`.
- **Masks:** the `mask` effect kind is edited generically through its `shape` parameter. `EffectNode.mask` (`MaskRef`) has no UI. The builder `raw.setEffectMask` (`ui/src/lib/rawCommands.ts:127`) is never called.
- **Blend:** there is no separate blend menu. The Composite effect's `blend` enum parameter is a generic `Select`. `EffectNode.blend` / `set_effect_blend` (`rawCommands.ts:126`) and `EffectView.blend` are unused, as are `has_mask`, `provenance` and `kind_id` (grep over `ui/src`).
- **Transitions:**
  - `const TRANSITION_NAMES = { cross_dissolve: 'Cross dissolve', dip_to_black: …, zoom: 'Zoom' }` with `{custom}` falling back to its string (`Sections.tsx:26-30`).
  - The `TransitionRow` length editor commits `setClipTransition(clip.id, edge, { kind: transition.kind, duration: frameStartTime(next, view.frame_rate) })` (`Sections.tsx:49`). **Transition `params` are dropped**, and `TransitionView` does not carry them anyway.
- **Speed:**
  - The clip section has a Speed % field, a Reverse checkbox and a ripple checkbox, sending `edit({ type: 'set_clip_speed', … })` (`Sections.tsx:85, 98-100, 120-129`).
  - The ramp note appears when `clip.speed === null` (126).

---

## 3. The timeline and other features

| Place | What it knows |
|---|---|
| `ui/src/features/timeline/paint.ts:82-88` `speedBadge` | `null` → "ramp", `<1` → "%", `>1` → "×" |
| `paint.ts:90-100` `clipBadges` | `effect.kind !== 'time_remap' && effect.kind !== 'audio_time_stretch'` are counted as "fx". This does not match the inspector, which offers `audio_time_stretch` as an ordinary effect (`controls.test.ts:60`) |
| `paint.ts:184-232` `paintTransition` (called 315-316) | Wedge drawing. The label is `transition.kind === 'cross_dissolve' ? 'Dissolve' : 'Transition'` (225) |
| `timeline/actions.ts:228-243` | `isPicture` (video track and asset source) and `fitSelection` → `fitPictures` |
| `actions.ts:313-319` `setSpeed` | Builds a `set_clip_speed` `EditOp` |
| `actions.ts:325-335` | `DISSOLVE_LENGTHS` 1, 1/2, 1/4, 1/10 s; `dissolveLength` |
| `actions.ts:342-348` `addDissolve` | Hard-coded `{ kind: 'cross_dissolve', duration }` through `raw.setTransition` |
| `actions.ts:351-370` | `removeTransitions`, `toggleDissolveOnSelection` |
| `timeline/controller.ts:1720-1731` | Context menu: "Speed and reverse…", "Fit to frame", "Fill the frame", "Add a cross dissolve at …", "Remove the transition at …" |
| `controller.ts:893-899` | Slip preview uses `span.reversed` and `sourceRate` |
| `timeline/index.tsx:239-251` | Shortcuts `timeline.speed` (Mod+R) and `timeline.dissolve` (Mod+Shift+D) |
| `timeline/TimelineDialogs.tsx:12-52` | `SpeedForm`: percent, reverse, ripple → `setSpeed`; ramp note at 48 |
| `timeline/dialogs.ts:1-23` | `speed: ClipId[] \| null`, `openSpeed` |
| `timeline/content.ts:1-52` | Thumbnails and waveform use `source_range`, `duration`, `reversed` (time knowledge only) |
| `ui/src/lib/rawCommands.ts:114-128` | The only effect and transition command builders: `setTransition`, `addEffect`, `updateEffect`, `removeEffect`, `reorderEffect`, `setEffectEnabled`, `setEffectBlend`, `setEffectMask`, `setKeyframes` |
| `rawCommands.ts:279-309, 320-338` | Named helpers `addEffect`, `setEffectParams`, `previewEffectParams`, `setEffectEnabled`, `removeEffect`, `reorderEffect`, `resetEffectParams`, `setClipTransition` |
| `ui/src/lib/edit.ts:47-71` | `fitPictures` |
| `ui/src/app/FileDrop.tsx:30-35`, `app/routing.ts:8-17` | A dropped file is either `.vep` (open) or media (import). No route for a package file yet |
| `ui/src/features/preview/index.tsx:394` | Speed text refers to playback rate, not effects |
| `ui/src/state/*` | No effect knowledge, except saved preference keys that embed the kind (§2.1) |

---

## 4. The command-line tool (`crates/cli`, binary `vep`)

- **Dependencies:** `effects-registry` and `render` (`crates/cli/Cargo.toml`).
- **Effect subcommands** (`main.rs:353-355, 735-742`): `Add {path, clip, kind, sets}`, `Set {path, effect, sets}`, `Remove {path, effect}`, `List {path, clip}`. Handlers are at `main.rs:1244-1290`:
  - Add (1245-1259): `parse_effect_kind` → `EffectNode::new` → `parse_param_assignment` per `--set` → `Command::AddEffect`. The label uses `effects_registry::spec(kind).title` (1255).
  - Set (1260-1269): looks up the node's kind (1262), then `parse_param_assignment` → `UpdateEffect`.
  - Remove (1270-1274).
  - List (1275-1289): `core_query::effects_of`, printing `e.kind_id` and `param_text` (1300-1311).
- **Helpers** (`crates/cli/src/util.rs`):
  - `resolve_effect` (290-296).
  - `parse_effect_kind` (299-306) adds a `core.` prefix and calls `EffectKind::from_id`. The error lists `EffectKind::ALL`.
  - `parse_param_assignment` (310-318) checks against `effects_registry::spec(kind).param(key)`.
  - `coerce(ParamKind, json)` (333-~373).
- **Schema output:**
  - `vep schema <commands|transaction|effects|project|timeline-view|edit-op|edit-preview|change-set|export-settings|jobs>` (`main.rs:136-138, 374-387, 599-610`).
  - `effects` prints `effects_registry::describe()` (983), the same JSON the window gets.
  - The others are `schemars::schema_for!` at runtime (981-985) or mirrors in `crates/cli/src/schemas.rs:384-404`.
  - Tests: `crates/cli/tests/cli.rs:87-92` (effect add/set/list/out-of-range) and 152-158 (every schema kind).
- **Other effect-aware spots:**
  - `demo` builds a Blur node and a 2× time-remap node (`main.rs:1354-1360`).
  - `explain.rs:37-43` has plain-English texts for `TransitionInvalid`, `EffectDomainMismatch`, `KeyframesUnsorted`, `KeyframeKindMismatch`, `ParamNotFinite`, `MaskInvalid` and `TimeRemapMisplaced`. The match has deliberately no catch-all (15-16), so a new "missing block" violation needs a text here.
  - `edit.rs:213-216, 251-252` mention `set_clip_speed` in help.
  - `RenderFrame` creates its own `render::Renderer::new()` (`main.rs:1027`).
  - The config folder logic is a duplicate (`export_cmd.rs:91-103`).

---

## 5. Per-user folders, and where block folders would fit

- **Config folder:**
  - Rules: `VEP_CONFIG_DIR` if set and not empty; otherwise Windows `%APPDATA%\vep`, macOS `~/Library/Application Support/vep`, elsewhere `$XDG_CONFIG_HOME/vep` or `~/.config/vep` (`ui/src-tauri/src/settings.rs:1-13`, `config_dir_from` 280-294, `default_config_dir` 297-299).
  - It holds `settings.json` and `encoders.json` (23-25). `SettingsStore::load/dir` are at 335-360.
  - The command-line copy is `crates/cli/src/export_cmd.rs:91-103` (encoders file at 105-109).
  - The folder is shown in `AppInfo.config_dir` (`contract.rs:94-117`; filled in `service/mod.rs:838-865`) and in the Settings dialog (`ui/src/features/settings/index.tsx:305-307`).
- **Cache folder:** `VEP_CACHE_DIR`, otherwise Windows `%LOCALAPPDATA%\vep\cache`, macOS `~/Library/Caches/vep`, Unix `$XDG_CACHE_HOME/vep` or `~/.cache/vep` (`crates/cache/src/layout.rs:14, 83-91`; `Cache::default_dir` at `crates/cache/src/store.rs:539-541`).
- **How the app is configured** (`AppConfig`, `ui/src-tauri/src/service/mod.rs:151-182`):
  - `from_env()` is used by the real app at `lib.rs:180`, **before and without the Tauri path resolver**.
  - `isolated(config_dir, cache_dir)` is used by tests: `ui/src-tauri/tests/common/mod.rs:69-75` (`dir/config`, `dir/cache`) and `ipc.rs:31-33`.
  - The end-to-end harness sets `VEP_CONFIG_DIR`/`VEP_CACHE_DIR` to temporary folders (`ui/e2e/harness/app.ts:26-42, 199-204, 767-768`).
  - `VEP_FFMPEG_DIR` (`crates/media/src/tools.rs:104-106`) is the precedent for a folder override through an environment variable.
- **Tauri setup:** `tauri.conf.json` has `identifier: dev.vep.editor`, no `bundle.resources`, and `security.csp: null`. Capabilities are `core:default`, `core:window:allow-set-title`, `dialog:default` (`ui/src-tauri/capabilities/default.json`). Note that Tauri's own `app_config_dir()` would be `…\dev.vep.editor`, not `…\vep`, so the app's own `vep` folder rule should be kept.

**User blocks folder.** The natural spot is `<config_dir>/blocks/`. On Windows that is the roaming `%APPDATA%\vep\blocks`. It inherits `VEP_CONFIG_DIR` isolation for free in backend tests, end-to-end runs and the command-line tool, if the command-line copy is shared. Derived artefacts (validated or compiled modules, shader pipeline caches) fit the cache's recipe and key model in `%LOCALAPPDATA%\vep\cache`.

**Built-in blocks shipped with the app (Tauri resources).** I checked this against the installed Tauri sources:
- `bundle.resources` (list or map form) is copied by `tauri-build` into `target/<profile>/` at **every build** (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-build-2.7.1/src/lib.rs:744-762`, `copy_resources` 171-190).
- `PathResolver::resource_dir()` returns the **exe's own folder on Windows**, in development and in release alike (`tauri-utils-2.10.1/src/platform.rs:289-310`). On macOS it is `../Resources` and on Linux `../lib/<name>` (312-343).

Consequences:
- **The real app:** it would have to call `app.path().resource_dir()` inside the `setup` closure (`ui/src-tauri/src/lib.rs:178-189`) and pass the result into `AppConfig`, which today has no such field.
- **`cargo test`:** test binaries run from `target/debug/deps`, so `resource_dir` would point there. Tests need an explicit path, for example through `AppConfig::isolated` or the repository path.
- **The command-line tool:** it has no Tauri. It needs its own lookup, such as an env var, `<exe dir>/blocks` (which works in `target/debug` after a `vep-app` build), or the repository path.
- **End-to-end runs:** the harness copies only `vep-app.exe` into `<tmp>/bin` (`app.ts:750-754`), so `<exe dir>/blocks` would be empty. The harness would also have to copy the folder, or the app would need an env override, which can be passed with `LaunchOptions.env` (`app.ts:50-51, 756-771`).
- **An alternative that avoids all path issues:** embed the built-in packages in the binary and load them through the same package reader.

---

## 6. The end-to-end harness: install a package, use the block, export, check the result

**Launch.** `npm run e2e` (`ui/e2e/vitest.config.ts`) runs a global setup that builds the debug app with `cargo build -p vep-app` (`harness/build.ts:54-62`) and starts the Vite dev server on 127.0.0.1:5173 (`build.ts:131-176`). `launchApp` (`app.ts:741-817`) then:
- copies the exe (750-754);
- starts it with a temporary config, cache and webview folder plus any extra `env` (757-771);
- connects over DevTools (playwright-core) and waits for `window.__VEP_TEST__.ready`;
- sets `dialogs.strict = true` (804-807).

Test hooks exist **only in development builds** (`ui/src/lib/testHooks.ts:108-115`).

**Native file dialogs are hooked like this:**
- Every picker goes through `pickOpen`/`pickSave` (`ui/src/lib/dialogs.ts:68-83`). The source test `ui/src/architecture.test.ts:42-44` enforces that `@tauri-apps/plugin-dialog` is imported only there.
- `testAnswer` (`dialogs.ts:46-61`) pops a queued answer from `window.__VEP_TEST__.dialogs.queue`, or throws `UnansweredDialogError` in strict mode.
- A test queues the answer with `app.answerDialog('open', path)` **before** the click, and can read `dialogLog()` (`app.ts:486-495`).
- So an "Install block…" button that calls a new `pickBlockPackage()` built on `pickOpen` would work as is.

**Other ways to install a package in a test:**
1. **Direct backend call:** `app.invoke('<install command>', { path })` (`app.ts:280-293`) uses `window.__TAURI_INTERNALS__.invoke` exactly as the window does.
2. **Simulated file drop:** `dropFiles(app, paths)` (`ui/e2e/panelsHelpers.ts:122-138`) emits `tauri://drag-enter`/`drag-drop` through `plugin:event|emit`. Routing would need a package extension in `ui/src/app/routing.ts:8-17` and `ui/src/app/FileDrop.tsx:30-35`.
3. **Pre-seeding:** write into `app.dirs.config` (VEP_CONFIG_DIR) and call `app.restart()`, which keeps the folders (`app.ts:712-715`), or pass `launchApp({ dirs })`.

**Using the block in a test.**
- Open the inspector with `showTab(app, 'inspector')` (`panelsHelpers.ts:140-143`) and the add menu through `inspector-add-effect-video|audio` (`EffectSection.tsx:130`). The menu item test id is `add-effect-<kind>` (114).
- Parameters are `param-<kind>-<param>[-slider|-x|-y|-reset]`, driven with `typeNumber` and `dragSlider` (`panelsHelpers.ts:77-111`).
- Read back with `effectsOf(app, 'V1')` (`panelsHelpers.ts:166-170`). Check the live picture with `picture`/`waitForPicture` (25-56).

**Exporting and checking the result.**
- **Through the dialog:** `answerDialog('save', out)` → `click('export-choose')` → `click('export-start')`, then wait for the job (`ui/e2e/panels-export.e2e.ts:107-121`). Or directly with `invoke('export_start', { request })`.
- **Checking:** `probeFile` (`harness/media.ts:46`) for streams. Pixel checks can use FFmpeg `rawvideo`/`gray` reads (`ui/e2e/dogfoodHelpers.ts:132-134`) or a still frame from `export-still` to a PNG (`panels-export.e2e.ts:258-267`).
- **Test media:** `generateClip`, `generateTone`, `generateImage` (`harness/media.ts:75, 93, 104`) for known inputs, such as a flat colour or a pure tone to measure gain.

**Existing end-to-end tests that assume today's kinds:**
- `panels-inspector.e2e.ts:69-146`: catalogue equality, the 5 standard sections, Transform opacity.
- `panels-review.e2e.ts:50-235`: transform and crop.
- `panels-walkthrough.e2e.ts:137-190`: dissolve, audio_gain, color_matrix.
- `dogfood.e2e.ts:396-545`: speed, cross dissolves, transform, audio_gain.
- `timeline-many.e2e.ts:232-246`, `timeline-ui.e2e.ts:271-313` and `timeline-review.e2e.ts:774-776`: speed and dissolves.

---

## 7. Inventory of effect-specific places (outside core-*, render, audio, export)

| File:line | What it knows |
|---|---|
| `crates/effects-registry/src/lib.rs:1-5` | Doc: the UI builds the inspector from it, the AI gets it with skill schemas |
| `effects-registry/src/lib.rs:16-39` | `ParamSpec`/`EffectSpec`: Serialize only, no JsonSchema |
| `effects-registry/src/lib.rs:72-130` | Constructors per parameter kind (float/bool keyframable defaults etc.) |
| `effects-registry/src/lib.rs:132-240` | The hand-written catalogue of all 16 kinds (ids, titles, descriptions, ranges) |
| `effects-registry/src/lib.rs:242-251` | `static REGISTRY: OnceLock`; `spec()` panics for an unknown kind |
| `effects-registry/src/lib.rs:253-380` | Defaults, value, keyframe, mask and node checks |
| `effects-registry/src/lib.rs:382-384` | `describe()`, the catalogue JSON |
| `ui/src-tauri/src/commands.rs:131-134` | `effects_catalog`, no state, static registry |
| `ui/src-tauri/src/commands.rs:153-156` | `fit_to_frame` |
| `ui/src-tauri/src/lib.rs:112,117,127-128` | Effect, fit and scratch commands registered |
| `ui/src-tauri/src/lib.rs:178-189` | Service started with `AppConfig::from_env()`; no resource dir |
| `ui/src-tauri/src/service/framing.rs:1-19,32-33,81-129,143-156` | Transform `scale`, keyframe refusal, insert before TimeRemap |
| `ui/src-tauri/src/service/edit.rs:376-414` | Scratch overlay for effect-parameter previews |
| `ui/src-tauri/src/service/project.rs:293-300` | Open report violations (effect problems as text) |
| `ui/src-tauri/src/contract.rs:253-268, 437-443` | `OpenReport.violations`; `ScratchRequest` |
| `ui/src-tauri/src/error.rs:192-195,228-229,242-257` | Speed errors; `CommandErrorDetail` (ParamError nested) |
| `ui/src-tauri/src/events.rs:134-166` | No catalogue event; fixed `NAMES: [&str; 9]` |
| `ui/src-tauri/src/schema.rs:93-107` | Engine roots incl. `EffectView` (104); catalogue absent |
| `ui/src-tauri/src/player.rs:655,692,848` | Mixer, renderer, `Renderer::new()` (registry plumbing point) |
| `ui/src-tauri/src/service/exporting.rs:236,280` | `export::export_sequence` / `export_still` (registry plumbing point) |
| `ui/src-tauri/src/settings.rs:280-299` | Config folder rules (where `blocks/` would go) |
| `ui/src-tauri/src/service/mod.rs:151-182` | `AppConfig` (no blocks fields) |
| `ui/src-tauri/tauri.conf.json` | No `bundle.resources`; `csp: null` |
| `ui/src-tauri/tests/bindings.rs:25-44,139-162` | Staleness test; gesture list incl. `set_clip_speed`; event and job lists |
| `ui/src-tauri/tests/ipc.rs:152` | `effects_catalog` shape check |
| `ui/src-tauri/tests/edit.rs:317-382` | Fit-to-frame Transform tests |
| `ui/src/bindings/types.ts:313,519-534,536-557,634-663,859-931,1477-1490,1770-1792,1846-1892,1989-2011,2396-2405,2548-2559,2757-2764,2803-2840,3075-3150,3394-3429,3572-3580,3847-3860,4276-4361` | Generated effect, transition, speed and blend types (closed `EffectKind`, `TransitionKind`, `BlendMode`) |
| `schemas/Command.schema.json:117,358-379,1825-1849` (and Transaction/ClipView/TimelineView/EffectView/ScratchRequest/EditOp) | Closed lists in the files the AI is to be given |
| `ui/src/bindings/commands.ts:297-300` | `effectsCatalog(): Promise<unknown>` |
| `ui/src/bindings/commands.ts:306-366,466-496` | Doc text naming speed, effect parameters, Transform |
| `ui/src/features/inspector/catalog.ts:13-16` | Fixed `ParamKind` list |
| `catalog.ts:90,92` | Domain defaults to video; kind cast |
| `catalog.ts:117-147` | `GESTURE_ONLY_KINDS`, `STANDARD_KINDS`, `DRAWN_KINDS`, `addableKinds` |
| `ui/src/features/inspector/catalog.fixture.ts` | Hand copy of the catalogue (all kinds) |
| `ui/src/features/inspector/controls.ts:57-83,127-131,169,180-186,217-230,245-249,265-267,343-413` | Unit guessing, % rule, link rule, R/G/B rule, slider cap, steps, enum labels, source-code text, shape editor |
| `ui/src/features/inspector/commands.ts:26-28,44-54,70-81,108-122,158-166` | Hidden time_remap, standard order, add-with-params, move |
| `ui/src/features/inspector/EffectSection.tsx:37,60-66,113-114,127,134-147` | Preference key per kind, "not drawn yet", menu, Picture/Sound, unknown-kind fallback |
| `ui/src/features/inspector/ParamRow.tsx:36,88-89,98,202-235,240,242,252,261-269` | Preference key, keep unknown enum value, default colour, shapes, wrong-kind guard, labels, test ids, animated read-only |
| `ui/src/features/inspector/Sections.tsx:26-57,85,98-100,120-141,167-174` | Transition names, length edit drops params, speed, reverse, ripple, Fit/Fill |
| `ui/src/features/inspector/live.ts:1-3,13-44` | Catalogue store, fetched once |
| `ui/src/features/inspector/index.tsx:3-14,40,62-63,77-90` | Picture/sound grouping, catalogue loading |
| `ui/src/features/inspector/*.test.*` (`controls.test.ts:54-128`, `inspector.test.tsx:146-154,356-401`, `commands.test.ts:89-101`) | Units per kind.param, addable lists, time_remap hidden |
| `ui/src/features/timeline/paint.ts:82-100,184-232,315-316,333` | Speed badge, fx count excluding time_remap and audio_time_stretch, Dissolve label |
| `ui/src/features/timeline/actions.ts:228-243,313-370` | Fit selection, setSpeed, dissolve lengths, cross_dissolve |
| `ui/src/features/timeline/controller.ts:893-899,1720-1731` | Reversed slip preview; speed, fit and dissolve menu |
| `ui/src/features/timeline/index.tsx:239-251` | Speed and dissolve shortcuts |
| `ui/src/features/timeline/TimelineDialogs.tsx:12-52`, `dialogs.ts:1-23` | Speed dialog |
| `ui/src/features/timeline/content.ts:1-52` | Speed and reverse for thumbnails |
| `ui/src/lib/rawCommands.ts:114-128,279-338` | All effect and transition command builders and helpers |
| `ui/src/lib/edit.ts:47-71` | `fitPictures` |
| `ui/src/app/FileDrop.tsx:30-35`, `ui/src/app/routing.ts:8-17` | Drop routing (no package type) |
| `ui/src/architecture.test.ts:46-55` | Features cannot share the catalogue store |
| `ui/src/app/README.md:57-75,129` | Inspector rules; "catalogue has no units / DRAWN_KINDS" |
| `crates/cli/src/main.rs:136-138,353-355,374-387,599-610,735-742,979-993,1027,1244-1311,1354-1360` | Schema and effect subcommands, renderer, param text, demo |
| `crates/cli/src/util.rs:290-318,333-373` | Kind parsing (closed, `core.` prefix), parameter parsing through the registry |
| `crates/cli/src/explain.rs:15-16,37-43` | Effect violation texts, exhaustive match |
| `crates/cli/src/edit.rs:213-216,251-252` | `set_clip_speed` help |
| `crates/cli/src/export_cmd.rs:91-109` | Duplicate config-folder rules |
| `crates/cli/tests/cli.rs:87-92,152-158,216` | Effect subcommand and schema list tests |
| `ui/e2e/harness/app.ts:26-42,280-293,486-495,712-715,741-817` | Folders, invoke, dialog queue, restart, launch (exe-only copy) |
| `ui/e2e/panelsHelpers.ts:122-138,166-170` | `dropFiles`, `effectsOf` |
| `ui/e2e/panels-inspector.e2e.ts:11,69-146`; `panels-review.e2e.ts:50-235`; `panels-walkthrough.e2e.ts:137-190`; `dogfood.e2e.ts:396-545`; `timeline-many.e2e.ts:232-246`; `timeline-ui.e2e.ts:271-313`; `timeline-review.e2e.ts:774-776` | End-to-end tests that assume today's kinds, transitions and speed |

Crates `playback`, `cache`, `jobs`, `media` and `ingest` have no effect knowledge (grep for `EffectKind`, `time_remap`, `transition`, `BlendMode` and `.effects` found nothing). `playback` only receives source times from the renderer (`crates/playback/src/frames.rs:1535`).