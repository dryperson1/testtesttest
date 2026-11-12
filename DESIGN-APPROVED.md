# The design as approved by the owner (verbatim)

This is the plain-English design put to the owner before any block work started, followed by the owner's reply. BLOCKS-SPEC.md is the technical contract that grew from it, and it wins on technical detail.

---

The design is ready for your approval. Nothing has been changed yet.

Two things first:

- **The review you asked for was cut off by the usage limit.** Only 2 of its 12 brief checks finished, and no code was touched. Both found small things best done early: there is no record of which clips you edited by hand, and there are the `.vep` naming and one-video-per-project rules. Neither touches effects.
- **Nothing in PRODUCT.md conflicts with plug-in blocks.** You recorded that decision there on 2026-10-28.

## The design

### What a block is
- **A package** is one file (`name.chadfx-block`, a zip archive) or, while developing, a plain folder. Inside is `block.json` (its description) and its code.
- **The description holds:**
  - an id like `acme.glow` and a version;
  - a name, and a plain-English description the AI reads;
  - its kind and inputs;
  - its parameters: type, default, range, unit, step, whether it can be animated, whether simple mode hides it, and labels per language;
  - rules for converting parameters from its older versions.
- **The app reads descriptions without running any block code.**

### Six kinds of video block
This part matters most. A researcher rebuilt today's renderer in a scratch program and tried 17 structures. Drawing each effect as its own step changed pixels by up to 72 shades. Only one structure kept every byte: each clip drawn in a single step. So blocks are pieces the app joins into that one step whenever it can:

- **Pixel blocks** (colour, opacity, masks): change one pixel's colour; joined into the clip's single step.
- **Placement blocks** (transform, crop): move the clip's corners; also joined into that step.
- **Neighbourhood blocks** (blur, glow): need nearby pixels, so they get their own steps, only when present.
- **Blend blocks:** how a clip combines with what is beneath it. Normal keeps the graphics card's built-in blending, as today.
- **Transition blocks:** take the outgoing and incoming clip. The cross dissolve is drawn exactly as today; wipes and pushes get their own step.
- **Source blocks:** make a picture from nothing (solid colour; later text).

A block only supplies functions. The app writes everything around them, so a block can reach nothing but the pixels and parameters it is handed.

A block can also declare how several copies of itself combine. Today two colour corrections on one clip are merged, and the last transform wins. The built-in blocks declare exactly those rules, which keeps old projects identical.

### Audio blocks
- **A small WebAssembly program run in a sandbox** (wasmi: pure Rust, free, identical on Windows, Mac and Linux).
- **Fixed slices.** It is handed 128 samples at a time, always cut at the same timeline positions, so preview and export sound the same.
- **No access to anything.** No files, network or clock. Memory is capped and each call has a work limit; a block that breaks a limit is switched off with a message.
- **Chains at every level.** Clips, tracks and the whole sequence each get a chain of audio blocks. That needs proper track and sequence mixing, which does not exist today.

### What stays in the core
- **About time and placement, not pixels or sound:**
  - where a clip sits;
  - linking, mute, solo, enable and track order;
  - adjustment layers;
  - how long a transition lasts and at which cut (its look is a block);
  - reading footage into the working colour space.
- **Speed and reverse also stay in the core.** They keep their place in the effect list under a reserved id, because saved edit histories identify effects partly by list position.
- **Everything else becomes a block:** transform, crop, opacity, colour, mask, blend modes, every transition, the solid-colour, colour-bars and tone generators, gain and pan.

### The project file
- **An effect is stored as "block id + version + parameters".** Format version 3, with a tested migration from version 2. Old edit histories are read through a compatibility layer, tested on a real old file.
- **The engine stops checking parameters against the catalogue while applying edits.** Today it re-checks even when replaying history on open, so removing or updating a block could stop a project from opening. Those checks move to before an edit is committed: in the app, the command-line tool, and later the AI.
- **A missing block** is kept exactly as stored. The inspector says "Missing block: Acme Glow 2.1 (not installed)", the picture and sound are made without it, export warns, and the AI's view lists it.
- **A newer version installed:** parameters are converted in memory using the block's own rules. The file changes only when you edit that effect.

### Keeping old projects identical (your rule 5)
- **A frozen copy of today's renderer and mixer** is kept as a test-only reference before anything changes. The new code is compared to it byte for byte, on the same graphics card, in the same test run. Golden images and sounds are also checked in.
- **What today's renderer ignores stays ignored:** masks attached to an effect, blend modes on effects, and transition kinds other than dissolve. All transitions draw as dissolves today, so they become dissolves, with the original kind kept as a note.
- **Effects that are catalogued but draw nothing today** (blur, curves, LUT, text, custom shader, EQ, compressor, time stretch) appear in old projects as "not available yet". They still draw nothing.

