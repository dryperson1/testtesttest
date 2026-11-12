# AI-native video editor

> **Working on this project? Start with [README-STAGES.md](README-STAGES.md) (what to do next) and [README-PROJECT.md](README-PROJECT.md) (how to interpret everything).** This file describes the crates, the app, the command line and the plug-in blocks.

This repository holds the editor described in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): a small permanent Rust core, with everything else built around it. Everything runs on the user's machine with free tools only; see [docs/LOCAL-FREE-DECISIONS.md](docs/LOCAL-FREE-DECISIONS.md).

The product is called ChadFX; what it should become is in [docs/PRODUCT.md](docs/PRODUCT.md). "Vep" is its older code name, which the command-line tool (`vep`), the app's package (`vep-app`), the `VEP_*` environment variables and the `.vep` project files still carry.

Status: Phase 2 of the roadmap (a real editor) is done, and so is the plug-in blocks rewrite: every effect, transition, blend mode and generator is now a block that anyone can write and install (see "Plug-in blocks" below). There is no AI in it yet; that starts with Phase 3 (skills) and Phase 4 (analysis).

## What is here

| Crate | Purpose |
|---|---|
| `core-time` | Exact rational time, frame rates, ranges, timecode |
| `core-ids` | Typed ULID identifiers and a deterministic ID factory |
| `core-model` | The immutable project model (effect nodes are a block id, a settings format and settings), invariant checker, time remapping, JSON codec with migrations (schema 2 projects are upgraded to 3 on reading), analysis and AI plan types |
| `core-commands` | Every edit as a validated, atomic command; undo/redo store. It checks structure only and knows no block |
| `core-edit` | Editing gestures (move, trim, ripple, roll, slip, slide, split, insert, speed) expanded into exact transactions, with linked picture and sound and sync lock |
| `core-query` | Read-only views and queries for the UI and the AI, including the AI's text of a timeline, which names blocks that cannot be used |
| `core-project` | The SQLite project file: checkpoints, crash-safe journal, blobs, analysis records, jobs |
| `blocks` | Plug-in blocks: manifests, the package reader, the shader and WebAssembly checkers, the registry of built-in and installed blocks, catalogue snapshots, settings migrations, and the checks every edit passes against the catalogue |
| `jobs` | Background scheduler: priorities, dependencies, cancellation, CPU and GPU leases |
| `cache` | Content-addressed store for derived files, with an index and size-bounded eviction |
| `media` | Probing, frame-exact decoding and seeking, packet index, encoder detection and encoding, through FFmpeg as a child process |
| `ingest` | Thumbnail strips, posters, waveform files and proxies |
| `render` | GPU renderer (wgpu): draws every picture block from the catalogue, in linear light, with the protections of untrusted blocks |
| `render-recipe` | One frame's drawing instructions as plain data |
| `audio` | Mixer: plays every sound block from the catalogue in WebAssembly, speed and reverse, streaming decode, speaker output with a playback clock |
| `playback` | Streaming frame source with decoder pooling, proxies and caching; transport state |
| `export` | Frame-exact render and encode of a sequence to a finished file, with the list of blocks it leaves out |
| `cli` (`vep`) | Headless command-line driver for scripting and tests, including `vep blocks` |
| `golden` | Reference pictures and sounds made by the editor before the blocks rewrite, which prove nothing changed (tests only) |
| `cpu-render` | A software renderer that interprets the real shaders with exactly defined arithmetic, to prove pictures without a graphics card (tests only, never shipped) |
| `ui/` + `ui/src-tauri` | The desktop app: React interface in a Tauri window over the core |

Outside `crates/`: `blocks/builtin` holds the 17 blocks that come with the editor, `blocks/src` the Rust source of its sound blocks (a workspace of its own, compiled to WebAssembly), `blocks/template` five blocks to copy, and `blocks/block-manifest.schema.json` the published schema of a block's manifest.

## Requirements (all free)

