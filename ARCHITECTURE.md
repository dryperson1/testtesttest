# AI-Native Video Editor: System Architecture

Status: founding design, v0.1
Scope: desktop editor (Windows/macOS/Linux) with a small Rust core, modular skills, and a hierarchical AI analysis/planning pipeline.

This document is opinionated on purpose. Where it disagrees with the original brief it says so and explains why. Section 17 lists the decisions that are expensive to reverse; everything else can evolve.

---

## 0. Executive summary of decisions

| Area | Decision |
|---|---|
| Core | One Rust library crate family (`core-*`) with **no UI, no AI, no FFmpeg** dependencies. Pure data model + commands + undo. Headless and fully testable. |
| State model | **Immutable persistent project state** (structural sharing). A command is a pure function `(State, Command) -> State`. Undo = keep previous state pointer. Renderer and UI read snapshots. |
| Time | **Rational time** (`i64/i64`) everywhere in the model. Never floats. Never a fixed tick rate. |
| Effects | The project stores **only renderer primitives** (Transform, Crop, Blur, LUT, TimeRemap, Mask...). Higher-level operations are skills that *expand* into primitives and leave **provenance** on the nodes so the UI can re-edit them through the skill. |
| Skills | Rust trait objects returning **Transactions** (batches of Commands). Never mutate state. Same path for manual UI and AI. Schemas generated from Rust types, not hand-written. |
| Processes | 1) Tauri app process (UI + core + render + playback + orchestrator). 2) Python **model service** (analysis + specialized CV + VLM proxy), gRPC over localhost. 3) Ollama (or llama.cpp/vLLM) as a managed external server. |
| Python's role | **Model execution only.** Not orchestration, not ingest, not project state. "If it does not need PyTorch, it is not Python." |
| Ingest | Probing, shot detection, frame sampling, audio extraction, thumbnails, waveforms, cheap signals: **Rust** via FFmpeg, not Python. |
| Fast judge | Embedding model + constrained scoring behind a `Judge` interface; first implementation SigLIP-2/V-JEPA-2 zero-shot in Python; swappable to ONNX-in-Rust later. Never routed through Ollama. |
| Persistence | Project file = **SQLite** database. Timeline document stored as a versioned blob; assets, analysis, jobs, transactions as relational tables. Large derived data in a **content-addressed cache** outside the project. |
| UI | React + TypeScript in Tauri v2 webview. Timeline on `<canvas>`. Preview via a **native wgpu surface**, not pixels pushed through the webview. |
| Models | Existing models everywhere. The only "training" we ever do is tiny heads on frozen embeddings from user feedback. |

### Phase 0 implementation notes (2026-10-01)

The headless core (sections 2 crates `core-*`, `effects-registry`, `cli`) is implemented and tested. Where the code deliberately deviates from the text below, the code wins:

- The project document is stored as **canonical JSON**, not CBOR. Migrations operate on a plain JSON tree, exports are diffable, and the size difference is irrelevant at project scale. CBOR or compression can be added behind `core-model::codec` later.
- **Slip and slide are their own commands** (`SlipClip`, `SlideClip`) rather than `TrimClip` modes, because they take a delta, not an edge position. `TrimMode` is `Normal | Ripple | Roll`.
- There is no `PutBlob` command. Blobs are **content-addressed** through `core-project::Project::put_blob`; the state only holds keys, so blobs are immutable and need no undo.
- `Param.keyframes` is a (possibly empty) vector, not an `Option`.
- Markers are kept sorted by time; `ReplaceMarker` replaces a whole marker instead of a field-wise update.
- The journal table is named `journal` and also records `undo` and `redo` operations, so undo history survives a crash until the next checkpoint. The journal row is written **before** the in-memory state changes, so a failed disk write never leaves memory ahead of the file. An undo or redo that reaches past the current checkpoint is persisted as a new checkpoint instead (the journal cannot express it).
- Every write verifies that no other handle has written to the file since this one last did; a second app instance or a parallel CLI run gets a clear "modified by another instance" error instead of corrupting the journal.
- Time remaps are clip-only and one per clip (anywhere else they would be inert); any change to a clip's effects re-checks that the clip still reads inside its source media.
- Non-finite numbers (NaN, infinity) are rejected at every command boundary, because JSON cannot represent them and a single one would make the project file unreadable.
- Audio clips snap to the sequence's audio sample grid; video clips snap to the frame grid (`Sequence::grid_for`).
- Reversed clips: a head trim leaves `source_in` unchanged and a tail trim moves it, because a reversed clip shows the end of its source range first.
- See `docs/LOCAL-FREE-DECISIONS.md` for how the open questions in section 21 were resolved under a "free and local only" constraint.

### Phase 1 implementation notes (2026-10-02)

Media, rendering, audio, playback and the desktop shell exist; deviations from the text below:

- **FFmpeg is driven as child processes** (`ffprobe` for probing and the packet index, `ffmpeg` streaming raw RGBA/f32 frames over a pipe) rather than linked. Playback keeps one decoder open per asset stream and reads sequentially, reopening only on jumps; audio is mixed a quarter second ahead per chunk. In-process decoding can replace this behind the same `media` types when it is worth the build complexity.
- **The renderer composites in linear light into an fp16 target and encodes to sRGB on output**, as planned. Phase 1 primitives: Transform, Crop, Composite (normal), ColorMatrix, Mask (rect/ellipse), cross-dissolves, solid generators, sequence-level colour. Asset layers keep their native pixel size even when decoded smaller for preview.
- **Preview is readback-to-canvas**: the player thread renders at preview width, the window polls the player status and fetches the latest frame as raw bytes over Tauri IPC.
- **The audio output's consumed-frame counter is the playback clock** when a device exists; otherwise wall-clock time.
- The Tauri crate lives at `ui/src-tauri` (the standard layout, so the Tauri tooling works unmodified) and is a workspace member.

### Phase 2 implementation notes (2026-10-08)

Phase 2 added the crates `core-edit`, `jobs`, `cache`, `ingest` and `export`, an encode module in `media`, and a second hardening pass over the core, media, render, playback and audio. Each crate's `src/lib.rs` documentation states its guarantees precisely; this list records the decisions that differ from, or go beyond, the text below.

**Editing**

- **Gestures live in `core-edit`.** A gesture ("move these clips", "ripple-trim this edge", "split under the playhead") is one `EditOp`; `expand` turns it into an exact transaction, including the clips linked to the ones named, and `preview` returns what a drag in progress would do without committing. The window, the command-line tool and later the AI planner all send `EditOp`s. The window contains no editing arithmetic.
- **Sync lock is the default.** Every rippling gesture shifts the later clips of all unlocked tracks by the same distance, so picture, sound, music and titles stay in step. `ripple_scope: own_tracks` restores track-local rippling. Locked tracks never move; a closing ripple closes only by the distance every track can give. Two engine commands, `ShiftClips` and `ShiftMarkers`, exist for this.
- **Grids and linked clips.** Picture sits on the frame grid and sound on the sample grid. A gesture involving picture is rounded to whole frames; linked sound follows in whole samples chosen to keep the pair in sync. At frame rates where a frame is not a whole number of samples (29.97, 59.94) pairs can differ by a fraction of a sample per gesture; this is far below anything audible and is documented with measurements in the crate.
- **Speed-changed clips keep their content when cut.** A clip's source in-point is stored as an exact fraction instead of being rounded to a source frame, so splitting or trimming a slowed or reversed clip never changes what any remaining frame shows. This changed the meaning of stored in-points: the project document is now **schema version 2**, and version 1 files are migrated on opening.
- **Limits at the command boundary.** Timeline times beyond one billion seconds and frame-rate and sample-rate pairs with no usable common grid are refused, so downstream arithmetic on an accepted project cannot overflow. Checking or opening a damaged document returns violations or an error and never crashes.
- **Decimals are read back exactly.** `serde_json` is built with `float_roundtrip` for the whole workspace; without it a saved parameter could reload one digit off and break journal replay.

**Jobs and cache**

