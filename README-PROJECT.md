# How to interpret the ChadFX project

Read this to understand what this repository is, what to trust, and what not to assume. For what to do next, see [README-STAGES.md](README-STAGES.md). Written 2026-10-30 by the main session from its knowledge of the work, in a hurry and without a separate fact-check: confirm names in the code before relying on them.

## 1. What this is

ChadFX is an AI-native desktop video editor, built from scratch. It is meant to be a public product that always runs on the user's own computer: chat-driven, with a full automatic first cut from an editable story-outline plan, sports and hype edits first, a minimal look, and simple and pro modes. `docs/PRODUCT.md` records this from an interview with the owner.

**How far it is:** Phases 0 to 2 are finished, which means a real editor with a Rust core, timeline, media bin, inspector, export, jobs, history and settings, plus a command line. **The AI features of the product are not built.** Do not read the name "AI-native" as meaning they exist.

**Names:** ChadFX is the product. "Vep" is the older code name, which survives in the `vep` command-line tool, the `vep-app` backend package, `VEP_*` environment variables and the `.vep` project extension. They are the same project.

## 2. The most important warning: the rewrite has just finished

The plug-in blocks rewrite is finished (Stages 0 to 6, tag `stage-6-done`, 2026-11-06); [README-STAGES.md](README-STAGES.md) says what it built and what is next, and [docs/blocks-rewrite/FINAL-REPORT.md](docs/blocks-rewrite/FINAL-REPORT.md) says in plain English what works, what does not and what was measured. Every effect is a block: the renderer draws every picture block from its manifest and shader, and the mixer plays every sound block from its WebAssembly module, with no block names in their code. The app has a block manager, an inspector built from manifests (for a clip's, a track's and the sequence's effects), a sound thread, the `.chadfx-block` file type with a Windows installer, and blocks in one `.wgsl` file. Outsiders write blocks from [docs/BLOCKS.md](docs/BLOCKS.md) and `blocks/template`.

- **Some behaviour is knowingly incomplete or unproven.** Everything was proven on Windows on one computer only: never on a Mac, Linux, another graphics card or a mid-range laptop. The Mac path of "Open with" has never been built. A packaged app and a release `vep` draw and play correctly only with `blocks/builtin` beside the program, as the installer puts it; a bare copy says the folder is missing. The owner has decisions open (the final report's list), among them the sound fuel price and the EQ's defaults. Spec 26's "Left open" lists the rest.
- **Some documents are behind.** Anything saying `effects-registry`, `EffectKind`, `TransitionKind`, `GeneratorKind`, schema 2 as current, or "not a git repository" predates this work. The `effects-registry` crate is deleted; read it at git tag `baseline-before-blocks`.

## 3. Which document wins

| Document | Purpose | How to treat it |
|---|---|---|
| `docs/PRODUCT.md` | What the owner wants | **Wins on WHAT to build.** Check plans against it |
| `docs/blocks-rewrite/BRIEF.md` | The owner's brief for the rewrite, verbatim | Binding for the rewrite |
| `docs/blocks-rewrite/BLOCKS-SPEC.md` | The technical contract | Binding on technical detail. Sections 17 to 26 override earlier text. Where a rule cannot work: say so with evidence and propose the smallest change, never silently diverge |
| `docs/blocks-rewrite/DESIGN-APPROVED.md` | The design the owner approved, and the four decisions | Background to the contract |
| `README-STAGES.md` | Current status and next steps | Current as of its date |
| `docs/ARCHITECTURE.md` | Founding design, with dated implementation notes per phase | Good for WHY. Its section "Plug-in blocks implementation notes (2026-11-06)" describes blocks; older sections that name effect kinds predate them |
| `docs/LOCAL-FREE-DECISIONS.md` | Choices made to stay free and local | Trust; still applies |
| `docs/REVIEW-2026-10-29.md` | Review of the code against the product brief | A dated snapshot of findings; re-check before acting |
| `docs/BLOCKS.md`, `docs/blocks-rewrite/FINAL-REPORT.md` | The guide for block authors; the plain-English final report of the rewrite | The guide is pinned to the code by `crates/blocks/tests/docs.rs`; the report is a dated snapshot |
| `README.md`, `ui/README.md` | Crates, app, command line | Updated at Stage 6; code wins |
| `docs/blocks-rewrite/stages/*.md` | Each job's report | **Claims, not facts, unless a checker confirmed them.** History, never updated |
| `docs/blocks-rewrite/maps/*.md` | Research from before the rewrite | Good reasoning, but line numbers are stale |
| `crates/golden/README.md` | How to use the references | Trust and follow |

When code and a document disagree about what exists now, the code wins. When they disagree about what should exist, `PRODUCT.md` and the contract win. Tell the owner either way.

## 4. Constraints that are not negotiable

- **Free and local.** No paid services, no cloud model APIs, no accounts. The user's footage, projects and AI work never leave the computer. The app itself may go online only for a one-time model download, updates, one-time activation if ever sold, and crash reports the user agrees to send. Never suggest hosted APIs or paid SDKs.
- **Untrusted blocks cannot do harm.** No files, no network, no system access; they cannot crash or freeze the app.
- **Preview equals export,** for pictures and sound.
- **Old projects keep opening** and look and sound the same.
- **Exact rational time, undo, the journal and crash recovery** do not change.
- **Windows, Mac and Linux, on a mid-range laptop.** Note the work so far has only been run on Windows.

## 5. The repository

| Path | What it is |
|---|---|
| `crates/core-time` | Exact rational time |
| `crates/core-ids` | Identifiers, including ids derived from position |
| `crates/core-model` | The project document, its JSON form, and the migration from schema 2 to 3 |
| `crates/core-commands` | Commands and transactions; checks structure only, and knows nothing about specific blocks |
| `crates/core-project` | The project file: checkpoint plus journal, undo, redo, crash recovery, replay of old journals |
| `crates/core-query` | Read-only views, including the text description of a timeline meant for the AI |
| `crates/core-edit` | Gestures: expands user intent into edit operations (for example `set_clip_speed`) |
| `crates/blocks` | The block system: manifests, package reader, registry, catalogue, edge validation, shader checker and composer, sound checker and runner |
| `crates/render` | Draws frames with wgpu |
| `crates/audio` | The mixer |
| `crates/playback` | Real-time playback |
| `crates/media` | FFmpeg tools, decoding, encoding |
| `crates/ingest`, `cache`, `jobs` | Bringing media in, stored results, background work |
| `crates/export` | Writes finished videos and reports missing blocks |
| `crates/cli` | The `vep` command-line tool, including `vep blocks` |
| `crates/golden` | Frozen reference pictures and sounds, with their own frozen generators and no dependence on editor crates |
| `crates/render-recipe` | One frame's drawing instructions as plain data, used by the graphics card path and the software renderer |
| `crates/cpu-render` | A pure-Rust software renderer that interprets the real shaders with exactly defined arithmetic. Tests only, never shipped |
| `blocks/builtin` | The 17 built-in block packages, using the same format as outsiders' |
| `blocks/src` | Rust source of the sound blocks, a separate workspace compiled to WebAssembly |
| `blocks/block-manifest.schema.json` | The published manifest schema |
| `schemas/`, `ui/src/bindings` | The generated typed contract between backend and window |
| `ui/src-tauri` | The desktop backend, a headless service in `src/service` |
| `ui/src` | The React window |
| `ui/e2e` | End-to-end tests that drive the real app |
| `docs/` | Documents; `docs/blocks-rewrite/` holds the rewrite's papers |

## 6. How it works

**An edit.** A gesture in the window or the `vep` tool becomes a transaction. At the edge (the app or the command line), it is checked against the block catalogue by `blocks::check_transaction`, so both refuse the same things with plain-English messages. The engine then applies it, checking only structure, and it is recorded in the journal. Undo, redo and crash recovery replay from the checkpoint and journal. Nothing is committed before validation.

**Effects today.** An effect is an `EffectNode`: id, block id, version, enabled flag, parameters, legacy mask and blend, note, provenance. Transitions and clip sources (`Asset`, `Block`, `Adjustment`, `Silence`) use block ids too. The time remap stays a core node, `core.time_remap`, at the same list position, so old journals still derive the same ids.

**A frame.** The renderer folds a layer's nodes into one draw, using today's arithmetic, in linear-light premultiplied colour. One fused draw per layer is what keeps pictures byte-exact.

**Sound.** The mixer computes samples from clips and nodes, running each sound block in 128-sample slices on the sequence's own sample grid, so preview and export give the same samples. Keyframed volume and pan are smooth ramps within each slice (owner decision D2).

**Upgrading an old project.** Reading a schema-2 document migrates its JSON tree: kinds become block ids, kinds that drew nothing become `chadfx.v2.*`, transitions depend on track kind, and neutral colour nodes are appended with derived ids. Sequence-level nodes that were ignored before are written switched off with a note, so they do not suddenly start working. Old journal rows are treated the same way, and a checkpoint is written before new rows land.

**Core versus block.** Time and placement in the timeline are core; what happens to pictures and sound is a block. This is what lets outsiders add effects with no rebuild. Blocks are packages with a manifest (`block.json`) plus code: WGSL for pictures, validated with naga, with no bindings of their own and a loop budget; sandboxed WebAssembly for sound via wasmi, with no imports and a fuel limit per call. Built-ins use the same format.

**What the AI sees.** `core-query` provides views and `describe_timeline`, which name missing blocks honestly. The AI planner itself is deliberately not built.

## 7. How to read the evidence

- **Golden references are the proof that nothing changed.** They were made from the OLD code in Stage 0. **Never regenerate them or the frozen generators.** A difference is a defect to fix, not a tolerance to add. The Stage 0 generators were retired to `.rs.txt` and must stay retired.
- **Adapter sets.** Picture references exist for NVIDIA Vulkan and WARP DX12, debug and release. On a computer whose card has no stored set the test prints why and compares nothing, so a green result there proves nothing about pictures: read the output. On a computer whose card has a set, comparing nothing is a failure (`golden_must_compare.rs`).
- **The no-GPU picture proof.** `crates/render/tests/no_gpu_pictures.rs` compares all 351 scenes byte for byte with no graphics card, through `crates/cpu-render` and `crates/golden/data/render-portable`, and cannot skip. `same_card.rs` compares today's renderer with the frozen old shaders on the computer's own card. It has run only on Windows on this PC: Mac, Linux and ARM are designed for, not measured (spec 20).
- **Byte-exact** means exactly that, for pictures. For sound since Stage 4, the statement is at most 3 units in the last place of a 32-bit float of each sample's own value, or of the largest value added up where clips are summed, with the tone's and eq3's stated exceptions (spec 23).
- **Line endings.** `.gitattributes` keeps bytes exact. Removing it makes byte-exact tests fail on Windows for a reason unrelated to the code. This happened once.
- **Green tests are scoped.** `cargo test --workspace` (about 2,360 tests), `npm test` (567), `npm run e2e` (69, about 15 minutes, opens windows, needs Windows).
- **Reports are claims.** The pattern here is: builders report, reviewers attack, a checker verifies, then the main session verifies again. The review stage has repeatedly found serious defects that the builders' own tests missed.

## 8. Conventions

- `#![forbid(unsafe_code)]`; library code never panics on bad input. No build profile may set `panic = "abort"`, because the safety guards depend on unwinding.
- Plain-English doc comments on public items. Every rule gets a test that fails without it. Errors are written for people.
- Generated files: `schemas/` and `ui/src/bindings` come from `npm run bindings`; a staleness test catches drift. `blocks/block-manifest.schema.json` comes from `cargo run -p blocks --example write_schema`. Sound modules rebuild with `CHADFX_WRITE_SOUND_BLOCKS=1` on the staleness test.
- Do not edit by hand: golden data, fixtures, generated bindings, `block.wasm`.

## 9. Wrong assumptions to avoid

- "The `vep` name is a different product." It is the same project.
- "A green golden test on my computer proves pictures match." Only on an adapter with references. `no_gpu_pictures` is the one that proves it anywhere.
- "The software renderer's pictures are what a card draws." They are about as close to each real adapter as those adapters are to each other, not the same. It is a fixed yardstick, and only the bytes it was compared with are frozen.
- "The engine validates parameters." It does not any more. Validation is at the edges, so any new commit path must call `blocks::check_transaction`.
- "A block missing from the catalogue is an error." It is a normal state: the node is kept byte for byte and skipped.
- "`chadfx.v2.*` are real blocks." They are placeholders for kinds that drew nothing before the upgrade, and new packages may not use the id.
- "The neutral colour node on every clip is junk." It exists for byte-exactness (D1). Do not remove it.
- "`docs/blocks-rewrite/maps` line numbers are right." They predate the rewrite.
- "`README-STAGES.md` was fact-checked." It was written fast. Verify.
- "Everything must be rebuilt after deleting build folders." See the disk notes in `README-STAGES.md`.

## 10. Running it

Prerequisites and other-computer notes are in `README-STAGES.md`, section 2. From the repository root:

- `cargo build --workspace --all-targets`: builds everything; about 3 minutes from clean on the owner's computer.
- `cargo test --workspace`: all Rust tests; about 10 minutes.
- In `ui`, after `npm ci`: `npm run build`, `npm run lint`, `npm test`, `npm run e2e`, and `npm run tauri dev` to run the app.
- `VEP_CONFIG_DIR` and `VEP_CACHE_DIR` isolate settings and cache. `VEP_BUILTIN_BLOCKS` points at built-in blocks. `VEP_RENDER_ADAPTER=software` selects the software renderer.

The owner's computer: Windows 11, NVIDIA RTX A4000, Intel i9-13900, Node as a portable copy, a nearly full C: drive with a 50 GB floor.

## 11. How the owner works

The owner reads plain English, understands basic programming, and does not want unexplained terminal output. They decide scope and product direction; ask when a decision is really theirs, and otherwise choose sensibly and say so. Usage limits interrupt long runs, and they will say "continue". They care about the product being right more than about saving usage, but do not want usage wasted either.

## 12. Glossary

- **Block:** an installable effect package; id, version, manifest, code.
- **Manifest:** a block's `block.json`: name, description the AI reads, kind, typed parameters, stages.
- **Node:** one use of a block in a project (`EffectNode`).
- **Catalogue / registry:** the registry finds packages in the built-in and user folders; the catalogue is its immutable snapshot of what is usable.
- **Edges:** the app and command line, where changes are validated before committing.
- **Fold:** how several nodes of one block combine (none, last, or per parameter add, multiply or last). **Role:** what a parameter means to the host (opacity, scale, position, rotation, anchor, gain_db, pan).
- **Stage (in a block):** a function a picture block supplies (region, place, pixel, source, pass, blend, transition). **Stage (of the rewrite):** one of the seven steps, 0 to 6.
- **Loop budget / fuel:** limits on picture-block loops and on sound-block work per call.
- **Quarantine:** a block switched off after failing. **In-flight record:** a file written before GPU work, so that a block that crashes the driver is quarantined at next start.
- **Golden:** a frozen reference picture or sound. **Adapter set:** references for one GPU, API and build type.
- **Gesture:** a user-level edit. **Transaction:** a set of commands applied together. **Journal:** the recorded history. **Checkpoint:** a saved full state the journal builds on.
- **Neutral colour node:** a do-nothing colour node added by the migration so pictures stay byte-exact.
- **Schema:** the project format version; 3 is current, 2 is migrated on read.