- Rust (stable) through rustup; on Windows also the Microsoft C++ Build Tools.
- FFmpeg (`ffmpeg` and `ffprobe`) in a folder named by the `VEP_FFMPEG_DIR` environment variable, on the PATH, or in a common install folder (winget's included). The LGPL build from the BtbN project works and is what development uses.
- Only to rebuild the built-in sound blocks or the sound template: the `wasm32v1-none` target (`blocks/src/README.md`). Their compiled modules are checked in.
- Node.js LTS for the desktop interface. A portable copy lives in `%LOCALAPPDATA%\Programs\node\current` on the development machine; add that folder to the PATH or install Node normally.

## Building and testing

The Rust workspace:

```bash
cargo test --workspace
```

The interface, from the `ui` folder: type check and bundle, lint, unit tests, and end-to-end tests that drive the real app.

```bash
npm run build
```

```bash
npm run lint
```

```bash
npm test
```

```bash
npm run e2e
```

The end-to-end tests launch the real desktop program, so its window appears on screen while they run. [ui/e2e/README.md](ui/e2e/README.md) explains the harness, including the "dogfood" scenario that cuts and exports a short film and checks every frame of the result.

## Running the desktop app

From the `ui` folder, with Node on the PATH:

```bash
npm install
```

```bash
npm run tauri dev
```

What the window offers:

- **Projects:** a welcome screen with recent projects. Every edit is saved to the project file as it happens, so a crash loses nothing; reopening tells you what was recovered.
- **Media bin:** import by button or by dropping files on the window. Posters, thumbnails, waveforms and optional proxies are made in the background. Missing files are flagged and can be relinked one at a time or by folder.
- **Timeline:** select, blade, ripple, roll, slip, slide and hand tools. Snapping, linked picture and sound, sync lock (ripple edits keep every track in step), markers, in and out marks with lift and extract, speed and reverse, cross dissolves, copy and paste, filmstrips and waveforms on clips.
- **Inspector:** the effects of the selected clips, of their tracks and of the whole sequence, and a clip's transitions, as controls built from each block's manifest, with the picture updating live while a control is dragged and one undo step on release. Effects whose block cannot be used keep their section, with why. Fit to frame and fill.
- **Blocks:** the block manager (the Blocks button, `Ctrl+Shift+B`) lists every block, installs one from a package file, a folder or a `.wgsl` file, turns blocks off and on, enables again a block the editor turned off because it misbehaved, removes installed ones, and lists what the open project cannot use.
- **Preview:** transport, J/K/L shuttle, loop, typed timecode, preview quality.
- **Export:** presets that say whether they work on this computer and why not, a range, size, frame rate and quality, run as a background job; also a still frame.
- **Jobs, history and settings panels.**

Press `?` in the app for the full list of keyboard shortcuts. The main ones:

| Keys | Action |
|---|---|
| Space, J / K / L | Play or pause, shuttle backwards / stop / forwards |
| Left / Right (Shift: ten) | Step one frame (ten frames) |
| Up / Down | Go to the previous or next cut |
| V, C, B, R, Y, U, H | Selection, blade, ripple, roll, slip, slide and hand tools |
| S or Ctrl+K | Split at the playhead |
| Delete / Shift+Delete | Delete leaving a gap / ripple delete |
| I, O, `;`, `'` | Mark in, mark out, lift, extract |
| M | Add a marker |
| N, Shift+L | Snapping on or off, linked selection on or off |
| `,` / `.` | Insert / overwrite the selected media at the playhead |
| Ctrl+Z, Ctrl+Shift+Z | Undo, redo |
| Ctrl+S, Ctrl+E | Save, export |

The window contains no editing logic: every gesture becomes one `core_edit::EditOp` sent to the same engine the command line uses and the AI planner will use.

## Using the core from the command line

`cargo run -p vep -- --help` lists every command. A few examples:

Build a sample project and print its timeline:

```bash
cargo run -p vep -- demo demo.vep
```

Import a real file:

```bash
cargo run -p vep -- import demo.vep C:\footage\clip.mp4
```

Apply an editing gesture, here a ripple delete, first as a dry run:

```bash
cargo run -p vep -- edit --dry-run demo.vep "{\"type\":\"delete_clips\",\"clips\":[\"<clip id>\"],\"ripple\":true}"
```

See which export presets work on this computer, then export:

```bash
cargo run -p vep -- presets
```

```bash
cargo run -p vep -- export demo.vep --out demo.mp4 --preset h264_mp4
```

`vep encoders` lists the video encoders that passed a real test encode here and why the others were rejected. `vep proxy`, `vep thumbs`, `vep waveform` and `vep poster` make derived files for one media file; `vep cache stats` shows the cache. `vep schema` prints the JSON schemas of every command and gesture, which is what the AI planner will be given.

## Plug-in blocks

Every effect, transition, blend mode and generator is a **block**: a manifest (`block.json`: name, description, kind, settings) and code, a WGSL shader for pictures or a WebAssembly module for sound. The editor's own 17 blocks (transform, crop, opacity, colour, mask, blur, glow, the blend modes, the normal blend, the cross dissolve, the wipe, solid colour, gain, pan, the crossfade, the tone and a three-band EQ) are in `blocks/builtin`, in exactly the format anyone else uses; the renderer, the mixer, the inspector and the AI's catalogue know no block by name. A block is installed from a `.chadfx-block` package file, a package folder or, for a picture block, one `.wgsl` file, with no rebuild of the editor. Everything in it is checked before anything of it runs, and a block that misbehaves is turned off with a reason. A project keeps a node of a block it does not have exactly as it was, says so in every list, and draws and plays without it.

**To write one, read [docs/BLOCKS.md](docs/BLOCKS.md)**, the guide for authors, and copy a block from `blocks/template`: a picture filter (also as one `.wgsl` file), a transition, a generator and a sound filter in Rust.

The `vep blocks` commands:

| Command | What it does |
|---|---|
| `vep blocks list` | Every block, built-in and installed, with its status |
| `vep blocks info ID` | One block: what it does, its settings, and why it cannot be used if it cannot |
| `vep blocks catalog` | Every usable block with its description, settings and roles, as the AI reads it |
| `vep blocks validate PATH` | Check a package folder, a `.chadfx-block` file or a `.wgsl` block without installing it, and explain every problem |
| `vep blocks pack PATH [OUT]` | Check a package folder or a `.wgsl` block and write it as one `.chadfx-block` file |
| `vep blocks install PATH` | Check and install a block for this user (`--replace` to replace another version) |
| `vep blocks remove ID` | Remove an installed block (`--project FILE` refuses while that project uses it, unless `--force`) |
| `vep blocks disable ID`, `enable ID` | Turn a block off and on |
| `vep blocks enable-again ID` | Use again a block the editor turned off because it misbehaved |

Installed blocks live in the `blocks` folder of the settings folder (`%APPDATA%\vep\blocks` on Windows; `VEP_CONFIG_DIR` chooses another settings folder), as `<id>.chadfx-block` files beside `state.json`. The built-in blocks are read from `blocks/builtin` beside the program, or from the folder `VEP_BUILTIN_BLOCKS` names; a debug build reads the repository's own folder.

## The Windows installer

From the `ui` folder, with Node on the PATH:

```bash
npx tauri build --bundles nsis
```

This builds the release app and `target/release/bundle/nsis/ChadFX_0.1.0_x64-setup.exe`. Only the NSIS installer is built (`ui/src-tauri/tauri.windows.conf.json` says so); the MSI one, and the WiX tools it needs, are not. Tauri fetches the NSIS tools into its own cache on the first build (the owner allowed that on 2026-11-05); later builds download nothing.

The installer installs for the current user, with no administrator rights. It puts the program in its folder with `blocks/builtin` beside it, adds a Start menu entry, a desktop shortcut (not with `/NS`) and an uninstall entry, and registers one file type: `.chadfx-block`, named "ChadFX Block", with the program's icon and an open command that quotes the program's path and the file's (`ui/src-tauri/windows/installer-hooks.nsh` mends Tauri's unquoted one). Double-clicking a block file then asks "Install *name* (*id*) by *maker*?"; a second file opened while ChadFX runs is handed to the running program. It registers nothing for `.wgsl`, so shader files keep opening in a code editor ("Open with" ChadFX still works). Uninstalling removes the program's folder, the shortcuts, the uninstall entry and the file type, with no empty `.chadfx-block` key left behind. One record stays unless "delete the application data" is ticked (a silent uninstall never ticks it): `HKEY_CURRENT_USER\Software\vep\ChadFX`, where Tauri's installer keeps the folder it installed to, so that the next install offers the same folder. These registrations have only been looked at from inside Claude's app container, which redirects registry writes; Explorer itself has not been seen to show them.

## Design rules enforced here

- Project state is immutable; a command is a pure function from one state to the next.
- Time is always a rational number of seconds, never a float.
- The project stores effects as a block id, the block's settings format and the settings, never as a closed list; higher-level operations are gestures and skills that expand into them. A node whose block is missing is kept byte for byte.
- Time and placement on the timeline belong to the core; what happens to pictures and sound is blocks ([docs/BLOCKS.md](docs/BLOCKS.md), section 10).
- Every change is checked against the catalogue of blocks at the edges (the app and `vep`) before it is committed; the engine itself never depends on which blocks are installed.
- Identifiers are supplied by the caller, so replaying a journal reproduces the same state byte for byte.
- Every committed transaction is journaled before memory changes; a crash loses nothing.
- Facts about one computer (whether a file is reachable, whether a proxy exists) never go into the project document.