### Safety of untrusted blocks (rule 2)
- **Shaders are checked before use.** The graphics library's own validator runs with one fixed feature set. Blocks may not declare inputs or outputs of their own. A loop budget is inserted; prototyped here, it stops runaway loops and costs about 0.01 ms on a blur.
- **Compiling has a time limit, and so do frames.** A frame that runs long trips an alarm well under Windows' two-second graphics reset.
- **No blind retries.** The blocks in use are recorded on disk before each frame. After a graphics reset or crash they are switched off at the next start instead of retried; retrying could trigger a Windows blue screen after five resets in a minute.
- **Untrusted shaders are refused on the OpenGL fallback**, which has no memory-bounds protection.
- **Packages are never unpacked to disk.** Size, file count and compression ratio are limited.
- **Honest limit:** a hostile shader can still freeze the graphics card for about two seconds before the system resets it. The app survives and switches that block off.

### Interface, command line and AI
- **A block manager:** install from a file (or by dropping it on the window), list, remove, see errors and missing blocks.
- **The inspector is built only from block descriptions.** Today's hand-written effect lists and guessed units go away. Features like "fit to frame" find the right parameter through a declared role, such as "this is the scale".
- **Command line:** `vep blocks list | install | remove | validate`.
- **The AI is given the installed blocks at run time.**

### Built-in blocks
They ship in a built-in folder next to the app, in the same format:
- **Video blocks** are WGSL text.
- **Audio blocks** are Rust compiled to WebAssembly. The compiled files are checked in, with a test that catches stale ones.
- **New examples:** blur, glow, a wipe transition and a three-band EQ.
- **For outsiders:** a template folder and docs/BLOCKS.md.

## Decisions I need from you

1. **Clips with no colour correction.** Today's renderer runs a neutral colour calculation on every clip anyway; skipping it changes a few pixels per HD frame by one shade.
   - To stay byte-exact, the migration would add a neutral "Color" effect, at defaults and visible in the inspector, to every picture clip of old projects.
   - I recommend adding it. The alternative is no extra effect, accepting those one-shade differences.
2. **Animated volume and pan.**
   - Today they jump in steps that depend on how the sound is chunked, so preview and export already sound different, by up to 0.58 in level on a fade.
   - Making them identical (your rule 3) means changing them to smooth ramps. I recommend accepting that.
   - All other sound stays identical, apart from possible rounding of about one part in ten million in gain and pan, which I will measure.
3. **Text and captions.** The design leaves room for a text-layout service blocks can use, but building it is sizeable. I recommend doing it as the next task, straight after this one.
4. **The unfinished review.** I recommend running its 10 remaining read-only checks now, alongside the first stage, and its bug-fixing reviewers after this work.

## Files affected
- **New:**
  - `crates/blocks` (description format, package reader, validator, registry, versions);
  - `blocks/` (built-in blocks, their sources and the template);
  - `docs/BLOCKS.md`.
- **Removed:** `crates/effects-registry`, replaced by `crates/blocks`.
- **Core crates:**
  - core-model (effect.rs, timeline.rs, codec.rs, validate.rs);
  - core-commands (effect operations);
  - core-edit (speed);
  - core-query (views and the AI's description of the timeline);
  - core-project (old-history compatibility).
- **Picture and sound:**
  - render (graph.rs, renderer.rs, gpu.rs; the shaders become templates);
  - audio (mixer, mixing buses, sandbox);
  - export (missing-block warnings).
- **App and tools:**
  - ui/src-tauri (block service, install and remove);
  - ui/src (inspector, transitions, block manager, and one central file for the new interface text, as the brief now asks);
  - crates/cli;
  - regenerated schemas and bindings;
  - the end-to-end harness.
- **Docs:** README, ARCHITECTURE and PRODUCT.
- **New free dependencies:**
  - wasmi 2.0 (MIT/Apache) and zip 9.1 (MIT); the shader validator is already in the build.
  - One development download on this machine, `rustup target add wasm32v1-none`, free from the Rust project. App users download nothing.

## How it will run
Eight stages, each ending with zero warnings and every test passing:

0. Backup, plus golden images and sounds from today's code.
1. Block format, reader, validator, and `vep blocks validate` and `list`.
2. Project format and engine, with the migration and old-history compatibility.
3. The renderer running video blocks, proven byte-exact.
4. The mixer running audio blocks.
5. The new example blocks.
6. Block manager, inspector, install and remove.
7. Safety tests, the end-to-end install-and-export test, and the docs.

This is the biggest change since Phase 2. Expect several days of agent work.

**How it fits the brief:**
- descriptions carry labels per language;
- each parameter says whether simple mode hides it;
- install is from a file only, with nothing downloaded;
- blocks work in floating-point linear light, so HDR later won't change their format;
- everything new is keyboard-reachable;
- the record of which skill made an effect, which hand-edit protection will build on, is kept unchanged.

Reply "approved" with your answers to the four decisions, or tell me what to change.

---

## The owner's reply

> approved, #1 sure add it, #2 yes, change em to smooth ramps, 3. After this one. 4. Ok

Meaning: (1) yes, the migration adds a neutral colour node to every picture clip and sequence, for byte-exactness; (2) yes, keyframed volume and pan become smooth ramps, identical in preview and export; (3) text and captions are the next task after this one; (4) yes, the remaining product-brief checks ran alongside Stage 0, and the defect-finding reviewers run after the plug-in work.