- **`jobs` is plain threads and channels**, with no async runtime. Differences from section 14: only the oldest job of each priority class ages; a dedupe key is released when its job ends (whether an artifact already exists is the cache's question, asked before submitting); resources are claimed GPU first, then CPU; a foreground reserve keeps interactive work responsive; a job whose dependency did not succeed ends as failed, naming the dependency; a queued dependency inherits the urgency of whatever waits for it.
- **`cache` layout** is `cache.sqlite`, `artifacts/<aa>/<bb>/<key>.<ext>` and `tmp/` only (no separate proxies, models or frames folders). A key is BLAKE3 over a length-prefixed, canonically ordered encoding of the recipe. Producers write into a reserved temp path and commit atomically; eviction is least recently used under a byte budget and skips pinned or open files.
- **Facts about one computer stay out of the project.** Whether a media file is reachable, whether a proxy exists and how far a job has got are runtime facts kept by the app and the cache. Undo restores whole snapshots, so storing them in the document would bring stale values back.

**Media**

- **Source time zero is the start of the file** (the earliest picture or sound stream start) in every container, for video decode, audio decode, the packet index and thumbnails alike. A sound stream that starts later than its file is delivered with silence in front.
- **Every seek is verified.** Decoding at a time is frame-exact in every container that can be generated here, including MPEG transport streams and variable frame rate; a seek that lands wrong is retried from an earlier keyframe. The packet index lists the frames a decoder actually shows.
- **Colour.** Scaling uses full chroma interpolation and exact rounding. Footage with no colour tags is read as BT.709 from 720 lines (or 1280 pixels wide) and as BT.601 below. Thumbnails, posters, proxies and exports all use the same reading, so every view of a clip shows the same colours.
- **Encoders are trusted only after a real test encode.** Being listed by FFmpeg does not mean usable (the NVIDIA encoder is listed on machines whose driver is too old for it). `media::EncoderCatalog` test-encodes each candidate once, remembers the result, and records why an encoder was rejected.
- **Non-square pixels** are applied in one place, the displayed size of a stream, used by the renderer, thumbnails, posters and proxies.

**Derived media and export**

- **`ingest`** makes a thumbnail strip (one JPEG sprite sheet plus a JSON description with the exact source time of every tile), a poster, a multi-level waveform peak file, and proxies. A proxy is picture only, keeps the source's exact frame timing (verified after encoding, or the proxy is discarded), and is never read by an export.
- **`export`** renders every frame through the preview's renderer at full size and pipes it to FFmpeg with the offline mix. The output is frame-exact, written under a temporary name and renamed only on success, tagged BT.709, and falls back to the next working encoder if one fails at the start. Missing or damaged media does not stop an export but is itemised in the report.

**Renderer, playback, audio**

- **All renderers share one graphics device.** Creating and dropping renderers from several threads at once used to crash the process; it is now safe and cheap, so the preview and each export simply create their own.
- **Playback keeps several decoders per file** (a dissolve between two parts of one recording no longer restarts FFmpeg every frame), plays proxies when the preview size fits inside them, reads reversed clips in forward blocks, holds the last frame when a clip runs slightly past its media, and reports decode failures instead of silently dropping a layer.
- **Audio has a streaming source** (one decoder per clip that reads on) used by the player and by export, and reversed constant-speed clips are audible. Speed ramps are still silent, and changing speed changes pitch.

**Desktop app**

- **The backend is an application service that runs without a window** (`ui/src-tauri/src/service`); the Tauri commands are thin wrappers over it, so its behaviour is tested headless. All slow work (import, posters, thumbnails, waveforms, proxies, export, missing-file scans) goes through one `jobs::Scheduler`, and every command returns in milliseconds or returns a job id.
- **Push, not poll, for project state.** Every change emits one `project:changed` event with a rising revision number; jobs, media state, derived files and settings have their own events. The preview frame and the transport position are still polled.
- **A drag is begin, update, end.** Each mouse move is one round trip that snaps, previews and shows the result in the preview monitor through a scratch state that never reaches the journal or the undo history. Releasing commits the same gesture; Escape drops it.
- **The typed contract is generated.** Every type crossing the boundary has a JSON Schema in `schemas/`, and `ui/src/bindings/types.ts` is generated from those schemas by the app's own generator (`npm run bindings`). `commands.ts` and `events.ts` are hand-written, and tests fail when any of the three drifts from the Rust code. Types from `jobs`, `export`, `media` and `ingest`, which do not derive schemas, are mirrored in `wire.rs` with compile-time checks.
- **A damaged journal is never discarded silently.** Opening fails with the reason, and only an explicit second request opens the last checkpoint, after a backup copy of the file is made.
- **Fit to frame and fill** are an app-level command for now (`service/framing.rs`), not a `core-edit` gesture, so the command-line tool does not have them yet.
- **In and out marks live in the window only** and are not saved with the project.
- **End-to-end tests drive the real app** through the WebView2 debugging port with playwright-core, each run in isolated config, cache and browser-profile folders (`ui/e2e`). The "dogfood" scenario cuts a 21-cut film from generated recordings that carry a frame counter and a tick, exports it, and checks every frame and the sync of the result.
- **Measured on the development machine** (debug build): start-up about 0.7 s; 30 s of 1080p30 playback with no dropped frames, though 3 to 5% of frames reach the screen one refresh late because the window polls for them; export about 100 frames per second for H.264 at 1080p; with 2,000 clips a drag answers in about 2 ms (16 ms for a ripple trim) and the window holds 60 frames per second.

**Build**

- The app links the C runtime statically only in release builds. Tauri's default of doing so in every build leaves a stand-in library on the linker's search path that breaks the documentation examples of other crates in a workspace-wide test run.
- The development server listens on 127.0.0.1 only.

### Plug-in blocks implementation notes (2026-11-06)

From 2026-10-28 to 2026-11-06 every effect, transition, blend mode and generator became a plug-in **block**, which anyone can write and install into a built editor. The owner's brief is `docs/blocks-rewrite/BRIEF.md`, the approved design `docs/blocks-rewrite/DESIGN-APPROVED.md`, and the binding technical contract `docs/blocks-rewrite/BLOCKS-SPEC.md` ("spec" below; its sections 17 to 25 override earlier ones). Each stage's reports are in `docs/blocks-rewrite/stages`. The guide for people who write blocks is [BLOCKS.md](BLOCKS.md). These notes say where things are and why; the spec holds the rules. Where they differ from sections 0, 2, 11 and 12 below (a closed `EffectKind` list, an `effects-registry` crate, "renderer primitives", a `Shader` escape hatch), these notes win. They were written during Stage 6, before its reviews and acceptance check.

**The line between the core and blocks** (the brief's rule 6). The core decides when and where on the timeline things happen: exact time, clip placement, the time remap (speed and reverse), transition timing, the stack order, compositing and mixing, undo and the journal. A block decides what happens to the picture or the sound at one moment the core gives it. The decision and its reasons are written out for authors in BLOCKS.md, section 10.

**The model and the engine** (spec 9, 11, 18, 21)

- An effect node is a block id, the settings format it follows (`version`) and its settings. `EffectKind`, `TransitionKind` and `GeneratorKind` are gone from the live model, and the `effects-registry` crate is deleted (read it at tag `baseline-before-blocks`). A generator is a clip source holding a node (`ClipSource::Block`).
- The time remap stays a core node, `core.time_remap`, at the same position in a clip's list, because saved journals find nodes partly by position (spec 9.2).
- The project document is schema 3. Schema-2 files are migrated on reading, on the JSON tree, with the old kinds mapped to built-in blocks and a neutral colour node appended where today's arithmetic needs it for byte-exact pictures (owner decision D1); old journal rows are replayed through the same migration (spec 9.4, 9.5, 18, 21).
- `core-commands` checks structure only and knows no block, so replaying a journal never depends on which blocks are installed. Every change is checked against the catalogue at the edges, the app service and `vep`, by `blocks::check_transaction`, before it is committed (spec 9.3, 11, 21).
- `core-query`'s views carry each node's block, status and converted settings, and the AI's text of a timeline names what cannot be used (spec 13, 25).

**`crates/blocks`** (spec 2 to 8, 10, 12, 19, 22 to 25)

- The manifest's types, its rules (`check.rs`, each a stable rule code with a plain-English message) and the JSON Schema published for authors (`blocks/block-manifest.schema.json`, generated, with a staleness test).
- The package reader: a folder, a `.chadfx-block` zip read in memory with every limit, or a picture block in one `.wgsl` file (`lone.rs`, spec 24). A package's identity is the BLAKE3 hash of its canonical archive.
- The code checks. Picture code: naga 30 with every validation flag and no capabilities, the rules the software renderer needs mirrored from `crates/cpu-render` (`wgsl/portable.rs`, with a parity test), a static cost per stage, a loop budget inserted into every loop, and a composer that joins a stack of blocks into one module per draw (spec 7, 19, 20, 22). Sound code: wasmi 2.0 with `portable-dispatch`, no imports, deterministic fuel per call, memory limits, and a trial run on silence at install (spec 8, 19, 23).
- The registry: the built-in folder and the user's folder, install, replace, remove, turned-off and quarantined blocks in `state.json`, the crash record `in-flight.json`, every write atomic. It produces immutable catalogue snapshots; `CatalogSlot` is the one place the renderer, the mixer and the export read the current one from (spec 12, 25). `RegistryLink` carries quarantines and runtime problems back from whoever runs blocks (spec 22).
- The edges: `edges.rs` judges every node where it sits (`judge`: the catalogue's status, then the place, then whether its settings convert, a mistyped shape included) and converts older settings on edit (`migrate_on_edit`); `describe.rs` is the catalogue as data for the window and the AI (`blocks_catalog`).

**The renderer** (spec 7, 20, 22). `crates/render` names no block. It plans each layer from the catalogue and draws a stack in one fused draw, adding draws only for pass stages, blend blocks and mix transitions. Programs are compiled ahead and cached. Blocks the user installed run under the protections of spec 22: the crash record before each submission and compile, quarantine after a crash or a lost device, a slow-frame tripwire judged on each block's own timed share of a frame, a 2-second compile limit, no drawing on OpenGL or without a registry link. A stack too large for one draw (more than 64 blocks, more code, or more settings than the card's uniform binding holds: since Stage 6 the cap is the smaller of 64 KiB and the device's own limit) loses the blocks that do not fit, never the frame. `crates/render-recipe` describes one frame as plain data, which both the graphics card and `crates/cpu-render`, a software renderer used only by tests, can draw (spec 20).

**The mixer** (spec 8, 23). `crates/audio` names no block. A mixer session reads the catalogue at every call and runs every sound node in 128-frame slices on absolute sample positions, with each setting's values at a slice's start and end, so keyframed volume and pan are smooth ramps (owner decision D2), and preview and export give the same samples. Blocks with memory are reset and warmed up after a jump. A fault turns a block off for the session and plays the fallback; one session's instances share 512 MiB.

**The app** (spec 15, 17, 25). One `Registry` and one `CatalogSlot`; every change publishes one snapshot and a `BlocksChanged` event, and a package another program puts in the folder is noticed within a second. The window has a block manager, an inspector built only from manifests (since Stage 6 it edits the effects of a whole track and of the sequence as it does a clip's), no block id in its code (a test reads the source), all its words in `ui/src/lib/strings.ts`, a strict content security policy, and package text shown only as plain, isolated text. Sound is mixed on a thread of its own. The `.chadfx-block` file type opens the editor's install question, also when the editor is already running (a hand-off through the settings folder). The Windows installer is NSIS only, per user, with `blocks/builtin` beside the program.

**`vep`**. `vep blocks list | info | catalog | validate | pack | install | remove | disable | enable | enable-again`, with the editor's own rules and words; `vep render-frame`, `vep export-still` and `vep export` say which nodes they leave out and why.

**The proofs, and what they prove** (spec 16, 19 to 23)

- `crates/golden` holds pictures and sounds made by the old code before the rewrite (Stage 0) and is never regenerated.
- `crates/render/tests/golden_pictures.rs`: all 351 scenes and the one known panic, byte for byte, on the computer that made them (an NVIDIA RTX A4000, Vulkan, and Microsoft's WARP software adapter for `golden_pictures_software.rs`). On a card with no stored set it compares nothing, and that proves nothing.
- `no_gpu_pictures.rs`: the same 351 scenes, byte for byte, drawn by the software renderer with no graphics card. This is the proof that does not depend on the computer's card; it has run on Windows only (no Mac, Linux or ARM yet).
- `same_card.rs`: today's renderer against the frozen old shaders on the computer's own card, both the way the preview draws and the way the export draws. `card_vs_cpu.rs`: the card and the software renderer stay within a stated bound of each other.
- `crates/audio/tests/golden_sounds.rs`: 118 cases and 590 references; every sample bit-exact or within the rounding statement of spec 23 (at Stage 4: 289 of 344 compared bit-identical, 28 within 3 units in the last place, 27 within the wider form for sums), and the same however a range is cut.
- Containment: tests that broken and hostile blocks of every picture kind are refused or contained, in `crates/blocks`, `crates/render`, `ui/src-tauri` and `crates/cli` (`contained_picture.rs` in each, Stage 6), and the sound side's tests from Stages 1 to 4 (`wasm_*.rs`, `security.rs`, `crates/audio/tests/block_safety.rs` and `hostile.rs`). For sound, Stage 6 added `contained_sound.rs` in `ui/src-tauri` and `crates/cli` (broken modules of every sound kind refused through the service, the install question and `vep`; trapping, fuel-exhausting and out-of-bounds blocks exported through the service and `vep export`, named in the report, with the fallback's samples; changes during an export), a check of the costliest module shapes in `crates/blocks`, `crates/audio/tests/contained_sound.rs` (hostile blocks of every sound kind at once in one mixer session, each playing its exact fallback with the other tracks untouched, bounded), and `ui/src-tauri/tests/contained_sound_player.rs` (the same met through the player's own sound thread: a hostile block installed, replaced and removed while the project plays without holding the transport past the sound leash, a block that lies about its warm-up and latency, and a block that traps or runs out of fuel reported through the preview's link). End to end, `ui/e2e/blocks-install-use-export.e2e.ts` installs the templates into the running app, uses them, exports and checks the result to the level or the sample; `blocks-contained.e2e.ts` runs the costliest valid picture block and a block turned off after a simulated crash, and installs hostile sound blocks (a trap, an out-of-fuel loop, and the costliest the fuel allows) into the running app, which the preview names and the export leaves running.
- `crates/blocks/tests/template.rs` and its siblings check that every template in `blocks/template` validates, installs, draws or plays what its README says; `crates/blocks/tests/docs.rs` checks that BLOCKS.md names every manifest field, block kind, rule code and `vep blocks` command, read from the code.

What none of them proves: other graphics cards, Mac, Linux; what NaN or infinite colours look like (undefined, spec 22); that fuel bounds wall-clock time tightly (denormal arithmetic costs about 9 times more time per unit of fuel; the owner decides whether to price it higher, spec 23); that a real installed file association looks right in Explorer (only seen from inside Claude's app container).

**Measured on the development machine** (i9-13900, RTX A4000, Stage 4 and Stage 6 reports): eight sound tracks with gain, pan and eq3 cost about 185 ms of one core per second of sound in the mixer (about 18%); the player's sound thread used 29% of a core for eight tracks with a tone source and those three filters; the costliest picture block the checker accepts draws a 4K frame in 37 to 49 ms; a mid-range laptop has not been measured.

---

## 1. Interpretation notes on the brief

**"JEV-style" fast judge.** I read this as the family of joint-embedding / verifier-style approaches: a frozen visual encoder (V-JEPA-2, SigLIP-2, CLIP-family) plus a cheap decision layer that scores or selects among supplied options, as opposed to a generative VLM writing prose. The design below defines a `Judge` contract that fits that family and also fits a small VLM with constrained decoding, so the exact model is an implementation detail.

**Where I push back on the brief:**

1. *Python for "AI/model orchestration."* Orchestration (director loop, planner loop, job scheduling, cancellation, budgets) touches project state and must be deterministic and testable. Put it in Rust. Python executes models and returns typed results. Two orchestrators in two languages is the classic way to end up with two sources of truth.
2. *`analysis/ingest/*` in Python.* Shot detection, frame sampling, and audio extraction are FFmpeg work. They belong in the Rust media layer. They run at import for every file, so they must be fast, cancellable, and free of the Python process lifecycle.
3. *Ollama as the integration point.* Fine for the deep VLM and the director LLM. Wrong for the fast judge: per-request HTTP + image base64 + no batching makes it 10-50x slower than batched tensor inference. The judge runs in-process in the model service (PyTorch/ONNX), batched.
4. *A separate manual vs. AI system.* You already rejected this. The design goes one step further: the UI has no editing logic at all. Every gesture produces Commands or a skill invocation.
5. *gRPC for everything.* gRPC for control and small results. Masks, flow fields, embeddings, sampled frame tensors go through memory-mapped files in the cache and are passed by reference.

---

## 2. Repository structure

Monorepo. Cargo workspace + pnpm workspace + uv workspace. One `proto/` directory is the contract between Rust and Python.

```
repo/
├── Cargo.toml                   # workspace
├── rust-toolchain.toml
├── crates/
│   ├── core-time/               # RationalTime, TimeRange, FrameRate, timecode formatting
│   ├── core-ids/                # typed ULID newtypes (AssetId, ClipId, ...)
│   ├── core-model/              # Project, Asset, Sequence, Track, Clip, EffectNode, Keyframe
│   │                            #   + schema version + migrations between model versions
│   ├── core-commands/           # Command enum, Transaction, validation, apply(), inverse-free undo
│   ├── core-query/              # read-only queries over a ProjectSnapshot (used by UI + AI)
│   ├── core-project/            # SQLite persistence: open/save/autosave/journal/recovery
│   ├── effects-registry/        # the primitive EffectKinds, their param schemas, defaults
│   ├── cache/                   # content-addressed artifact store + cache.sqlite index + eviction
│   ├── media/                   # FFmpeg: probe, demux, decode, frame index, seek, encode, proxies
│   ├── ingest/                  # shot detection, frame sampler, audio extract, thumbnails,
│   │                            #   waveforms, loudness, cheap media signals (deterministic analyzers)
│   ├── analysis-api/            # Analyzer trait + manifest + registry (native and remote analyzers)
│   ├── analysis-store/          # AnalysisRecord read/write (SQLite + cache artifacts), queries
│   ├── jobs/                    # scheduler, priorities, cancellation, resource leases, persistence
│   ├── render/                  # wgpu render-graph compiler, primitives, WGSL shaders, color mgmt
│   ├── audio/                   # mixing graph, resampling, cpal output, audio clock
│   ├── playback/                # transport, frame scheduler, decoded-frame cache, prefetch
│   ├── export/                  # offline render + encode pipeline, presets
│   ├── skills-api/              # Skill trait, SkillManifest, registry, schema gen, SkillContext
│   ├── skills/                  # built-in skills, one module per family (video/, audio/, color/, ...)
│   ├── ai-llm/                  # LlmBackend/VlmBackend traits: Ollama, OpenAI-compatible, Anthropic
│   ├── ai-judge/                # Judge trait + criteria types + window scoring driver
│   ├── ai-orchestrator/         # Director, candidate pipeline, Planner agent loop, EditPlan types
│   ├── model-service-client/    # tonic client + Python process supervisor + health/restart
│   ├── app/                     # Tauri v2 app: commands, events, state projection, preview surface
│   └── cli/                     # headless CLI: import, analyze, run-plan, render (CI + scripting)
├── proto/
│   ├── model_service.proto      # Rust <-> Python contract
│   └── analysis_types.proto     # shared analysis result messages
├── python/
│   ├── pyproject.toml           # uv workspace
│   ├── vidai_service/
│   │   ├── server.py            # gRPC server, job runner, cancellation, GPU lease handling
│   │   ├── registry.py          # analyzer discovery (entry points + decorator)
│   │   ├── artifacts.py         # read/write cache artifacts (npy / arrow / png sequences)
│   │   └── models/              # model manager: download, version pin, device placement
│   ├── analyzers/
│   │   ├── fast_judge/          # visual_judge, shot_scorer, range_scorer, duplicate_scorer
│   │   ├── multimodal/          # vlm_client, shot_understanding, compare_shots, semantic_tags, story
│   │   ├── audio/               # transcribe (whisper), diarize, beat_detect, stem_separate
│   │   └── specialized/         # tracking, segmentation, optical_flow, face_identity, interpolation,
│   │                            #   matting, depth
│   └── tests/
├── ui/
│   ├── package.json
│   ├── src/
│   │   ├── bindings/            # GENERATED: Tauri commands, model types, skill schemas
│   │   ├── app/                 # shell, routing, layout, theming
│   │   ├── state/               # Zustand stores mirroring core snapshots (read models only)
│   │   ├── features/
│   │   │   ├── media-bin/
│   │   │   ├── timeline/        # canvas renderer, interaction state machine, gesture -> Command
│   │   │   ├── preview/         # native-surface host element, overlays, transport
│   │   │   ├── inspector/       # effect stacks, skill-provenance editors (generated from schemas)
│   │   │   ├── ai/              # chat/brief panel, plan viewer, candidate browser, approvals
│   │   │   └── jobs/            # job list, progress, cancel
│   │   └── lib/
│   └── tests/
├── schemas/                     # GENERATED + committed: JSON Schema for commands, skills, analysis
├── shaders/                     # WGSL, shared by render crate (include_str!)
├── tools/                       # build-ffmpeg, bundle-python, codegen, fixture generators
├── testdata/                    # small media fixtures (git-lfs), golden frames
└── docs/
```

Rules enforced by CI:

- `core-*` crates must not depend on `media`, `render`, `tauri`, `tonic`, or anything with FFmpeg/GPU. A `cargo deny`/`cargo udeps`-style check plus a `#![forbid(unsafe_code)]` on core.
- `skills` may depend on `core-*`, `analysis-store`, `model-service-client`, `skills-api`. Not on `app` or `render`.
- `render` depends on `core-model`, `effects-registry`, `cache`. Not on `skills` or anything AI.
- `ui/` depends only on generated bindings. Hand-written type duplication is a build failure.

---

## 3. Language per component

| Component | Language | Why |
|---|---|---|
| Data model, commands, undo, queries, persistence | Rust | Determinism, performance, single truth. |
| Media I/O, probing, decode, encode, proxies, frame index | Rust (FFmpeg via bindings) | Frame-accurate seeking needs in-process control, not CLI calls. |
| Ingest analyzers (shots, sampling, signals, waveforms) | Rust | Runs on every file at import. No model needed. |
| Renderer, compositing, effects | Rust + WGSL | wgpu is the only portable GPU story that is also fast. |
| Audio engine | Rust | Real-time thread, no GC. |
| Job scheduler, cache | Rust | Must be cancellable and consistent with project state. |
| Skills (all of them) | Rust | They produce Commands. Some *call* the Python service for model output. |
| Director / Judge driver / Planner loop | Rust | Orchestration owns state and budgets. LLM calls are HTTP. |
| LLM/VLM backends | Rust (HTTP clients) | Thin. |
| Fast judge, deep VLM batching, specialized CV | Python | PyTorch ecosystem. Only models live here. |
| Desktop shell | Rust (Tauri v2) | Small binary, native windows, Rust in the same process as the core. |
| UI | TypeScript + React | Productivity, ecosystem. Zero editing logic. |
| Headless CLI | Rust | CI, scripted tests, batch workflows. |

---

## 4. Libraries and frameworks

### Rust

| Need | Choice | Why / alternatives rejected |
|---|---|---|
| Async runtime | `tokio` | Needed by tonic and reqwest anyway. Core crates stay sync. |
| Immutable collections | `im` (or `imbl`) | Structural sharing for cheap undo snapshots. |
| IDs | `ulid` | Sortable, 128-bit, string-friendly for LLM tool calls. UUIDv7 is an acceptable alternative. |
| Rational math | `num-rational` (wrapped in our own `RationalTime`) | Keep the public type ours. |
| Serialization | `serde` + `serde_json` (interchange) + `ciborium` CBOR (timeline blob) | CBOR for compact blob; JSON for IPC and debugging. |
| JSON Schema | `schemars` | Single source of truth for skill/command schemas sent to the LLM and the UI. |
| TS bindings | `tauri-specta` + `specta` | Generated bindings; kills type drift between Rust and UI. |
| SQLite | `rusqlite` with `bundled` feature | Simple, sync, fine for a desktop app. Not `sqlx` (async adds nothing here). |
| FFmpeg | `ffmpeg-next` (bindings over `ffmpeg-sys-next`) | In-process decode/seek/encode with progress. Also keep the `ffmpeg` CLI available as a job backend for bulk transcodes (proxies) where crash isolation matters. |
| GPU | `wgpu` + `naga` (WGSL) | Portable (DX12/Metal/Vulkan). |
| Audio output | `cpal` | Portable, low-level. Resampling via `rubato`. |
| gRPC | `tonic` + `prost` | Mature, streaming. |
| HTTP | `reqwest` | For Ollama / OpenAI-compatible / Anthropic. |
| ONNX in-process (later) | `ort` | Lets the fast judge and VAD move into Rust without Python. |
| Errors / logging | `thiserror`, `anyhow` (edges only), `tracing` + `tracing-subscriber` | Standard. |
| Concurrency | `crossbeam`, `parking_lot`, `rayon` | Lock-free queues for playback, rayon for CPU analysis. |
| Cancellation | `tokio_util::sync::CancellationToken` | Shared across jobs and skills. |
| Memory-mapped files | `memmap2` | Frame tensors and artifacts shared with Python. |
| Hashing | `blake3` | Content hashes for assets and cache keys. |
| Property tests | `proptest` | Command/undo invariants. |
| Snapshot tests | `insta` | Golden JSON for project state; golden PNG via custom harness. |

### TypeScript / UI

| Need | Choice |
|---|---|
| Build | Vite, TypeScript strict, pnpm |
| UI | React 19, Radix primitives (or shadcn), CSS modules or Tailwind |
| State | Zustand (read-model stores fed by core events). No Redux. |
| Timeline rendering | Custom Canvas2D renderer (PixiJS acceptable if WebGL is needed). The timeline is never DOM. |
| Validation | Zod schemas generated from JSON Schema (`json-schema-to-zod`) for form building |
| Forms for skills/effects | Schema-driven forms (generated from `schemars` output with UI hints) |
| Tests | Vitest + Playwright for the webview |

### Python

| Need | Choice |
|---|---|
| Packaging | `uv`, Python 3.12, locked dependencies, bundled with `python-build-standalone` at release |
| Types / schemas | `pydantic` v2 (mirrors proto messages for validation) |
| gRPC | `grpcio` + `grpcio-tools` (or `betterproto` for nicer types) |
| Tensors | `torch`, `numpy`, `pyarrow` |
| Models (initial picks, re-evaluate at implementation time) | SigLIP-2 / V-JEPA-2 (embeddings), Qwen3-VL (deep VLM), faster-whisper (ASR), pyannote (diarization), SAM 2.1 (segmentation + video object tracking), SEA-RAFT (optical flow), InsightFace/ArcFace (identity), RIFE (interpolation), Demucs (stems), DeepFilterNet (denoise; has a Rust crate too), madmom/BeatNet (beats), Depth Anything v2 (depth), MatAnyone (video matting) |
| Non-ML tools | `ffmpeg` vidstab for stabilization, OpenCV for feature tracking fallbacks |

### FFmpeg build and licensing

Ship an **LGPL** FFmpeg build, dynamically linked, with hardware encoders (NVENC/AMF/QSV/VideoToolbox) and without x264/x265 unless the product itself is GPL. Use OpenH264 or hardware for H.264 software fallback. Decide this at day one because it affects what "export" can promise.

---

## 5. Process boundaries

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Process A: app (Tauri)                                                  │
│                                                                         │
│   WebView (React UI)  ──Tauri IPC (JSON, typed)──►  app crate           │
│                                                        │                │
│   ┌──────────────┐  ┌─────────────┐  ┌──────────────┐  │  ┌──────────┐ │
│   │ core-*       │  │ jobs        │  │ skills       │◄─┘  │ ai-      │ │
│   │ state+cmds   │◄─│ scheduler   │  │ registry     │◄────│ orchestr │ │
│   └──────┬───────┘  └──────┬──────┘  └──────┬───────┘     └────┬─────┘ │
│          │ snapshots       │                │ model calls      │ HTTP   │
│   ┌──────▼───────┐  ┌──────▼──────┐  ┌──────▼───────┐          │        │
│   │ playback +   │  │ ingest +    │  │ model-service│          │        │
│   │ render(wgpu) │  │ media(ffmpeg)│ │ -client      │          │        │
│   │ + audio      │  └─────────────┘  └──────┬───────┘          │        │
│   └──────────────┘                          │ gRPC localhost   │        │
└─────────────────────────────────────────────┼──────────────────┼────────┘
                                              │                  │
┌─────────────────────────────────────────────▼──────┐  ┌────────▼────────┐
│ Process B: model service (Python, child of A)      │  │ Process C:      │
│  analyzer registry · job runner · GPU lease        │  │ Ollama / llama  │
│  fast_judge · multimodal · audio · specialized     │─►│ .cpp / vLLM     │
│  reads/writes cache artifacts (mmap)               │  │ (HTTP, managed) │
└────────────────────────────────────────────────────┘  └─────────────────┘
```

**Why the core, renderer, and playback are in-process with the UI shell:** scrubbing latency. A separate render process would need shared textures across process boundaries on three OSes. Not worth it before v2. The trade is that a GPU device loss can kill the app; mitigate with wgpu device-lost recovery and autosave.

**Why Python is a separate process:** crash isolation (CUDA OOM must not kill the editor), independent restart, independent VRAM budget, and the ability to run it on another machine later (the gRPC address is configuration).

**Why Ollama is a third process:** it is an external server with its own lifecycle. The app *manages* it (detect, start, pull models) but never links it. The `ai-llm` crate has an `OpenAiCompatible` backend so llama.cpp server, vLLM, LM Studio, or a cloud API are drop-in.

**Threads inside process A:**

- UI thread (webview) — owned by Tauri.
- Core thread — owns the `ProjectStore` (current `Arc<ProjectState>`, undo stack). All mutations are serialized through one `mpsc` channel. This is the only writer.
- Playback thread — real-time-ish; holds a snapshot `Arc<ProjectState>`, drives decode prefetch and render; swaps snapshots on change.
- Audio callback thread — cpal; lock-free ring buffer fed by the mixer.
- Decode pool — N threads for FFmpeg decode.
- Job workers — rayon pool for CPU analyzers; tokio tasks for I/O and gRPC.

---

## 6. Communication paths

| Path | Transport | Payload | Notes |
|---|---|---|---|
| UI → core | Tauri commands (`invoke`) | JSON, generated types | `apply_transaction`, `invoke_skill`, `query`, `transport.*`, `jobs.*` |
| core → UI | Tauri events | JSON deltas: `state.changed {version, changed_ids}`, `job.progress`, `playback.position` | UI pulls read-models via query after a change notification (pull, not push, so the UI never falls behind). |
| UI → preview | none (native) | wgpu renders to a native child surface positioned under a placeholder element | Overlays (safe zones, gizmos) drawn by wgpu too. Fallback path: readback → shared memory → `<canvas>` for environments where child surfaces misbehave. |
| UI → thumbnails/waveforms | custom URI scheme `cache://` | PNG/binary from the cache | Served by Tauri's protocol handler, so the webview caches it. |
| core ↔ Python | gRPC (tonic ↔ grpcio) on `127.0.0.1:<random port>` with a per-launch auth token | protobuf control messages, streaming progress | Named pipes/UDS are not worth the platform divergence. |
| core ↔ Python (bulk) | memory-mapped files in the cache dir | sampled frames (`u8 NHWC` + index), embeddings (Arrow), masks (RLE or PNG seq), flow (`f16` npy) | Messages carry artifact keys, never bytes. |
| orchestrator → LLM/VLM | HTTP | Ollama `/api/chat` with `format: <json schema>`; OpenAI-compatible `/v1/chat/completions`; Anthropic Messages | VLM images are sent base64 by the Rust side; frame selection is ours. |
| Python → Ollama | HTTP | only for batch VLM analyzers | Same `VlmBackend` policy config as Rust, read from the app at startup. |

---

## 7. Footage flow end to end

```
IMPORT
 1. media::probe(path)                      → StreamInfo, color, timebases, VFR flag
 2. blake3 content hash (sampled chunks)    → AssetId stable across relinks
 3. Command::ImportAsset                    → project state (undoable)
 4. jobs enqueued, in priority order:
      a. frame_index   (pts table, keyframe positions; frame-accurate seek)  [Rust, high]
      b. thumbnails + waveform + loudness                                     [Rust, high]
      c. proxy (if enabled or source is heavy)                                [Rust/ffmpeg CLI, bg]
      d. shot_detect + frame_sample(2 fps, 384px) + audio_extract(16k mono)   [Rust, bg]
      e. media_signals (sharpness, luma, motion, audio RMS, VAD)              [Rust, bg]
      f. transcribe (if speech detected)                                      [Python, bg]
      g. embed_frames (SigLIP/V-JEPA)                                         [Python, bg]

USER INTENT  ("fast-paced football hype video, prioritize tackles...")
 5. Director (LLM, structured output)       → Brief { criteria[], avoid[], pacing, duration, style }
 6. Judge driver (Rust) builds CriteriaSet, requests judge job
      Python fast_judge: criteria text → embeddings; windows → scores        [ms per window]
      Output: per-window score vectors + fixed dimensions (technical_quality, ...)
 7. Candidate selection (Rust, deterministic):
      rank windows, suppress duplicates, keep top-K per criterion + a "wildcard" quota of
      technically imperfect but visually interesting windows (never discard on quality alone)
 8. Deep analysis on candidates only: shot_understanding (VLM, structured JSON) → AnalysisRecords
 9. Planner (LLM agent loop) with tools = {queries, skills}:
      queries: analysis.search, project.describe, candidates.list, asset.frames
      skills:  timeline.assemble, video.speedRamp, graphics.caption, audio.duck, ...
    Each skill call → Transaction → core applies → one undo step → UI updates live
10. Playback thread renders from the new snapshot; user tweaks manually (same Commands)
11. Export: offline render graph evaluation → encoder → file; also a job, cancellable
```

Everything in steps 4–8 is a **Job** with a cache key. Re-running a brief re-uses embeddings and shots; only step 6 onward recomputes, and 6 is cheap.

---

## 8. Project database and cache

### 8.1 Project file: `project.vep` (a SQLite database)

Single file, WAL mode, opened exclusively. Portable: all asset paths stored relative to the project when possible, with content hashes for relinking.

```sql
-- identity + schema
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);          -- schema_version, app_version, created
-- the timeline document: one row per save point; current = max(version)
CREATE TABLE document (
  version INTEGER PRIMARY KEY,
  state_json TEXT NOT NULL,        -- canonical JSON of ProjectState (sequences, tracks, clips, effects, markers)
  state_hash TEXT NOT NULL,        -- blake3 hex of the canonical JSON
  created_unix_ms INTEGER NOT NULL,
  label TEXT
);
-- journal of operations since the last document checkpoint (crash recovery + history UI)
CREATE TABLE journal (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  base_version INTEGER NOT NULL,
  op TEXT NOT NULL,                -- apply | undo | redo
  tx_json TEXT,                    -- Transaction (label, origin, commands) for apply ops
  result_hash TEXT NOT NULL,       -- state hash after the op; verified on replay
  applied_unix_ms INTEGER NOT NULL
);
-- assets are relational because the UI and AI query them
CREATE TABLE assets (
  id TEXT PRIMARY KEY, kind TEXT, rel_path TEXT, abs_path_hint TEXT,
  content_hash BLOB, size INTEGER, mtime INTEGER, status TEXT,
  media_info_json TEXT, imported_at TEXT
);
-- small blobs the project depends on (dense curves, LUTs, generated images)
CREATE TABLE blobs (key TEXT PRIMARY KEY, mime TEXT, bytes BLOB, created_at TEXT);
-- analysis metadata (payload inline when small, else artifact key into cache)
CREATE TABLE analysis (
  id TEXT PRIMARY KEY, asset_id TEXT, analyzer_id TEXT, analyzer_version TEXT,
  params_hash BLOB, scope_kind TEXT, scope_start_num INTEGER, scope_start_den INTEGER,
  scope_dur_num INTEGER, scope_dur_den INTEGER,
  status TEXT, payload_json TEXT, artifact_key TEXT, created_at TEXT
);
CREATE INDEX analysis_lookup ON analysis(asset_id, analyzer_id, analyzer_version);
-- flattened, queryable score rows for the judge (so the planner can SQL over them)
CREATE TABLE judge_scores (
  analysis_id TEXT, criteria_set_id TEXT, asset_id TEXT,
  start_num INTEGER, start_den INTEGER, dur_num INTEGER, dur_den INTEGER,
  criterion TEXT, score REAL
);
CREATE INDEX judge_by_criterion ON judge_scores(criteria_set_id, criterion, score DESC);
-- AI provenance
CREATE TABLE briefs (id TEXT PRIMARY KEY, json TEXT, created_at TEXT);
CREATE TABLE plans  (id TEXT PRIMARY KEY, brief_id TEXT, json TEXT, status TEXT, created_at TEXT);
-- persisted job queue (resume background work after restart)
CREATE TABLE jobs (id TEXT PRIMARY KEY, kind TEXT, spec_json TEXT, priority INTEGER,
  status TEXT, cache_key TEXT, created_at TEXT, finished_at TEXT, error TEXT);
```

Why a blob for the timeline and tables for everything else: the in-memory immutable state is the truth; the timeline is read whole and written whole. Analysis and assets are queried by predicate, so they are relational. Normalizing clips into rows buys nothing until collaboration, and we are not building collaboration first.

Save semantics: every committed Transaction is appended to `transactions` immediately (autosave is free). "Save" writes a new `document` row and truncates the journal. On open, load latest `document`, replay `transactions` where `base_version` matches. The `result_hash` after replay must match or recovery stops and reports.

Also provide `project export --json` producing a canonical JSON document for diffing and version control.

### 8.2 Cache: `~/.cache/<app>/` (configurable, per machine, never inside the project)

```
cache/
  cache.sqlite                    # index: key → path, size, last_used, producer, pinned
  artifacts/<aa>/<bb>/<key>.<ext> # content-addressed by recipe hash
  proxies/<asset_hash>/...        # proxies are artifacts too, just big
  models/                         # downloaded model weights (Python model manager + Ollama dir)
  frames/                         # mmap frame arenas, short-lived
```

Artifact key = `blake3(producer_id, producer_version, asset_content_hash, params_canonical_json, scope)`. The project references artifacts by key **and** stores the recipe in `analysis`, so anything missing can be regenerated. Eviction: LRU by `last_used` under a size budget; keys referenced by open projects are pinned.

Artifact formats:

| Data | Format |
|---|---|
| Per-frame tables (signals, embeddings, window scores) | Arrow IPC file (mmap-able), one row per sample |
| Masks | PNG sequence in a zip, or RLE in Arrow; renderer streams per frame |
| Optical flow | `.npy` f16 per frame or one chunked file |
| Dense curves (stabilization transforms, tracked positions) | small → `blobs` table in project; Arrow if large |
| Thumbnails | JPEG/WebP sprite sheets |
| Waveforms | binary peaks (min/max per bucket, multiple zoom levels) |
| Proxies | H.264/ProRes Proxy via ffmpeg |

---

## 9. Skill discovery and registration

Skills are Rust. Discovery is **compile-time** via `inventory` (or an explicit `builtin_skills()` list; both are fine, inventory just avoids forgetting). Each skill is a struct implementing `Skill`:

```rust
pub trait Skill: Send + Sync + 'static {
    fn manifest(&self) -> &SkillManifest;
    /// Validate input against schema, compute, and return a Transaction (or none).
    fn run<'a>(&'a self, ctx: &'a SkillContext, input: Value) -> BoxFuture<'a, Result<SkillOutput, SkillError>>;
}

pub struct SkillManifest {
    pub id: SkillId,                    // "video.speedRamp"
    pub version: Version,               // semver; bump on behavior change (affects provenance re-run)
    pub title: String,
    pub description: String,            // written for an LLM: when to use, when not to
    pub category: SkillCategory,        // Video | Audio | Color | Graphics | Transition | Tracking | Segmentation | Timeline | Composite
    pub input_schema: RootSchema,       // schemars from the typed Input struct
    pub output_schema: RootSchema,
    pub requires: Requirements,         // gpu: bool, network: bool, models: Vec<ModelRef>, analyzers: Vec<AnalyzerId>
    pub cost: CostHint,                 // Instant | Seconds | Minutes | Long (planner uses this)
    pub targets: Vec<TargetKind>,       // Clip | Range | Track | Sequence | Asset
    pub ui: Option<UiHints>,            // sliders, ranges, grouping, "re-run on change"
    pub composite_of: Vec<SkillId>,     // for composite skills, declared dependencies
    pub idempotent: bool,
}

pub struct SkillContext {
    pub project: ProjectSnapshot,       // immutable view
    pub query: QueryEngine,             // core-query over the snapshot + analysis store
    pub analysis: AnalysisClient,       // ensure_analysis(asset, analyzer, params) → waits on a Job
    pub models: ModelClient,            // VLM/LLM/judge access with budget
    pub skills: SkillInvoker,           // call other skills (composite)
    pub cancel: CancellationToken,
    pub progress: ProgressSink,
    pub ids: IdFactory,                 // deterministic ULIDs when seeded (tests, replay)
}

pub struct SkillOutput {
    pub transaction: Option<Transaction>,   // commands to apply; core validates again
    pub result: Value,                      // matches output_schema; returned to planner/UI
    pub notes: Vec<String>,                 // human-readable explanation
}
```

Registration macro:

```rust
#[skill(id = "video.speedRamp", version = "1.2.0", category = Video, cost = Instant)]
pub struct SpeedRamp;

#[derive(Deserialize, JsonSchema)]
pub struct SpeedRampInput {
    pub clip_id: ClipId,
    /// Keyframes of speed multiplier in clip-local time
    pub ramp: Vec<SpeedKey>,
    #[serde(default)] pub preserve_pitch: bool,
}
```

The registry exposes:

- `list() -> Vec<&SkillManifest>` → generates LLM tool definitions, UI menus, `schemas/skills/*.json`.
- `invoke(id, input, ctx)` → the only entry point. Used by the Tauri command `invoke_skill` (manual UI) and by the planner (AI). Identical path.
- `dry_run(id, input, ctx)` → returns the Transaction without applying (planner uses it to validate plans).

**Provenance rule.** Every EffectNode a skill creates carries `provenance: { skill_id, skill_version, input_hash, step_id }`. The inspector shows the skill's input form instead of the raw primitives when provenance exists, and re-invokes the skill on change, replacing the nodes with the same provenance. This is how "Stabilize" remains an editable effect to the user while the renderer only ever sees Transform + DenseCurve.

**Skills that need Python** do not know Python exists. They call `ctx.analysis.ensure(asset, "specialized.segmentation", params)` and receive an artifact key. The analyzer registry decides where it runs.

---

## 10. Analysis module discovery and registration

One `Analyzer` concept, two hosts.

```rust
pub struct AnalyzerManifest {
    pub id: AnalyzerId,                 // "ingest.shot_detect", "fast_judge.range_scorer", "specialized.segmentation"
    pub version: Version,               // part of the cache key
    pub host: Host,                     // Native | ModelService
    pub inputs: Vec<InputSpec>,         // AssetStream(Video) | Artifact("ingest.frame_sample") | Analysis("ingest.shot_detect")
    pub outputs: Vec<OutputSpec>,       // Inline(schema) | Artifact(format)
    pub params_schema: RootSchema,
    pub resources: Resources,           // cpu_threads, gpu: None|Shared|Exclusive{vram_mb}, network, model: Option<ModelRef>
    pub cache: CachePolicy,             // Persistent | Session | None ; granularity: Asset | Range
    pub supports_range: bool,           // can run on a sub-range (tracking: yes; shot_detect: no)
    pub cancellable: bool,
}
```

- **Native analyzers** register with `inventory` in the `ingest` crate (and later `ai-judge` when the judge moves to `ort`).
- **Python analyzers** register with a decorator and are discovered via `importlib.metadata` entry points (`vidai.analyzers` group), so third-party packages can add analyzers. The service's `ListAnalyzers` RPC returns manifests (as proto) at startup; the Rust registry merges them with native ones. Mismatched or missing manifests fail fast with a clear UI message.

```python
@analyzer(
    id="specialized.segmentation",
    version="1.0.0",
    inputs=[Input.artifact("ingest.frame_sample"), Input.asset_stream("video")],
    outputs=[Output.artifact("masks", format="png_zip")],
    resources=Resources(gpu="exclusive", vram_mb=6000, model="sam2.1-hiera-large"),
    cache=Cache.persistent(granularity="range"),
    supports_range=True,
)
class Segmentation(Analyzer):
    Params = SegmentationParams          # pydantic model → JSON schema → proto Struct
    def run(self, ctx: AnalyzerContext, params: SegmentationParams) -> AnalyzerResult: ...
```

Dependencies resolve into a job DAG automatically: asking for `fast_judge.range_scorer` on an asset schedules `ingest.shot_detect` → `ingest.frame_sample` → `fast_judge.embed_frames` → `range_scorer`, skipping anything already cached.

---

## 11. Core data structures

Rust is the canonical definition; TS and Python types are generated. Shown abbreviated.

```rust
// ---------- time ----------
/// Exact rational seconds. Normalized (gcd=1, den>0). i64 num gives centuries at any sane den.
pub struct RationalTime { pub num: i64, pub den: i64 }
pub struct TimeRange   { pub start: RationalTime, pub duration: RationalTime }
pub struct FrameRate   { pub num: u32, pub den: u32 }            // 24000/1001
impl RationalTime {
    pub fn from_frames(n: i64, rate: FrameRate) -> Self;
    pub fn to_frame_floor(&self, rate: FrameRate) -> i64;
    pub fn to_samples(&self, sample_rate: u32) -> i64;
    pub fn snap(&self, rate: FrameRate) -> Self;                 // sequence-time positions are always snapped
}

// ---------- assets ----------
pub struct Asset {
    pub id: AssetId,
    pub kind: AssetKind,                       // Video | Audio | Image | ImageSequence | Subtitle
    pub source: SourceRef,                     // rel_path, abs_hint, content_hash, size, mtime
    pub info: MediaInfo,                       // container, duration, streams
    pub status: AssetStatus,                   // Online | Offline | Relinked
    pub proxy: Option<ArtifactKey>,
    pub tags: Vec<String>,
}
pub enum StreamInfo {
    Video { index: u32, codec: String, width: u32, height: u32, par: Rational,
            frame_rate: Option<FrameRate>, vfr: bool, time_base: Rational,
            color: ColorInfo /* primaries, transfer, matrix, range */, rotation: i32,
            frame_count: Option<u64> },
    Audio { index: u32, codec: String, sample_rate: u32, channels: u16, layout: String },
}

// ---------- timeline ----------
pub struct Sequence {
    pub id: SequenceId, pub name: String,
    pub frame_rate: FrameRate, pub width: u32, pub height: u32,
    pub audio_sample_rate: u32,                // 48000
    pub working_space: WorkingSpace,           // LinearRec709 (day one), ACEScg (later)
    pub tracks: Vector<Track>,                 // im::Vector; video tracks bottom→top, audio tracks
    pub effects: Vector<EffectNode>,           // sequence-level (output) effects
    pub markers: Vector<Marker>,
}
pub struct Track {
    pub id: TrackId, pub kind: TrackKind, pub name: String,
    pub clips: Vector<Clip>,                   // sorted by timeline start, non-overlapping (invariant)
    pub effects: Vector<EffectNode>,
    pub muted: bool, pub locked: bool, pub solo: bool,
}
pub struct Clip {
    pub id: ClipId,
    pub source: ClipSource,                    // Asset { asset_id, stream_index } | Generator(GeneratorKind)
    pub timeline_range: TimeRange,             // sequence time, snapped
    pub source_in: RationalTime,               // asset stream time, exact (VFR-safe)
    pub effects: Vector<EffectNode>,           // ordered stack; TimeRemap is just a node here
    pub transition_in: Option<Transition>, pub transition_out: Option<Transition>,
    pub link_group: Option<LinkGroupId>,       // A/V linking
    pub enabled: bool, pub label: Option<String>, pub color: Option<Label>,
    pub provenance: Option<Provenance>,        // who placed it (user | skill | plan step)
}

// ---------- effects ----------
pub struct EffectNode {
    pub id: EffectId,
    pub kind: EffectKind,                      // closed enum of renderer primitives (see §12)
    pub enabled: bool,
    pub params: BTreeMap<ParamId, Param>,
    pub mask: Option<MaskRef>,                 // restrict to a mask artifact or shape
    pub blend: Option<BlendMode>,
    pub provenance: Option<Provenance>,
}
pub struct Param { pub value: ParamValue, pub keyframes: Option<Vector<Keyframe>> }
pub enum ParamValue {
    Float(f64), Int(i64), Bool(bool), Vec2([f64;2]), Vec3([f64;3]), Color(Rgba),
    Text(String), Enum(String), Curve(Vec<CurvePoint>),
    DenseCurve(BlobKey),                       // per-frame data (stabilization, tracking); in project blobs
    Artifact(ArtifactKey),                     // large cached data (mask sequence, flow)
}
pub struct Keyframe { pub time: RationalTime /* clip-local */, pub value: ParamValue, pub interp: Interp }
pub enum Interp { Hold, Linear, Bezier { in_tangent: [f64;2], out_tangent: [f64;2] } }
pub struct Provenance { pub skill_id: SkillId, pub skill_version: Version, pub input_hash: Hash, pub plan_step: Option<StepId> }

// ---------- commands ----------
pub enum Command {
    ImportAsset { asset: Asset }, RemoveAsset { id: AssetId }, RelinkAsset { id: AssetId, source: SourceRef },
    CreateSequence { sequence: Sequence }, AddTrack { seq: SequenceId, track: Track, index: usize }, RemoveTrack { id: TrackId },
    InsertClip { track: TrackId, clip: Clip },                       // fails if overlap unless ripple
    RemoveClip { id: ClipId, ripple: bool },
    MoveClip { id: ClipId, track: TrackId, start: RationalTime },
    TrimClip { id: ClipId, edge: Edge, to: RationalTime, mode: TrimMode /* Normal | Ripple | Roll | Slip | Slide */ },
    SplitClip { id: ClipId, at: RationalTime, new_id: ClipId },     // caller supplies the new id
    SetClipEnabled { id: ClipId, enabled: bool },
    AddEffect { target: EffectTarget, effect: EffectNode, index: Option<usize> },
    UpdateEffect { id: EffectId, patch: ParamPatch },
    RemoveEffect { id: EffectId }, ReorderEffect { id: EffectId, index: usize },
    SetKeyframes { effect: EffectId, param: ParamId, keyframes: Vec<Keyframe>, range: Option<TimeRange> },
    SetTransition { clip: ClipId, edge: Edge, transition: Option<Transition> },
    AddMarker { .. }, RemoveMarker { .. },
    PutBlob { key: BlobKey, bytes: Bytes, mime: String },
    SetMetadata { target: EntityRef, key: String, value: Value },
}
pub struct Transaction {
    pub id: TransactionId,
    pub label: String,                         // "Speed ramp on Clip 12", shown in Edit > Undo
    pub origin: Origin,                        // User | Skill { id, step } | System
    pub commands: Vec<Command>,
}

// ---------- analysis ----------
pub struct AnalysisRecord {
    pub id: AnalysisId, pub asset_id: AssetId,
    pub analyzer: AnalyzerId, pub version: Version, pub params_hash: Hash,
    pub scope: Scope,                          // Asset | Range(TimeRange) | Shot(ShotId) | Frame(RationalTime)
    pub status: Status, pub created_at: Timestamp,
    pub payload: Payload,                      // Inline(Value) | Artifact(ArtifactKey, Format)
}
// typed inline payloads (all `JsonSchema`):
pub struct Shot { pub id: ShotId, pub range: TimeRange, pub representative: Vec<RationalTime> }
pub struct WindowScores { pub criteria_set: CriteriaSetId, pub windows: Vec<WindowScore> }
pub struct WindowScore { pub range: TimeRange, pub scores: BTreeMap<String, f32> /* criteria + fixed dims */ }
pub struct ShotUnderstanding {                 // the VLM's structured output
    pub shot_id: ShotId, pub description: String, pub subjects: Vec<String>,
    pub shot_type: ShotType, pub purpose: Vec<String>, pub camera_motion: CameraMotion,
    pub best_range: TimeRange, pub starts_cleanly: bool, pub ends_cleanly: bool,
    pub quality_notes: Vec<String>, pub confidence: f32,
}
pub struct Transcript { pub segments: Vec<Segment>, pub words: Vec<Word>, pub language: String }

// ---------- AI ----------
pub struct Brief {
    pub id: BriefId, pub user_intent: String,
    pub look_for: Vec<Criterion>, pub avoid: Vec<Criterion>,
    pub pacing: Pacing, pub target_duration: Option<RationalTime>,
    pub style: Vec<String>, pub music: Option<AssetId>, pub deliverable: Deliverable,
}
pub struct Criterion { pub id: String, pub text: String, pub weight: f32, pub polarity: Polarity }
pub struct EditPlan {
    pub id: PlanId, pub brief: BriefId,
    pub steps: Vec<PlanStep>, pub status: PlanStatus,
}
pub struct PlanStep {
    pub id: StepId, pub rationale: String,
    pub skill: SkillId, pub input: Value,
    pub depends_on: Vec<StepId>,
    pub status: StepStatus,                    // Proposed | Approved | Running | Applied(TransactionId) | Failed | Skipped
}
```

Invariants checked on every `apply`: clips in a track sorted and non-overlapping; `timeline_range.start` snapped to sequence frame rate; every referenced id exists; keyframes sorted by time; `source_in >= 0` and within asset duration (warn, not fail, for VFR edges).

---

## 12. Renderer primitives and the effect graph

> Superseded where it names a fixed list of primitives: every effect is now a plug-in block, and the renderer and the mixer know none by name. See "Plug-in blocks implementation notes (2026-11-06)" above and [BLOCKS.md](BLOCKS.md). The colour management and the compile-a-snapshot design below still hold.

The project model never contains "stabilize" or "caption". It contains:

| Primitive | Params (keyframable) |
|---|---|
| `Transform` | position, scale, rotation, anchor, opacity |
| `Crop` | left/right/top/bottom, feather |
| `Composite` | blend mode, opacity (implicit per track) |
| `Blur` | radius, type (gaussian/box/directional), angle |
| `ColorMatrix` | 4x4 + offset (exposure, saturation, temperature all compile here) |
| `Curves` | per-channel curve |
| `Lut` | artifact (cube), intensity |
| `TimeRemap` | DenseCurve or keyframes mapping clip time → source time, frame blending mode |
| `Mask` | shape (rect/ellipse/path) or artifact (mask sequence), feather, invert |
| `Text` | rich text runs, font, layout box, stroke/shadow |
| `Shader` | WGSL source + declared uniforms (escape hatch, sandboxed) |
| `AudioGain`, `AudioEq`, `AudioCompressor`, `AudioPan`, `AudioTimeStretch` | the audio side |

The `render` crate compiles a snapshot at time `t` into a DAG: for each visible clip, evaluate TimeRemap → source time → decode frame → input transform to working space → effect stack → track composite → sequence effects → output transform. The DAG is cached per snapshot version and invalidated by `changed_ids`.

Color management is explicit from day one: decode → input transform (by stream `ColorInfo`, incl. HLG/PQ/log) → **linear fp16 working space** → output transform. Phone HLG footage shows up on day two; retrofitting this is painful.

---

## 13. Undo/redo and determinism

- `ProjectState` is immutable (`im` collections). `apply(&ProjectState, &Transaction) -> Result<ProjectState, CommandError>` is pure. No I/O, no clock, no randomness.
- The `ProjectStore` holds `current: Arc<ProjectState>`, `undo: Vec<(Arc<ProjectState>, Transaction)>`, `redo: Vec<..>`. Undo is `current = undo.pop().0`. Memory is fine thanks to structural sharing; cap the stack at N and drop the oldest.
- One Transaction = one undo step, regardless of origin. A composite skill producing 400 commands is one undo.
- Entity IDs are supplied by the caller (UI, skill), never generated inside `apply`. Replaying a journal yields identical state and identical hashes. Tests assert `hash(replay(journal)) == stored_hash`.
- Interactive gestures (dragging a clip) use a **scratch overlay**: the UI sends `preview_transaction` which applies to a throwaway state the playback thread renders; `commit_transaction` on mouse-up makes it real. No undo noise from drags.
- Skill randomness (if ever) takes an explicit `seed` input and records it in provenance.
- Timestamps live only in Transaction metadata, never in state.

---

## 14. Jobs, cancellation, prioritization, GPU, caching

```rust
pub struct Job {
    pub id: JobId,
    pub kind: JobKind,                 // Analyze { analyzer, asset, scope, params } | Proxy | Export | SkillRun | ModelPull
    pub priority: Priority,            // Interactive(0) | UserRequested(1) | Background(2) | Idle(3)
    pub deps: Vec<JobId>,
    pub resources: Resources,
    pub cache_key: Option<ArtifactKey>,
    pub cancel: CancellationToken,
    pub status: JobStatus,
}
```

Scheduler rules:

1. **Dedupe by cache key** before enqueue. If the artifact exists, complete immediately. If an identical job is running, attach a second waiter.
2. **Priority with aging** so background jobs eventually run during long sessions.
3. **Resource leases**: CPU threads (rayon budget), GPU `Shared` vs `Exclusive`, network. Exactly one GPU-exclusive job runs at a time.
4. **Playback preemption**: when the transport is playing or scrubbing, the scheduler sends `Throttle { gpu_budget: Low }` to the model service; Python analyzers check the budget and cancellation between batches and shrink batch size. Export holds a GPU lease above background analysis.
5. **Cancellation is cooperative** everywhere: Rust via `CancellationToken`, Python via `ctx.should_stop()` between batches, FFmpeg via interrupt callback. Jobs must be cancellable within ~250 ms or declare `cancellable: false` (rare; e.g. model download finalization).
6. **Persistence**: jobs are written to the project `jobs` table on enqueue and on terminal state. On open, `Background`/`Idle` jobs resume; `Interactive` ones are dropped.
7. **Ranged work**: analyzers with `supports_range` run on the smallest range the requester needs (tracking on the clip's used range, not the whole 20-minute file).
8. **Per-asset pipelines** run in dependency order; the scheduler never starts a model job whose inputs are not cached.

VRAM policy: the app process keeps its wgpu device small (frame textures, effect intermediates). The model service runs one model family at a time by default and unloads idle models after a timeout. Ollama's `keep_alive` is managed the same way. All three honor a user-configurable VRAM split.

---

## 15. Keeping the UI responsive

- The webview thread never blocks: all Tauri commands are async and return within milliseconds or return a `JobId`.
- The core thread applies transactions in microseconds; heavy work (skill computation) happens before the transaction exists.
- UI stores are **read models** built from `core-query` projections (e.g. `TimelineView { tracks: [{ clips: [{id, x, w, label, thumbs}] }] }`), requested after a `state.changed` event and diffed by id. The UI never holds the full project model.
- Timeline draws on canvas with virtualization; thumbnails and waveforms arrive via `cache://` URLs and lazy generation jobs at `Interactive` priority for the visible range only.
- Playback runs on its own thread with a lock-free frame queue; the audio callback is the master clock; video frames are presented by pts against that clock. Decoding prefetches ahead in both directions for scrubbing.
- Analysis runs in another process. Its progress arrives as throttled events (max ~10 Hz per job).
- Preview rendering uses a resolution ladder (full / half / quarter) chosen by frame budget; drop to lower rung on misses, climb back when idle.

---

## 16. Local inference through Ollama

`ai-llm` defines:

```rust
pub trait LlmBackend { async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>; }  // supports JSON schema output + tool calls
pub trait VlmBackend { async fn describe(&self, req: VlmRequest /* frames + prompt + schema */) -> Result<Value>; }
pub struct ModelPolicy {            // user-editable config
    pub director: ModelRef,         // e.g. ollama:qwen3:30b or anthropic:claude-...
    pub planner: ModelRef,
    pub vlm_deep: ModelRef,         // ollama:qwen3-vl:8b
    pub vlm_cheap: ModelRef,        // ollama:qwen3-vl:2b for compare_shots triage
    pub judge: JudgeRef,            // python:siglip2 | python:vjepa2 | native:onnx:...
}
```

Ollama integration specifics:

- Detect binary, start if not running, `GET /api/tags`, `POST /api/pull` with progress as a `ModelPull` job. Surface missing-model states in the UI, not as errors.
- Use `/api/chat` with `format: <JSON schema>` for all structured outputs (Brief, ShotUnderstanding, Plan). Validate with the same `schemars` schema on return; on validation failure retry once with the error appended.
- Images: we pick frames (4–8 per shot, ≤ 768 px long edge, chosen by shot detect + signals), send base64. Never send a video file. For long shots, tile: multiple calls with overlapping windows, then a cheap merge.
- Concurrency: Ollama serializes per model unless `OLLAMA_NUM_PARALLEL` is set; the orchestrator keeps a small semaphore (2–4) and prefers larger batches of frames per call over more calls.
- Keep-alive: set `keep_alive` to a few minutes during an active plan; set to `0` when export starts (frees VRAM).
- Everything above is behind the trait, so `llama.cpp` server, vLLM, LM Studio (OpenAI-compatible) and cloud (Anthropic, OpenAI, Gemini) are config changes. Cloud backends require explicit opt-in per project because footage leaves the machine.

---

## 17. The fast visual judge

Contract (`ai-judge` crate, implementation in Python first):

```rust
pub struct CriteriaSet { pub id: CriteriaSetId, pub criteria: Vec<Criterion>, pub fixed: Vec<FixedDim> }
pub enum FixedDim { TechnicalQuality, VisualInterest, SubjectClarity, ActionPresent, DialogueUsability, BrollUsability, DuplicateLikelihood, AccidentalFootage }
pub trait Judge {
    fn capabilities(&self) -> JudgeCaps;   // supports_text_criteria, supports_video_windows, max_batch
    async fn score_windows(&self, asset: AssetId, windows: &[TimeRange], criteria: &CriteriaSet) -> Result<Vec<WindowScore>>;
}
```

Implementation A (ship first): **embedding judge**.
- `fast_judge.embed_frames`: SigLIP-2 (image) or V-JEPA-2 (short clips) over the 2 fps samples → Arrow artifact per asset. Computed once per asset, criteria-independent. This is the expensive step and it is cached forever.
- `fast_judge.range_scorer`: criteria → text embeddings (SigLIP text tower) or a few-shot prototype bank; window score = temporal pooling of cosine similarities, minus duplicate penalty (similarity to already-selected windows), with fixed dims from deterministic signals (sharpness, luma, motion, VAD) and small zero-shot prompts. Milliseconds per window, thousands of windows per second.
- Scores are **rank signals**. Normalize per project (z-score per criterion) so director thresholds ("top 15%") mean something across footage sets.

Implementation B: **small VLM constrained decoding** (Qwen3-VL-2B / SmolVLM / Moondream): score by reading logprobs over a fixed answer set (`KEEP/MAYBE/DISCARD` or `0..9`). Slower but handles compositional criteria ("coach yelling at referee") better. Use it as a second-pass re-ranker on the top-N from A.

Implementation C (the one place custom "training" makes sense): **learned heads on frozen embeddings**. Logistic regression / small MLP trained from user choices (kept vs. removed clips, inspector thumbs up/down). Trains in seconds on CPU, per user, per genre. Lives behind the same `Judge` trait.

Windows: sliding 3 s windows with 1 s stride inside each shot, plus the shot itself. Output example matches the brief: `0–4 s low, 4–11 s high, 11–16 s high, 16–21 s low`.

Non-discard rule: candidate selection always reserves a quota for windows with high `visual_interest` but low `technical_quality`, labeled `wildcard`, so the planner sees them with a note. The judge never deletes anything; it orders.

Swapping implementations later: the artifact format (`WindowScores`) and `CriteriaSet` are the contract. A native `ort` implementation of A is a drop-in once the model service is optional.

---

## 18. Existing models vs. custom

| Capability | Use existing | Custom? |
|---|---|---|
| Shot detection | Histogram/content diff in Rust (TransNetV2 optional) | No |
| Visual embeddings | SigLIP-2, V-JEPA-2, CLIP | No |
| Deep shot understanding | Qwen3-VL (or best local VLM at the time) | No |
| Transcription / diarization | faster-whisper, pyannote | No |
| Segmentation / video object tracking | SAM 2.1 | No |
| Point/box tracking | SAM 2.1 masks → boxes; CoTracker for points | No |
| Optical flow | SEA-RAFT | No |
| Stabilization | ffmpeg vidstab (CPU, deterministic); flow-based later | No |
| Frame interpolation | RIFE | No |
| Face identity | InsightFace | No |
| Beat detection | madmom / BeatNet / librosa | No |
| Denoise / stems | DeepFilterNet / Demucs | No |
| Matting | MatAnyone / RVM | No |
| Color auto-grade | Histogram matching + LUTs (no ML) | No |
| Judge decision heads | — | **Yes, tiny**: heads on frozen embeddings from user feedback |
| Director / planner reasoning | Any capable LLM | No; prompts + tools are the product |

Our moat is the orchestration contract (skills → commands → deterministic state) and the data we collect about which candidates humans actually keep. Not model weights.

---

## 19. Decisions that are hard to change later

1. **Rational time in the model.** Changing this later means touching every command, every analysis scope, every file.
2. **Immutable state + pure `apply`.** Mutable state with inverse commands cannot be bolted on afterward without rewriting undo, playback snapshotting, and the AI dry-run.
3. **Caller-supplied IDs, ULIDs.** Required for deterministic replay and for the LLM to refer to things it is about to create.
4. **Project stores primitives only; skills leave provenance.** If "Stabilize" ever becomes a first-class effect kind the renderer has to understand, the core stops being small.
5. **Effect stacks per clip/track/sequence, not a free node graph.** A node graph in the model would make every UI, skill, and AI query harder. The render DAG is derived; that is enough.
6. **Skills return Transactions; nothing else mutates.** The entire AI story depends on this.
7. **Python never owns project state and never calls skills.** Prevents the two-brain problem.
8. **Artifacts are recipe-addressed and regenerable.** Projects must open on a machine with an empty cache.
9. **Explicit color management with a linear working space.** Retrofitting is a visual regression on every existing project.
10. **Schema versioning + migrations from v0.** `document.state_blob` carries a schema version; every release ships migrations; a project from the first alpha must open in v3.
11. **Frame index per asset** (pts table) instead of trusting container seeking. Frame accuracy on VFR phone footage depends on it.
12. **Typed IPC contracts generated from one source.** Hand-maintained TS or Python mirrors rot in weeks.
13. **FFmpeg licensing posture** (LGPL dynamic). Changing it later means re-auditing every codec path.
14. **Preview via a native surface.** Deciding later to move from readback-to-canvas to native changes the UI shell layout model. Start native, keep readback as a fallback flag.

---

## 20. MVP implementation order

Each phase ends with something runnable and tested. No AI until phase 4, on purpose.

**Phase 0: the core, headless (2–3 weeks)**
- `core-time`, `core-ids`, `core-model`, `core-commands`, `core-query`, `core-project`.
- All commands from §11, invariants, undo/redo, journal replay, CBOR blob, migrations scaffold.
- Property tests: random command sequences never violate invariants; undo(apply(x)) == x; replay hash equality.
- `cli`: `project new/import/apply/export-json`. CI green on three OSes.

**Phase 1: media + playback of one clip (3–4 weeks)**
- `media`: probe, frame index, decode with hwaccel-off path, audio decode; `ingest`: thumbnails, waveforms.
- `render`: Transform + Composite + ColorMatrix + input/output transforms; `audio`: mixer + cpal; `playback`: clock + prefetch.
- `app`: Tauri shell, native preview surface, transport controls, media bin. Open a file, play it.

**Phase 2: a real editor (4–6 weeks)**
- Timeline canvas UI: insert, move, trim (all modes), split, delete, ripple, A/V link, markers, snapping.
- Scratch-overlay drag/commit, undo UI, autosave + crash recovery, relink.
- `export`: H.264/HEVC via hardware, ProRes, audio AAC; proxy generation job; `jobs` crate with priorities and cancellation.
- At this point it is a usable basic NLE. Dogfood it.

**Phase 3: skills framework with no AI (2–3 weeks)**
- `skills-api`, registry, schema generation, `invoke_skill` command, provenance-driven inspector forms.
- Atomic skills: `video.transform`, `video.crop`, `video.speedRamp`, `video.freezeFrame`, `video.reverse`, `audio.normalize`, `audio.gain`, `transition.crossDissolve`, `graphics.text`.
- One composite skill without models: `timeline.assemble` (place a list of ranges with transitions). Everything invoked from menus. Verify the "UI has no editing logic" rule holds.

**Phase 4: analysis pipeline + Python service (4–5 weeks)**
- `proto`, `model-service-client`, Python service skeleton, analyzer registry on both sides, cache crate, `analysis-store`.
- Rust analyzers: shot_detect, frame_sample, audio_extract, media_signals.
- Python analyzers: embed_frames, range_scorer (embedding judge), transcribe.
- Candidate browser UI: scores per window, filter by criterion, drag candidates to timeline. Useful **without any LLM**.

**Phase 5: director + deep VLM + planner (4–6 weeks)**
- `ai-llm` with Ollama + OpenAI-compatible backends, structured output, model management UI.
- Director → Brief; judge driver; `multimodal.shot_understanding` on top candidates; `compare_shots`.
- Planner agent loop over query tools + skills with dry-run, step approval UI, per-step undo.
- First composite AI skill: `sportsHypeEdit` built from `timeline.assemble` + `video.speedRamp` + `audio.beatDetect` + `transition.*`.

**Phase 6: specialized skills, one at a time, each driven by a concrete feature**
- `segmentation.subject` → `graphics.textBehindSubject`; `tracking.subject` → `video.cinematicPunchIn`; `video.stabilize` (vidstab); `graphics.caption` (whisper words); `audio.duck`; `color.match`.
- Learned judge heads from user feedback once there are users.

Things deliberately not in the MVP: collaboration, plugin SDK for third-party skills (the registry supports it, but no public API promises yet), node-graph compositing, HDR export, cloud rendering.

---

## 21. Open questions to settle during Phase 1

- Native preview surface on Windows (WebView2 + child HWND z-order) vs. readback: prototype both in the first week of Phase 1 and pick.
- Hardware decode zero-copy into wgpu (D3D11 → DX12 interop, VideoToolbox → Metal): defer; measure whether CPU readback at proxy resolution is acceptable for MVP.
- Whether `ffmpeg-next` bindings are maintained enough or we wrap `ffmpeg-sys-next` ourselves (likely the latter for seek control).
- Python distribution: `python-build-standalone` + uv-locked venv in the app bundle, versus requiring a user install. Ship bundled; it is the only option that works for non-developers.
