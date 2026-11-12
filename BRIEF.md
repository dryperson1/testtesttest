# The owner's brief for the plug-in blocks rewrite (verbatim)

Pasted by the owner on 2026-10-28 (local time). Not edited. It names the folder on the owner's computer; the project is now also this repository.

```text
Project: ChadFX, an AI-native desktop video editor at C:\Users\Mauro\Documents\CLAUDESTART
(Rust workspace in crates/, Tauri + React app in ui/). Read first: docs/PRODUCT.md (what the
owner wants; it wins over other docs on WHAT to build), README.md, docs/ARCHITECTURE.md
(sections 11 and 12 and every "implementation notes" section), docs/LOCAL-FREE-DECISIONS.md,
and the lib.rs docs of core-model, effects-registry, core-commands, core-edit, core-query,
render, audio, export and ui/src-tauri/src/service.

GOAL
Turn effect building blocks into plug-ins. Anyone, including outsiders, must be able to add a
new effect building block by installing a block package into an already-built app, with no
change to the core and no rebuild. The renderer, the audio mixer, export, the inspector and
the AI must all pick the new block up automatically. Every effect that exists today must be
moved onto this same system; when you are done nothing about any specific effect is
hard-wired in the core, the renderer, the mixer or the interface.

WHY
Today the effect types are a closed enum (core_model::EffectKind), described by a hand-written
table (effects-registry), and the renderer (render/src/graph.rs, renderer.rs, shaders/) and
mixer know specific effects by name. Adding one touches about six places in the core. The
owner needs third parties to extend the editor safely.

WHAT A BLOCK IS (design direction; improve it if you find better, but justify changes)
- A block package: a folder or single archive with a manifest (stable namespaced id such as
  "chadfx.blur" or "acme.glow", a version, a display name, a plain-English description the AI
  planner reads, video or audio, its parameters with type, default, range, unit, whether
  keyframable, inputs: one for a filter, two for a transition) plus its processing code.
- Video blocks: WGSL shaders run by our renderer as one or more passes over textures in the
  renderer's working space (linear light, premultiplied alpha), with access to time, frame
  size and the block's parameters. Validate shaders (naga) before use and enforce limits on
  passes, textures and uniform sizes so a bad block cannot hang or crash the app.
- Audio blocks: a sandboxed WebAssembly module called by the mixer on sample buffers, with
  no file, network or system access and a time limit per call.
- Parameter changes between block versions are handled by declarative migrations in the
  manifest.
- Built-in blocks use exactly the same package format, shipped in a built-in folder; user and
  third-party blocks live in a user blocks folder. Installing means pointing the app at a
  package file (no online store; nothing may be downloaded).

RULES THAT MUST HOLD
1. The project stores effect nodes as "block id + version + parameters", never as a closed
   list. A project that uses a block this app does not have still opens, shows the block as
   missing in the interface and in the AI's view, renders without it, warns on export, and
   keeps the node byte-for-byte so nothing is lost when it is saved again.
2. Untrusted blocks cannot harm the computer or the user's files, cannot reach the network,
   and cannot crash or freeze the app; a failing block is disabled with a clear message.
3. Preview and export produce identical pictures and sound from the same blocks.
4. Undo, the journal, crash recovery and exact rational time keep working exactly as now.
5. Existing projects (document schema version 2) open in the new version with a tested
   migration that converts every old effect node to the matching built-in block, and render
   pixel-for-pixel the same (prove it with golden-image tests made BEFORE the change).
6. Keep the line between building blocks and core editing meaning: decide, document and
   justify which current things stay in the core because they are about time and placement
   rather than pixels or sound (for example time remapping and clip placement) and which
   become blocks (transform, crop, colour, mask, compositing and blend modes, dissolves and
   other transitions, gain, pan, and the catalogued-but-unimplemented blur, curves, LUT,
   text, EQ, compressor, time-stretch).
7. Free and local only: no paid services, nothing online. Blocks must work on Windows, Mac
   and Linux and on a mid-range laptop (16 GB, basic graphics card or Apple M1).

DELIVERABLES
- The block format and a written guide for outsiders (docs/BLOCKS.md): how to write, test,
  package and install a block, with a template folder.
- The runtime: registry, loader, validator, sandboxing, versioning and migrations, missing-
  block handling.
- Core, commands, gestures, queries, the generated typed contract (schemas/, ui/src/bindings)
  and the project migration reworked to use block ids.
- The renderer and mixer reworked to run blocks generically.
- Every existing effect re-implemented as a built-in block, plus at least: blur, a glow, one
  two-input transition other than the cross dissolve, and one audio block, as working examples.
- Interface: a block manager (install from file, list, remove, show errors and missing
  blocks); the inspector built entirely from block manifests with no effect-specific code.
- Command line: vep blocks list | install | remove | validate.
- Tests: unit, property and golden-image tests; an end-to-end test that installs a new block
  from a package file into the running app, uses it, exports, and checks the result; tests
  that a malicious or broken block is contained.

HOW TO WORK
- Before changing code, explain your design in plain English (what changes, why, which files)
  and WAIT for my approval: this decision is hard to change later. Mention anything in
  docs/PRODUCT.md that this plan conflicts with.
- Then work in stages, keeping the whole workspace building with zero warnings and every test
  passing at the end of each stage: cargo test --workspace, and in ui: npm run build,
  npm run lint, npm test, npm run e2e.
- Take a source backup before starting. Keep test media small, clean up temp files, and stop
  if free disk space on C: falls below 50 GB (the disk runs nearly full). Never end processes
  by image name; Node.js is a portable copy at %LOCALAPPDATA%\Programs\node\current that must
  be put on PATH first.
- Do not start the AI planner, skills or anything else from later phases.
- At the end, update README.md, docs/ARCHITECTURE.md (new implementation notes section) and
  docs/PRODUCT.md, and report in plain English what works, what does not, and what you
  measured.
```
