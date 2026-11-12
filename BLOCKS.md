# Writing blocks for ChadFX

This guide is for anyone who wants to add an effect, a transition, a blend or a generator to ChadFX, including people outside the team. It says how to write a block, check it, look at it, pack it, install it, update it and remove it, and what the editor does with it. Every rule here is the editor's own; when the editor and this guide disagree, the editor is right and the guide is wrong.

The examples come from the template folder, [`blocks/template`](../blocks/template). Start there: copy one of its five blocks and change it.

Contents:

1. [What a block is](#1-what-a-block-is)
2. [From a template to an installed block](#2-from-a-template-to-an-installed-block)
3. [The manifest](#3-the-manifest)
4. [Picture blocks: the WGSL contract](#4-picture-blocks-the-wgsl-contract)
5. [Sound blocks: the WebAssembly contract](#5-sound-blocks-the-webassembly-contract)
6. [Packages](#6-packages)
7. [When a block cannot be used](#7-when-a-block-cannot-be-used)
8. [The safety model](#8-the-safety-model)
9. [Every refusal, by rule code](#9-every-refusal-by-rule-code)
10. [The line between the core and blocks](#10-the-line-between-the-core-and-blocks)
11. [The `vep blocks` commands](#11-the-vep-blocks-commands)

---

## 1. What a block is

Every effect, transition, blend mode and generator in ChadFX is a block, including the ones that come with the editor. A block is two things:

- a **manifest**, `block.json`: what the block is called, what it does (in words the AI reads), what kind of block it is, and its settings, with their types, defaults and ranges;
- **code**: a WGSL shader for a picture block, or a WebAssembly module for a sound block.

The editor reads the manifest without running any code. It checks everything, code included, when the block is installed, and refuses a block that breaks any rule. After that:

- the renderer draws the block and the mixer plays it, in the preview and in exports, with the same result in both;
- the inspector builds the block's controls from its manifest;
- the AI's catalogue lists the block with its description and settings;
- a project stores each use of the block (a **node**) as the block's id, the settings format it follows, and its settings. Nothing else of the block goes into the project.

The 17 blocks that come with the editor are in [`blocks/builtin`](../blocks/builtin), in exactly the format described here. Their ids start with `chadfx.`, which only they may use.

### The seven kinds

| Kind | What it is | What its code gets | Code | Where it may be used |
|---|---|---|---|---|
| `video.filter` | Changes a clip's picture | One layer | WGSL stages `region`, `place`, `pixel`, and pass stages | The effects of a clip, a track or the whole sequence, on picture tracks |
| `video.transition` | Turns one picture clip into the next at a cut | The outgoing and the incoming clip | Style `fade` (no code) or `mix` (stage `transition`) | Only as a clip's transition in or out, on a picture track |
| `video.blend` | How a layer combines with the picture beneath | The layer and what is beneath | WGSL stage `blend` | The effects of a clip or a track, on picture tracks |
| `video.source` | Makes a picture from nothing (a generator) | Nothing | WGSL stage `source` | Only as a clip's source, on a picture track |
| `audio.filter` | Changes sound | The clip's sound | WebAssembly `vep_process` | The effects of a clip, a track or the whole sequence, on sound tracks |
| `audio.transition` | Turns one sound clip into the next at a cut | The outgoing and the incoming sound | WebAssembly `vep_transition` | Only as a clip's transition in or out, on a sound track |
| `audio.source` | Makes sound from nothing | Nothing | WebAssembly `vep_generate` | Only as a clip's source, on a sound track |

A node anywhere else (a sound effect on a picture clip, a filter as a transition) is not used, and every list says why (section 7).

What the editor itself keeps, and why, is in section 10: time and placement on the timeline stay in the editor; what happens to pictures and sound is blocks.

---

## 2. From a template to an installed block

### 2.1 What you need

- **`vep`**, the editor's command-line tool. It is not part of the installer: build it from the editor's source with `cargo build --release -p vep` (the program is `target/release/vep`, `vep.exe` on Windows). A release `vep` reads the editor's own blocks from `blocks/builtin` beside the program, or from the folder the environment variable `VEP_BUILTIN_BLOCKS` names: set it to the source's `blocks/builtin` folder, or copy that folder beside the program. A debug build of `vep` reads the source's folder by itself.
- **For a picture block:** nothing else. WGSL is plain text.
- **For a sound block:** Rust with the `wasm32v1-none` target for the compiler you build with: `rustup target add wasm32v1-none --toolchain stable`. Name the toolchain: inside the sound template's folder, its `rust-toolchain.toml` makes rustup add the target to the compiler it names instead (and download that compiler first). Section 5.7 says how to build.
- **To try it in the editor:** ChadFX itself.

`vep` keeps installed blocks in the editor's settings folder, the same one the editor uses: `%APPDATA%\vep` on Windows, `~/Library/Application Support/vep` on macOS, `$XDG_CONFIG_HOME/vep` (or `~/.config/vep`) elsewhere. To try things without touching your real settings, set the environment variable `VEP_CONFIG_DIR` to an empty folder first; both `vep` and the editor then use that folder instead.

### 2.2 Copy a template

| Template folder | Block | Kind |
|---|---|---|
| `blocks/template/picture-filter` | `example.warm`, "Warm tint": a number and a colour setting | `video.filter` |
| `blocks/template/picture-filter-lone` | `example.warm-one-file`: the same filter as one `.wgsl` file (section 4.10) | `video.filter` |
| `blocks/template/picture-transition` | `example.iris`, "Iris": a circle that opens onto the next clip | `video.transition` |
| `blocks/template/picture-source` | `example.gradient`, "Gradient": a two-colour gradient | `video.source` |
| `blocks/template/sound-filter` | `example.width`, "Stereo width", in Rust | `audio.filter` |

Copy one folder anywhere. Each has a short README that says which lines to change.

### 2.3 Change it

1. In `block.json`, give the block your own `id`: your name (or your organisation's) before the dot, such as `yourname.warm`. Ids starting with `chadfx.` or `core.` are refused.
2. Change `name`, `description`, `category`, `author` and `license`.
3. Change `params`, one entry per setting (section 3.4).
4. Change the code. Keep each stage function's first line exactly as it is: its name and the types it takes and returns are fixed (section 4.3).

Delete or repoint the `"$schema"` line: it lets a code editor check `block.json` against [`blocks/block-manifest.schema.json`](../blocks/block-manifest.schema.json), and the editor ignores it.

### 2.4 Check it

From the block's folder:

```text
vep blocks validate .
```

It runs every check installing runs, code included, and installs nothing. When the block is fine it prints its id, version, kind and identity (section 6.3). Otherwise it prints each problem as `problem N of M: where: what`, and exits with code 1:

```text
problem 1 of 1: block.wgsl:15:1: the pixel stage must be exactly fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> (any argument names; no attributes)
```

The manifest is checked first, and the code only once the manifest passes, so fixing one problem can reveal the next. `vep --json blocks validate .` gives the same as JSON, with each problem's rule code (`"code": "wgsl_stage_signature"`). Section 9 lists every rule code with what to do.

### 2.5 Look at it

Install it as it is (section 2.7 has every way):

```text
vep blocks install .
```

After each later change, install it again with `vep blocks install --replace .`: without `--replace`, installing another package of a block that is installed is refused (2.8).

Then make a small test project and draw a frame. A generator needs no media file, so the picture-source template is the quickest thing to look at. With the gradient installed as well (`vep blocks install path/to/picture-source`):

```text
vep new look.vep --name Look
vep seq add look.vep --name Main --width 640 --height 360
vep track add look.vep --kind video --name V1
```

`vep seq add` and `vep track add` print the new sequence's and track's ids on their last line. Put a gradient clip on the track with a file, `clip.json`, holding the clip, with the track id where `TRACK` is. A clip's and a node's ids are 26 characters (digits and capital letters except I, L, O and U, the first one 0 to 7); any unused ones will do:

```json
{
  "type": "insert_clip",
  "track": "TRACK",
  "clip": {
    "id": "01J00000000000000000000001",
    "source": { "type": "block", "node": { "id": "01J00000000000000000000002", "block": "example.gradient", "version": 1 } },
    "timeline_range": { "start": { "num": 0, "den": 1 }, "duration": { "num": 5, "den": 1 } }
  }
}
```

```text
vep apply look.vep --file clip.json
vep effect add look.vep --clip 01J00000000000000000000001 --block yourname.warm --set amount=1
vep export-still look.vep --at 1 --out frame.png
```

`vep export-still` writes one frame as a PNG; `vep render-frame look.vep --at 1 --out frame.ppm` writes a PPM image; `vep export look.vep --out look.mp4 --preset h264_mp4` writes a video (`vep presets` lists what works on your computer, and `--preset audio_wav` writes sound alone). `vep timeline look.vep` prints what the project holds. Times are seconds, `N/D`, `Nf` (frames) or a timecode.

To change a setting later, find the effect's id with `vep effect list look.vep --clip 01J00000000000000000000001` (it prints each effect's id and settings), then `vep effect set look.vep --effect EFFECT-ID --set amount=0.25` (a generator's settings the same way, with its node's id). A `--set` value is a number (`amount=0.25`); `true` or `false` (also `yes`, `no`, `on`, `off`); a choice's value as a bare word (`mode=fast`); `[x, y]` or `[x, y, z]` for a vector; `[r, g, b, a]` for a colour (`tint=[1,0.5,0,1]`), or the form `block.json` and `vep blocks info` use (`tint={"r":1,"g":0.5,"b":0,"a":1}`); `[[x, y], ...]` for a curve (a curve is not read in `block.json`'s `{"x": .., "y": ..}` form here); a shape as in section 3.4. Quote it as your shell needs.

The settings of a generator clip, and of a transition, are written in the command itself. The form differs from `block.json`'s: each value carries its type (section 3.4, "In a project"). The gradient, left to right:

```json
{
  "type": "insert_clip",
  "track": "TRACK",
  "clip": {
    "id": "01J00000000000000000000001",
    "source": { "type": "block", "node": { "id": "01J00000000000000000000002", "block": "example.gradient", "version": 1,
      "params": { "horizontal": { "value": { "type": "bool", "value": true } },
                  "end_color": { "value": { "type": "color", "value": { "r": 0.0, "g": 0.2, "b": 1.0, "a": 1.0 } } } } } },
    "timeline_range": { "start": { "num": 0, "den": 1 }, "duration": { "num": 5, "den": 1 } }
  }
}
```

For your own footage instead of a generator: `vep import look.vep yourfile.mp4` registers it and `vep clip add` places it (`vep clip add --help`). For a transition, add a second clip after the first and give it a transition in:

```json
{ "type": "set_transition", "clip": "01J00000000000000000000003", "edge": "in",
  "transition": { "block": "example.iris", "version": 1, "duration": { "num": 1, "den": 1 },
                  "params": { "softness": { "type": "float", "value": 0.1 } } } }
```

A transition's settings are never animated, so each is a plain typed value, without the `"value"` wrapper and the keyframes a clip's node has; leave `params` out to use the defaults.

A sound filter goes on a clip of a sound track (`vep track add look.vep --kind audio --name A1`), then `vep export look.vep --out look.wav --preset audio_wav` writes what you hear. The editor's tone generator, `chadfx.tone`, is a sound source that needs no file (a clip like the gradient's, with `"block": "chadfx.tone"`), but it sounds the same on both channels: to hear a stereo effect such as the width template, import a stereo recording instead. Animated settings are written with a `set_keyframes` command (section 3.6).

Every command that draws or plays says which nodes it left out and why:

```text
warning: left out: Warm tint (yourname.warm), an effect of clip 01J00000000000000000000001: this block is turned off in the block manager
```

In the editor, the same project opens like any other (`look.vep`), and the inspector shows your block's controls. The editor finds the blocks `vep` installed when both use the same settings folder (the same `VEP_CONFIG_DIR`, or neither sets it).

### 2.6 Pack it

```text
vep blocks pack . yourname.warm.chadfx-block
```

This checks the block and writes one `.chadfx-block` file to share. Without a file name it writes beside the folder, named after it (`vep blocks pack .` in `my-warm` writes `my-warm.chadfx-block` next to `my-warm`). Packing the same files always gives the same bytes (section 6.3).

### 2.7 Install it

Any of these installs a block for the current user, after checking all of it:

- **The block manager:** Blocks in the toolbar (or `Ctrl+Shift+B`), then "Install from file…" for a `.chadfx-block` file or a block's `.wgsl` file, or "Install a folder…" for a package folder.
- **A drop:** drop a `.chadfx-block` or `.wgsl` file on the editor's window. It is installed, never imported as media. While the block manager is open, a dropped folder is installed too.
- **"Open with" ChadFX:** after the Windows installer, double-clicking a `.chadfx-block` file opens ChadFX, which asks "Install *name* (*id*) by *maker*?" first. Nothing in the file runs before you answer. If ChadFX is already running, the file is handed to it and the question appears there. ChadFX never takes `.wgsl` files over (they open in your code editor), but "Open with" ChadFX on one asks the same question.
- **The command line:** `vep blocks install FILE-OR-FOLDER`. A running editor notices within a second. Like the window, `vep` reads a block's file or folder only from a drive or a network share (section 8.1): a path in Windows' device namespace (`\\.\NUL`, `\\?\GLOBALROOT\...`, a volume), on a server's `pipe` share, or a device name such as `NUL` typed as the path is refused at once, before anything is read, with exit code 1.

The installed package is kept as `<id>.chadfx-block` in the `blocks` folder of the settings folder (section 6.4). The file or folder you installed from is not touched, and the editor never reads it again.

A sound filter appears in the inspector's effects for sound clips, tracks and the sequence; a picture filter in those of picture clips, tracks and the sequence (a picture filter with only `region` or `place` stages does nothing at sequence level, and the sequence's menu leaves it out); a blend in those of picture clips and tracks; a transition among the transitions of a clip of its media; a generator wherever a clip's source is shown. The editor's window cannot yet put a new generator clip on a track: use `vep apply` as in section 2.5.

### 2.8 Update it

Raise `version` (for people) with every change. Raise `format` (for projects) whenever a setting is renamed, removed, or changes meaning, and add a migration that converts older settings (section 3.8).

Installing another package of an installed block replaces it, but only after you confirm:

- the block manager asks "Replace the installed block?", and says first how many effects of the open project use it, and whether some of them would need a newer version than the one you are installing;
- `vep blocks install --replace FILE` replaces; without `--replace` it refuses and says so. `--project FILE` makes it say what the editor says about a project.

Installing the very same package again changes nothing and says so. A block the editor turned off because it misbehaved (section 7) stays off until "Enable again" is pressed. That holds for the package that misbehaved, so installing a different package of the block usually lifts it (not when a crash record could not tell which package of the block was in use).

### 2.9 Turn it off, or remove it

- **Turn off and on:** the switch on the block's row in the block manager, or `vep blocks disable ID` and `vep blocks enable ID`. A turned-off block is not drawn or played anywhere, and it stays installed. Built-in blocks can be turned off too.
- **Enable again:** after the editor turned a block off because it misbehaved, "Enable again" in the block manager, or `vep blocks enable-again ID`, uses it again.
- **Remove:** "Remove" in the block manager, or `vep blocks remove ID`. The package is deleted from the editor's blocks folder. Projects keep every node of it unchanged; they are just not drawn or played until the block is installed again. When the open project uses the block, the block manager asks first; `vep blocks remove ID --project FILE` refuses while that project uses it, unless `--force`. Built-in blocks cannot be removed.

`vep blocks list`, `vep blocks info ID` and `vep blocks catalog` show what is installed (section 11).

---

## 3. The manifest

`block.json` is JSON in UTF-8 (a byte order mark is allowed). **Unknown fields are an error**, so a misspelt field is caught instead of ignored. This is the picture-filter template's:

```json
{
  "$schema": "../../block-manifest.schema.json",
  "manifest": 1,
  "id": "example.warm",
  "version": "1.0.0",
  "format": 1,
  "name": "Warm tint",
  "description": "Tints the picture towards a colour, warm by default. At full amount every colour is multiplied by the tint, so white becomes the tint itself.",
  "category": "colour",
  "kind": "video.filter",
  "params": [
    { "id": "amount", "type": "float", "default": 0.5, "min": 0.0, "max": 1.0, "step": 0.01,
      "unit": "fraction", "keyframable": true, "label": "Amount",
      "description": "How far the picture moves towards the tint: 0 leaves it exactly as it is, 1 multiplies it by the tint." },
    { "id": "tint", "type": "color", "default": { "r": 1.0, "g": 0.8, "b": 0.6, "a": 1.0 },
      "min": 0.0, "max": 1.0, "keyframable": true, "label": "Tint",
      "description": "The colour the picture is multiplied by, as it is picked (sRGB values from 0 to 1). Its alpha is not used." }
  ],
  "video": { "source": "block.wgsl", "stages": ["pixel"] },
  "author": "Example Author",
  "license": "MIT OR Apache-2.0"
}
```

### 3.1 The fields

| Field | Type | Required | Limits | Example |
|---|---|---|---|---|
| `$schema` | text | no | at most 200 bytes; ignored by the editor | `"../block-manifest.schema.json"` |
| `manifest` | whole number | yes | must be 1, the format of this guide | `1` |
| `id` | text | yes | section 3.2 | `"acme.glow"` |
| `version` | text | yes | a semantic version, major.minor.patch, at most 64 bytes | `"1.2.0"` |
| `format` | whole number | yes | 1 or more: the format of the settings, which projects store (section 3.8) | `2` |
| `name` | text (3.3) | yes | at most 80 bytes, one line | `"Glow"` |
| `description` | text (3.3) | yes | at most 2,000 bytes; line breaks allowed. The AI reads it | `"Soft light that spreads from the bright parts."` |
| `category` | text | no | at most 40 bytes, one line; groups blocks in menus | `"light"` |
| `kind` | one of the seven kinds | yes | section 1 | `"video.filter"` |
| `params` | list of settings | no | at most 64 (section 3.4) | |
| `fold` | `none`, `last` or `params` | no | filters only, and blends (always `last`) (3.5) | `"last"` |
| `default_for` | `placement` or `transition` | no | 3.5 | `"transition"` |
| `video` | object | picture blocks | 4.2 | `{ "source": "block.wgsl", "stages": ["pixel"] }` |
| `audio` | object | sound blocks | 5.4 | `{ "module": "block.wasm", "warmup_ms": 0 }` |
| `transition` | object | transitions | 4.7 (pictures), 5.5 (sound) | `{ "style": "mix" }` |
| `migrations` | list of `{ "from", "to", "steps" }` | no | at most 64, each of at most 64 `steps` (3.8) | |
| `author` | text | no | at most 200 bytes, one line; shown only | `"Acme"` |
| `license` | text | no | at most 200 bytes, one line; shown only | `"MIT"` |
| `homepage` | text | no | at most 200 bytes, starting with `https://` or `http://`; shown, never opened by the editor on its own | `"https://acme.example/glow"` |

A picture block has a `video` section and no `audio` section; a sound block the reverse. A fade transition is the one block with neither code nor `video` section.

### 3.2 The id

`namespace.name`, at most 64 bytes, lower case:

- the namespace (before the first dot): letters `a` to `z`, digits, `_` and `-`, starting with a letter or a digit;
- the name (after the first dot): the same, plus dots.

Taken: `chadfx.` (the editor's own blocks), `chadfx.v2.` (effects of older projects that have no block yet) and `core.` (the editor's own nodes). The namespace may also not be a name Windows keeps for a device (`con`, `prn`, `aux`, `nul`, `com0` to `com9`, `lpt0` to `lpt9`), because the installed package is kept as a file named after the id.

**Never change the id of a block people use.** Projects find a block by its id; a block under a new id is another block.

### 3.3 Text

`name`, `description`, every `label` and every setting's `description` are text for people: either a plain string, which is English, or an object from language tag to text that includes English, such as `{"en": "Glow", "fr": "Lueur"}` (at most 64 languages; a tag is two or three lower-case letters, then any number of `-` parts of one to eight letters or digits, such as `pt-BR`). The editor shows English for now. Text may not be empty and may not hold control characters; only descriptions may hold line breaks.

The editor shows all of a package's text as plain text: no HTML, no Markdown, and characters that change how the text after them is shown (right-to-left overrides and the like) are written as visible codes, with the block's id beside its name.

### 3.4 Settings (`params`)

Each setting is an object:

| Field | Type | Required | Rules |
|---|---|---|---|
| `id` | text | yes | `[a-z][a-z0-9_]*`, at most 32 bytes, unique in the block, not a word WGSL keeps (such as `loop` or `filter`), not starting with `vep_`. **Projects store settings by id: never rename or remove one without raising `format` and adding a migration.** |
| `type` | a type below | yes | |
| `default` | a value of that type | yes | within `min` and `max`; a node without the setting has this value |
| `min`, `max` | a number, or one number per component | no | numbers, vectors, colours and curves only (a curve's two are for x and y); `min` at most `max`; inclusive |
| `step` | number | no | more than 0; numbers, vectors and colours only; the inspector's step |
| `unit` | a unit below | no | numbers, vectors and colours only; how the number is shown. The stored value is what the code reads |
| `options` | list of `{ "value", "label" }` | for `enum` | 1 to 64 choices; `value` is 1 to 64 letters, digits, `_`, `.` or `-`, unique, and is what projects store; `label` is text (3.3) |
| `keyframable` | true or false | no | whether the setting can be animated. Left out: yes for `float`, `vec2`, `vec3`, `color`, `shape` and `curve`, no for the rest. A transition's settings are never animated, so `true` is refused there |
| `label` | text (3.3) | yes | at most 80 bytes |
| `description` | text (3.3) | no | at most 1,000 bytes; the AI reads it |
| `advanced` | true or false | no | hidden in the inspector's simple mode |
| `hidden` | true or false | no | never shown; kept on the node |
| `role` | a role below | no | 3.5 |
| `fold` | `add`, `multiply` or `last` | with block `fold: params` | 3.5 |
| `components` | list of labels | no | for `vec2`, `vec3` and `color`: one label per component, 1 to 16 bytes each (the defaults are X, Y / X, Y, Z / R, G, B, A) |

The types, with the JSON a `default` in `block.json` writes, and what code sees:

| `type` | JSON value | WGSL `Params` field | Sound value(s) |
|---|---|---|---|
| `float` | `12.5` | `f32` | 1 |
| `int` | `3` | `i32` | 1 |
| `bool` | `true` | `u32`, 0 or 1 | 1 (0 or 1) |
| `enum` | `"gaussian"` (an option's value) | `u32`, the option's index | 1 (the index) |
| `vec2` | `[0.5, 0.5]` | `vec2<f32>` | 2 |
| `vec3` | `[1.0, 0.0, 0.0]` | `vec3<f32>` | 3 |
| `color` | `{"r": 1, "g": 0.8, "b": 0.6, "a": 1}` | `vec4<f32>`, r, g, b, a as stored | 4 |
| `shape` | `{"type": "rect", "x": 0, "y": 0, "width": 1, "height": 1, "corner_radius": 0}`, `{"type": "ellipse", "cx": 0.5, "cy": 0.5, "rx": 0.25, "ry": 0.25}` or `{"type": "path", "points": [[0, 0], [1, 1]], "closed": true}` | `VepShape` (4.1) | not passed |
| `curve` | `[{"x": 0, "y": 0}, {"x": 1, "y": 1}]` (x strictly rising) | not visible | not passed |
| `text` | `"Hello"` | not visible | not passed |
| `blob` | a key of data kept in the project | not visible | not passed |
| `artifact` | a key of data kept in the computer's cache | not visible | not passed |

**In a project**, and in the commands `vep apply` reads, a value carries its type: `{"type": "float", "value": 12.5}`, `{"type": "enum", "value": "gaussian"}`, `{"type": "color", "value": {"r": 1, "g": 0.8, "b": 0.6, "a": 1}}`, with the type names of this table (a `blob` is written `"dense_curve"`). A node's setting wraps that once more, because it may also hold keyframes (section 3.6): `"params": { "amount": { "value": { "type": "float", "value": 1.0 } } }`. A transition's setting is the typed value alone (section 2.5 shows both). A setting a node or a transition leaves out has its default.

A picture block may declare `curve`, `text`, `blob` and `artifact` settings only with `"hidden": true` (data kept on the node that its code cannot see yet). A colour is stored as people pick it: sRGB values from 0 to 1, as footage carries them; turn it into linear light with `vep_srgb_to_linear` before mixing it with pictures (the template does).

Units (`unit`): `px` (sequence pixels), `degrees`, `percent` (stored as 0 to 100), `fraction` (a share, stored as 0 to 1, shown as a percentage), `db`, `ms`, `seconds`, `hz`, `stops`, `multiplier` (1 leaves things as they are), `none`.

A value that comes from a project has been checked when it was written: its type, its range, and that it is a finite number. A file edited by hand can still hold anything. The editor then reads a whole number where a `float` is declared as that number, and uses the default for any other wrongly typed value, except a shape: a picture block's node whose `shape` setting holds something else is not drawn at all (section 7).

### 3.5 Roles, folding and defaults

**Roles** tell the editor what a setting means, so its own features need no knowledge of your block. Filters only, one setting per role, of the type shown:

| `role` | Type | Media | What the editor does with it |
|---|---|---|---|
| `opacity` | float | picture | Multiplies every opacity of every enabled node of a stack together, and with a fade transition's factor, into the layer's opacity. Your code does not apply it: the layer's alpha already has it |
| `scale` | vec2 | picture | "Fit to frame" and "Fill" change it, on the last block of the stack that has one |
| `position` | vec2 | picture | Placement |
| `rotation` | float | picture | Placement |
| `anchor` | vec2 | picture | Placement |
| `gain_db` | float | sound | Kept for automatic mixing later; the mixer does nothing with it today |
| `pan` | float | sound | The same |

**Folding** (`fold`) says how several nodes of one block in one stack combine. A clip's stack is its own enabled nodes followed by its track's.

- `none` (the default): every node runs, in stack order.
- `last`: only the last enabled node of the block runs. Blends always fold `last`.
- `params`: every enabled node of the block folds into one run, at the place of the first. Each setting then needs its own `fold`: `add` and `multiply` (numbers, vectors and colours) start from the default and combine in stack order in 32-bit floats; `last` takes the last node's value.

**`default_for`** marks a block the editor picks for something:

- `placement`: the block "fit to frame" adds when a stack has no `scale` role. Only the editor's own blocks may say it.
- `transition`: the block the "add a cross dissolve" gesture puts on a cut of its media. Only a transition may say it. When several usable blocks claim it, the editor's own wins, then the first by id, and the block manager notes the clash.

### 3.6 Keyframes

An animated setting is a list of keyframes on the node: clip-local times (0 is the clip's start) with a value each, and an easing (`hold`, `linear` or `bezier`). The editor works the value out at the moment it needs, in 64-bit floats, then hands it to your code in 32 bits. A picture block sees the value at the frame's time; a sound block sees the values at the start and the end of each slice of 128 frames, so it can change smoothly inside it (section 5.3). Your code never sees keyframes.

The window shows an animated setting as "Animated, N keys" and can make it one value again; it cannot add keys yet. `vep apply` with a `set_keyframes` command does:

```json
{ "type": "set_keyframes", "effect": "EFFECT-ID", "param": "amount",
  "keyframes": [ { "time": { "num": 0, "den": 1 }, "value": { "type": "float", "value": 0.0 } },
                 { "time": { "num": 2, "den": 1 }, "value": { "type": "float", "value": 0.5 }, "interp": { "type": "hold" } },
                 { "time": { "num": 4, "den": 1 }, "value": { "type": "float", "value": 1.0 } } ] }
```

Each keyframe's value is a typed value (section 3.4, "In a project"). Its `interp` says how the value goes from it to the next keyframe: `{"type": "linear"}` when left out, `{"type": "hold"}` (it stays until the next), or `{"type": "bezier", "out_tangent": [0.4, 0], "in_tangent": [0.6, 1]}`, the two inner points of the curve as in CSS's `cubic-bezier`. `vep effect list` prints the effect's id.

### 3.7 The other sections

- `video`: `source` (the WGSL file), `stages`, `passes`, `fixed_function` (section 4).
- `audio`: `module`, `warmup_ms`, `latency_samples`, `tail_ms` (section 5.4).
- `transition`: `style`, and for a fade the four curves `incoming`, `outgoing`, `outgoing_alone`, `incoming_alone` (section 4.7).

### 3.8 Formats and migrations

`format` is the version of your settings, as projects store them. Each node remembers the format it was written with. When you rename, remove, rescale or reinterpret a setting, raise `format` and add a migration from the old format:

```json
"format": 3,
"migrations": [
  { "from": 1, "to": 2, "steps": [ { "rename": { "from": "size", "to": "radius" } },
                                   { "scale": { "param": "radius", "by": 2.0 } } ] },
  { "from": 2, "to": 3, "steps": [ { "offset": { "param": "angle", "by": 90.0 } },
                                   { "enum": { "param": "mode", "map": { "box": "fast" } } },
                                   { "default": { "param": "quality", "value": 1.0 } },
                                   { "remove": "legacy_flag" } ] }
]
```

| Step | What it does |
|---|---|
| `rename` (`from`, `to`) | Gives a setting a new id; its value and keyframes move with it |
| `scale` (`param`, `by`) | Multiplies a number (float, int rounded, every component of a vector or colour), keyframes included |
| `offset` (`param`, `by`) | Adds to a number the same way |
| `enum` (`param`, `map`) | Replaces stored choices, keyframes included; values not in the map stay |
| `default` (`param`, `value`) | Gives old nodes that lack the setting this value |
| `remove` | Drops a setting from old nodes |

Migrations form one chain, oldest first: each starts where the one before ended (`from` of one is `to` of the one before), and the last ends at `format`. Each step must leave only settings the block declares, unless a later step removes them. The checks are in section 9.4.

What the editor does with them:

- A node of an older format is converted in memory, by the same code, for drawing, playing, the inspector and the AI. Every list shows the converted values with a note. The project file changes only when one of the node's settings is changed; turning it off or moving it keeps the stored format.
- A node of a newer format than the installed block is `needs_newer`; one the migrations cannot convert (no path, or a value of the wrong type, or a number too large to keep) is `cannot_convert`. Both are not used, and are kept (section 7).
- Replacing a block with one of a lower format makes newer nodes `needs_newer`; the editor warns before replacing (2.8).

---

## 4. Picture blocks: the WGSL contract

### 4.1 What you write

A picture block's code is WGSL, the shader language of WebGPU. You write **functions only**: the stage functions your manifest declares, and any helper functions, constants, structs and aliases they need. The editor writes everything else: entry points, bindings, the uniform buffer holding your settings, the textures, and the join with other blocks. Many blocks run in one draw, so the editor renames every name of yours with a prefix of its own; your names cannot meet anyone else's.

Before your code the editor puts its **prelude**. You may use these names, and no other name starting with `vep_` or `Vep`:

```wgsl
struct VepCtx {
    seq_size: vec2<f32>,     // the sequence's frame size, in sequence pixels
    layer_size: vec2<f32>,   // the layer's displayed size, in sequence pixels (a source's, the transition's and the sequence's: the frame size)
    time: f32,               // seconds since the clip's start, the time the settings were evaluated at (sequence level: sequence seconds)
    frame_rate: f32,         // the sequence's frames per second
    progress: f32,           // a transition's progress, 0 to 1; a clip in a transition sees it too; otherwise 0
    output_scale: f32,       // output pixels per sequence pixel (below 1 in a reduced preview)
};

struct VepShape {            // a shape setting
    kind: u32,               // 0 rectangle, 1 ellipse (a path comes as its bounding rectangle, kind 0)
    a: vec4<f32>,            // rectangle: x, y, width, height; ellipse: centre x, centre y, radius x, radius y (0..1 of the layer)
    b: vec4<f32>,            // rectangle: corner radius, 0, 0, 0; otherwise zeros
    bounds: vec4<f32>,       // the bounding rectangle, x, y, width, height, worked out by the editor in 64 bits
};

// (The functions' bodies are left out here.)
fn vep_srgb_to_linear(c: vec3<f32>) -> vec3<f32>;   // the sRGB curve, decoded
fn vep_linear_to_srgb(c: vec3<f32>) -> vec3<f32>;   // the sRGB curve, encoded
fn vep_luma(c: vec3<f32>) -> f32;                   // BT.709 weights

// Only pass, blend and transition stages may call these (they read pictures):
fn vep_sample(input: u32, uv: vec2<f32>) -> vec4<f32>;  // bilinear, level 0, clamped to the edge
fn vep_load(input: u32, px: vec2<i32>) -> vec4<f32>;    // one texel, the position clamped to the picture
fn vep_input_size(input: u32) -> vec2<f32>;             // the picture's size in texels
```

An input number that does not exist reads transparent black and has size 0.

### 4.2 Settings, and the `video` section

The editor generates a struct called `Params` from your manifest: one field per setting visible to shaders, named by its id, in manifest order, of the type in the table of section 3.4. Every stage gets it as its last argument, `p`. In the template, `p.amount` is an `f32` and `p.tint` a `vec4<f32>`.

The `video` section says what your code provides:

```json
"video": { "source": "block.wgsl", "stages": ["pixel"], "passes": [] }
```

- `source`: the WGSL file in the package: a plain file name ending in `.wgsl`, at most 64 bytes, no folders. Required when there are stages or passes, refused when there are none.
- `stages`: the stage functions it provides. Which ones a kind may have: a `video.filter` any of `region`, `place` and `pixel` (or none, if it only has passes, or only an `opacity` role); a `video.transition` with style `mix` exactly `transition`; a `video.blend` exactly `blend`; a `video.source` exactly `source`.
- `passes`: pass stages, filters only (4.5).
- `fixed_function`: `premultiplied_over`, the graphics card's own blending. Only the editor's normal blend may say it.

### 4.3 The stages

| Stage | Exact signature | Runs | Colour in, colour out |
|---|---|---|---|
| `region` | `fn region(r: vec4<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | for each corner of the layer | `r` is the visible part of the layer as x0, x1, y0, y1 in layer uv (it starts as 0, 1, 0, 1); return the new one. A crop |
| `place` | `fn place(pos: vec2<f32>, luv: vec2<f32>, ctx: VepCtx, p: Params) -> vec2<f32>` | for each corner | `pos` is where the corner is so far, in sequence pixels (0, 0 top left, y downwards); `luv` is the corner's layer uv; return where it goes |
| `pixel` | `fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | for each pixel of the layer | linear light, **straight** (not premultiplied) alpha; `uv` is the layer uv (0, 0 top left) |
| `source` | `fn source(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | for each pixel of a generator's layer | return the colour **encoded as footage is** (sRGB values from 0 to 1) with straight alpha; the editor decodes it like footage |
| pass | `fn <name>(uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | for each pixel of the pass's output | **premultiplied** linear light; `uv` is the output's uv; read the inputs with `vep_sample` |
| `blend` | `fn blend(src: vec4<f32>, dst: vec4<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | for each pixel of the frame | `src` is the layer, `dst` what is beneath, both premultiplied; return the result, premultiplied |
| `transition` | `fn transition(a: vec4<f32>, b: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32>` | for each pixel of the frame | `a` the outgoing clip, `b` the incoming one, premultiplied, at the frame's uv; return the mix, premultiplied |

The argument names are yours; the types, their order and the result are not, and no attributes are allowed on them. A stage the manifest lists must exist with exactly this signature (`wgsl_stage_missing`, `wgsl_stage_signature`). `region`, `place`, `pixel` and `source` see one pixel or corner at a time and may not call `vep_sample`, `vep_load` or `vep_input_size`, even through a helper.

**How one layer is drawn.** The editor draws each clip's layer in one draw, every block of its stack joined in:

1. The corners: `r` starts as the whole layer; every `region` stage runs in stack order; each corner's `luv` follows from `r`; its `pos` is the editor's default placement (the layer centred at its displayed size), then every `place` stage runs in stack order.
2. Each pixel: the picture is read at `uv` (or your `source` stage runs), decoded from the sRGB curve into linear light, its alpha multiplied by the layer's opacity (every `opacity` role and any fade transition's factor); then every `pixel` stage runs in stack order; then the colour is premultiplied and laid over what is beneath (or handed to a blend block).

Values above 1 are allowed and are never clipped before the very end. **NaN and infinite values are not defined**: what a graphics card makes of them differs between cards, so keep your values finite.

**At sequence level** (a filter on the whole sequence), `pixel` stages run in the editor's output pass on the finished frame, which is **premultiplied**, with `uv` the frame's uv and `ctx.time` in sequence seconds; then the frame is clipped to 0 to 1 and encoded. `region` and `place` stages do nothing there.

### 4.4 Order in a stack

In a layer, every block's `pixel` stage runs in the layer's one draw, before any block's passes, whatever their order in the stack: so a mask followed by a blur draws exactly like a blur followed by a mask, and a mask's edge is always blurred. At sequence level the passes run on the frame first, and the `pixel` stages in the output pass after them. The inspector says when a block's stages run earlier or later than its place in the list suggests.

### 4.5 Pass stages

Neighbourhood effects (blurs, glows) read pictures around a pixel. They are pass stages: each is a draw of its own, declared in the manifest in the order they run:

```json
"passes": [
  { "name": "down", "input": "layer", "output": "half", "scale": 0.5 },
  { "name": "blur_h", "input": "half", "output": "tmp" },
  { "name": "blur_v", "input": "tmp", "output": "out" },
  { "name": "combine", "inputs": ["layer", "out"], "output": "result" }
]
```

- `name`: the pass's function, a name like a setting's id; not the name of a stage; unique.
- `input` (one picture) or `inputs` (one to four, never both): `layer` (the layer, drawn with its fused stages into a picture of the output's size) or an earlier pass's `output`. Input 0 is the first.
- `output`: the intermediate picture the pass writes: a name of your own (not `layer`, and not one the pass reads).
- `scale`: the intermediate's size as a share of the output size, 0.25 to 1 (default 1), set by the first pass that writes it.

At most 4 passes and 4 intermediates per block. Intermediates hold 16-bit floats, premultiplied. The intermediate called `result`, or else the last pass's output, is what is laid over what is beneath. Passes run at the output's size, so a setting in sequence pixels must be multiplied by `ctx.output_scale` in your code to look the same in a reduced preview and in the export. A filter with passes may also have `pixel` stages; they run in the layer's draw, before the passes.

### 4.6 Blends

A `video.blend` decides how a layer combines with the picture beneath it, in a clip's or a track's effects. The last usable blend of a stack decides; with none, the layer is laid over normally. In the `blend` stage, input 0 is the layer and input 1 a copy of what is beneath, both at the output's size, texel for texel: `src` and `dst` are those two at the pixel. A blend on the whole sequence does nothing (nothing is beneath it).

### 4.7 Transitions

A transition has one of two styles, in its `transition` section:

- **`fade`**: no code at all. The section gives four curves of opacity against progress, and the editor draws both clips with them. `incoming` and `outgoing` apply when both clips are there; `incoming_alone` (a fade in) and `outgoing_alone` (a fade out) when the other side has nothing. Each curve is 2 to 64 points `[progress, opacity]`, progress from exactly 0 to exactly 1 and strictly rising, opacity from 0 to 1, evaluated in straight lines between points. Every pixel of a clip gets the same opacity, so a fade can dissolve or dip, but cannot draw a shape. A fade has no `video` section. The editor's cross dissolve is a fade:

  ```json
  "transition": { "style": "fade", "incoming": [[0, 0], [1, 1]], "outgoing": [[0, 1], [1, 1]],
                  "outgoing_alone": [[0, 1], [1, 0]], "incoming_alone": [[0, 0], [1, 1]] }
  ```

- **`mix`**: both clips are drawn, each with its own effects, into pictures of the frame's size; your `transition` stage combines them pixel by pixel. Input 0 is the outgoing picture and input 1 the incoming one, for `vep_sample`; `a` and `b` are those at the pixel. `ctx.progress` runs from 0 to 1; return `a` at 0 and `b` at 1 so the cut lands exactly (the iris template does). With no clip on one side, that side is transparent. The result is laid over normally, or with the blend both sides share.

The editor decides which clip's transition applies at a cut, when it starts and how long it lasts; your block decides only how the two pictures mix. A transition's settings are never animated. A transition the editor cannot use is a cut.

### 4.8 Sources (generators)

A `video.source` makes a picture from nothing: a clip's source, in place of a media file. Its layer is the whole frame (`layer_size` is the frame size); its `source` stage returns colour encoded as footage is, with straight alpha; the clip's own effects then apply to it like to any picture.

### 4.9 Limits, and what is forbidden

Each limit has a reason: a block from anyone must not be able to hang, crash or slow down the editor, must give the same picture on every computer, and must be checkable by the editor's software renderer, which proves pictures without a graphics card.

| Rule | Why |
|---|---|
| Functions, constants, structs and aliases only: no module-scope variables, bindings, entry points, `override` constants, `enable`, `requires` or `diagnostic` directives | The editor owns every binding and variable; a block reaches nothing but its settings and the pictures it is given |
| No storage buffers, workgroup memory, atomics, barriers, subgroup operations, textures or samplers of its own, pointers outside the function's own variables, ray tracing | None of these exist for a block; several would let one pixel see or disturb others |
| No derivatives (`dpdx`, `fwidth`...), no `textureSample` with an implicit level, no `discard` | Their result depends on neighbouring pixels running in step; to drop a pixel, make it transparent |
| No recursion (WGSL has none) | Loops are then the only unbounded thing, and the loop budget bounds them |
| ASCII outside comments | Names and numbers mean the same everywhere; anything is fine in comments |
| At most 64 KiB of source and 64 functions | Checking stays fast |
| At most 2,048 operators in one statement, 256 `else if` in one function, 127 levels of nested blocks | Longer chains overflow the shader parser's stack, which would end the whole program |
| At most 16 KiB for one value (an array or a struct), module constants included, and 4 Mi 32-bit values in a whole draw | Memory on the graphics card stays bounded |
| A stage's straight-line cost at most 4,096 units, everything it calls included | Code without loops can still do exponential work through calls |
| The loop budget: 4,096 units per pixel, shared by every loop | See below |
| A whole draw's code at most 2^18 units; at most 64 blocks in one draw; at most 64 KiB of settings in one draw (less on a card that holds less) | A stack too large for one draw loses the blocks that do not fit, never the frame |
| No call to a maths function (`sin`, `pow`...) with constant arguments only; no `continuing` block reading a value a `continue` skipped; no function that can end without returning its value | The parser would work the first out itself with whichever maths library it was built with, so it could differ between computers; the others are undefined in WGSL and differed from run to run |
| No name starting with `vep_` or `Vep` other than the prelude's, and none that hides a WGSL built-in | They are the editor's |

**The loop budget.** Every loop of every block is given a counter by the editor. Each turn of a loop costs its body's units (1 for each expression or statement, 16 for each picture read, 1 more for each 16 bytes of an array or struct it moves, and a call costs the whole code it calls). When the 4,096 units of one pixel are spent, every loop stops. A loop that finishes within the budget computes exactly what it would without it; one that does not stops early and gives a wrong but harmless picture. Blocks fused into one layer share one budget per pixel, so a hostile block can spoil only its own layer's picture.

A block that passes every check can still be left out of a frame at run time (section 8.3).

### 4.10 A picture block in one `.wgsl` file

A picture block can also be one `.wgsl` file whose manifest sits in a comment at its top. The template `blocks/template/picture-filter-lone/warm-tint.wgsl` is the warm tint written this way:

```wgsl
/* chadfx-block
{
  "manifest": 1,
  "id": "example.warm-one-file",
  "version": "1.0.0",
  "format": 1,
  "name": "Warm tint in one file",
  "description": "Tints the picture towards a colour, warm by default. ...",
  "kind": "video.filter",
  "params": [ ... ],
  "video": { "stages": ["pixel"] }
}
*/

fn pixel(c: vec4<f32>, uv: vec2<f32>, ctx: VepCtx, p: Params) -> vec4<f32> {
    let tint = vep_srgb_to_linear(p.tint.rgb);
    let amount = clamp(p.amount, 0.0, 1.0);
    return vec4<f32>(mix(c.rgb, c.rgb * tint, amount), c.a);
}
```

The rules of the form:

- The file is UTF-8 (a byte order mark is ignored). Its first text, after any blank lines, is a block comment that opens with `/*` followed **on the same line** by the word `chadfx-block`. Everything after that word up to the comment's `*/` is the manifest, as JSON: the same object as `block.json`, with the same rules.
- Leave `source` out of the `video` section: the file is its own shader. A header that names any file (a source, a module, a README) is refused.
- Picture kinds only. A sound block needs a WebAssembly module, which a text file cannot hold.
- The JSON may contain neither `/*` (it would open a comment inside the header) nor `*/` (it ends the header early). A block whose texts need them is written as a package.
- One block per file; a second `/* chadfx-block` comment is refused.
- At most 64 KiB, header included (the shader's own limit, since the file is the shader).
- No icon, README or licence file can come with it. Make a package for those.

Install it, check it and pack it like a package: `vep blocks install warm-tint.wgsl`, `vep blocks validate warm-tint.wgsl`, `vep blocks pack warm-tint.wgsl` (which writes `warm-tint.chadfx-block` beside it). Installing keeps it as the very package `pack` writes: a `block.json` holding the header's manifest with `"source": "block.wgsl"`, and `block.wgsl` holding the file's bytes unchanged. So the file and its packed form are the same block with the same identity, and everything after installing is that of any package.

---

## 5. Sound blocks: the WebAssembly contract

### 5.1 The module

A sound block's code is a WebAssembly 1.0 module in binary form. The editor runs it in wasmi, a WebAssembly interpreter, with no SIMD, no 64-bit memory, one memory, no threads.

A module **imports nothing at all**: no functions, memories, tables or globals from the editor. It has no files, no network, no clock and no way to call the editor, and it brings its own maths (with Rust, the `libm` crate). It has **no start function**. It exports its memory as `memory`, and these functions with exactly these signatures (it may export other things, but no other name starting with `vep_`):

| Export | Signature | Kinds | What it does |
|---|---|---|---|
| `vep_abi` | `() -> i32` | all | Returns 1, the version of this contract |
| `vep_prepare` | `(sample_rate: i32, channels: i32, max_frames: i32) -> i32` | all | Called once per instance, before anything else; 0 means ready, anything else refuses the setup |
| `vep_io` | `() -> i32` | all | The byte offset in memory of the sound buffer: `max_frames * channels` interleaved f32 samples (frame 0 left, frame 0 right, frame 1 left...), read and written in place |
| `vep_io2` | `() -> i32` | transitions | The offset of the second buffer, the incoming clip's, the same size |
| `vep_params` | `() -> i32` | all | The offset of the settings: `2 * P` f32 values, the values at the slice's start, then the values at its end |
| `vep_reset` | `()` | all | Forget all state: the next call follows a jump in time |
| `vep_process` | `(frames: i32) -> i32` | `audio.filter` | Change `frames` frames of the sound buffer in place |
| `vep_transition` | `(frames: i32, offset: i32, full: i32) -> i32` | `audio.transition` | Mix the incoming buffer into the outgoing one; `offset` is the number of frames since the fade started, `full` the fade's length in frames |
| `vep_generate` | `(frames: i32, t0: f64) -> i32` | `audio.source` | Write `frames` frames into the sound buffer; `t0` is the clip-local time in seconds of the first frame |

The editor asks for the three offsets once, after `vep_prepare`. They must stay the same, lie inside the memory, and not overlap. A return value of 0 means success.

### 5.2 Slices

The mixer calls a block on **slices** of at most 128 frames (`max_frames` is 128), aligned to absolute sample positions on the sequence's timeline: slice k covers samples 128k up to 128k + 128. The first and last slice of a clip may be shorter. The sample rate and the number of channels are the mix's, which `vep_prepare` gives (normally the sequence's sample rate, in stereo: 2 channels, left first); a block should handle any channel count, if only by leaving the sound as it is. Because slices sit at the same places whatever plays them, the preview and an export call a block identically for the same stretch of time.

What a block is given:

- a filter: the clip's sound so far (after its source and the nodes before it in the stack). A track's filters run on each clip of the track, timed from that clip's start; the sequence's filters run on the mixed sound of every track, timed in sequence time.
- a transition: the outgoing clip's slice in `vep_io` and the incoming one's in `vep_io2`; write the mix into `vep_io`. With no clip on one side, that side is silence. The editor decides which clip's transition applies and its timing.
- a source: nothing; write the sound.

The editor never passes a sample that is not a finite number or is beyond plus or minus 2^20, nor a setting that is not finite. Every track's sound is added up and clipped to plus or minus 1 at the very end; values above 1 inside a stack are fine.

### 5.3 Settings

`P` is the number of f32 values your settings flatten to, in manifest order: 1 for a `float`, `int`, `bool` (0 or 1) or `enum` (the option's index), 2 for a `vec2`, 3 for a `vec3`, 4 for a `color`. `shape`, `curve`, `text`, `blob` and `artifact` settings are not passed. Before each call the editor writes the values at the slice's first frame, then the values just after its last frame (the next slice's first). A block that moves from the one to the other inside the slice changes smoothly; one that uses only the start values changes in steps of 128 frames. The template's `Slice::at` does the straight line.

### 5.4 The `audio` section: memory, warm-up and latency

```json
"audio": { "module": "block.wasm", "warmup_ms": 0, "latency_samples": 0, "tail_ms": 0 }
```

- `module`: the WebAssembly file, a plain file name ending in `.wasm`, at most 64 bytes.
- `warmup_ms`: how long the block needs to settle after a jump, in milliseconds, at most 4,000. A block with **no memory** between calls declares 0, and gives exactly the same samples in the preview and in an export. A block **with memory** (a filter with feedback, a delay) keeps it in its module's memory, forgets it in `vep_reset`, and declares its warm-up: after a jump (a seek, the start of an export, the start of a clip) the editor calls `vep_reset` and runs the block over that much sound before the point, discarding the result. It then agrees with uninterrupted playback once that time has passed (the editor's EQ agrees to about 1e-12, not bit for bit).

  **Keep `vep_reset` cheap.** It gets one slice's fuel (section 5.6), and a plain loop that clears memory spends it fast: measured with Rust, clearing 50,000 32-bit samples fits, 100,000 do not (half a second of stereo sound at 48 kHz). A block whose reset runs out of fuel is refused at install (`wasm_trial_failed`, "took too long in vep_reset"). So do not clear a long delay line there: reset its counters (where it writes next, and how much it has written since the reset) and read what it has not written since as silence. A few filter states, like the EQ's, can simply be set to zero.
- `latency_samples` (at most 192,000) and `tail_ms` (at most 30,000): the delay the block adds and how long it keeps sounding after its input stops. They are recorded only: this version of the editor neither compensates latency nor makes clips longer by the tail.

### 5.5 Sound transitions

A sound transition must have `"transition": { "style": "mix" }`; sound has no fade style. `offset` and `full` let a block work out where in the fade each frame is: the editor's crossfade multiplies the outgoing sample by `1 - u` and the incoming one by `u`, with `u = clamp((offset + i) / max(full, 1), 0, 1)` for frame `i` of the call.

### 5.6 Limits, fuel and faults

| Limit | Value |
|---|---|
| Memory when the module starts | at most 64 pages (4 MiB), the stack and all `static` data together. With Rust, link with `-C link-arg=-zstack-size=65536` (the template does): Rust's default stack is 1 MiB, and this leaves almost 4 MiB for statics (a 4,000,000-byte `static` array fits). A block that needs more grows its memory in `vep_prepare` |
| Memory it may grow to | 16 MiB per instance; growing beyond that fails (`memory.grow` returns -1) without stopping the block. Growing costs fuel, so grow in `vep_prepare` |
| All the instances of one mix together | 512 MiB. An instance that does not fit is not started yet, plays its fallback, and starts later when memory is free |
| Locals of one function | at most 1,024 (the interpreter clears them on every call without charging fuel) |
| Tables and element segments | at most 1,000 elements per table, and 1,000 function references in all element segments |
| Fuel per call | 100,000, plus 2,000 per frame per channel: 612,000 for a full stereo slice. `vep_prepare` and `vep_reset` get a full slice's fuel |
| The module file | at most 4 MiB, like any file of a package |

**Fuel** is how the editor limits a block's time: every instruction uses some, and copying, filling or growing memory use 1 per 64 bytes. It is counted the same on every computer, never by a clock. A call that uses up its fuel stops. The editor's own blocks use between 1% (gain) and 20% (tone) of a full slice's fuel.

**Faults.** A trap (reading outside memory, dividing an integer by zero, `unreachable`, a Rust panic), running out of fuel, a return value other than 0, or an output sample that is not a finite number turns the block **off for the rest of the session**, on every node of it: in the preview a session is the open project, and an export is a session of its own. The slice then plays the fallback: a filter passes its sound through unchanged, a source is silent, a transition is the editor's straight-line crossfade. The preview's list of block problems and the export's report name the block and what happened. A block that could not start (its module would not start, it refused the setup, its buffers were unusable, or the mix's memory was full) is reported as not started and plays the fallback.

**At install** every module is checked and then run once, as the mixer would, on silence at 48 kHz stereo with its default settings (`vep_abi`, `vep_prepare`, the offsets, one slice, `vep_reset`). A module that fails that is refused.

### 5.7 Building one in Rust

The template `blocks/template/sound-filter` is a sound filter in Rust that builds on its own:

- `src/lib.rs` is the block: a type implementing the `Filter` trait (`PARAM_VALUES`, `prepare`, `reset`, `process`), then `contract::filter!(Width);`, which writes the exports.
- `src/contract.rs` is the contract as safe Rust types: `Buffer` (the samples), `Slice` (frames, channels, `start`, `end` and `at` for settings), and the export macro. Leave it as it is.
- `Cargo.toml`, `.cargo/config.toml` and `rust-toolchain.toml` set the target (`wasm32v1-none`), the small stack and a small output.

Build it from its folder:

```text
cargo +stable build --release --locked
```

and copy `target/wasm32v1-none/release/block.wasm` over `block.wasm`. `+stable` uses your installed compiler; without it, rustup uses the one `rust-toolchain.toml` names and downloads it if it is missing. Delete `block.wasm.recipe` in your copy: it is for the editor's own test of the template.

In `src/lib.rs`, set `PARAM_VALUES` and the positions of your settings (`WIDTH` and `SWAP` in the template) in the order of `params`, then change `process`. A panic stops the call with a trap, which turns the block off. A block with memory keeps it in `static` atomic cells (they compile to plain loads and stores without threads), forgets it in `reset` without a long loop (section 5.4), and declares `warmup_ms`. The editor's own sound blocks, in `blocks/src` of the editor's source, are written with a fuller kit that also has transitions (`vep_io2`, `vep_transition`), sources (`vep_generate`) and state cells; `blocks/src/crossfade` and `blocks/src/tone` are a transition and a source to copy from.

Any language that compiles to WebAssembly 1.0 without imports works; Rust is what the editor uses.

---

## 6. Packages

### 6.1 The two forms

A **package** is a block on disk:

- a **folder**, while you make it;
- one **`.chadfx-block` file**, to share and install: a zip archive of the folder's contents (not of the folder itself: `block.json` is at the top of the archive).

```text
acme-glow/
  block.json      required: the manifest
  block.wgsl      a picture block's code (any name; the manifest names it)
  block.wasm      a sound block's module (any name; the manifest names it)
  README.md       optional: shown in the block manager, as plain text
  LICENSE         optional: shown in the block manager, as plain text
  icon.png        optional: shown in the block manager
```

The editor reads **only** these files: `block.json`, the code files the manifest names, `README.md`, `LICENSE` and `icon.png`, each from the top of the package, by exactly that name. Anything else in a folder (tests, build files, sources) is never read. In a `.chadfx-block` file nothing else is read either, but every entry must still keep the archive's rules below, so pack only what the block needs (`vep blocks pack` does). A picture block may also be one `.wgsl` file (section 4.10).

### 6.2 Limits

| What | Limit |
|---|---|
| A `.chadfx-block` file | at most 16 MiB, at most 64 entries |
| Each file read | at most 4 MiB, as declared and as actually read |
| All the files read together | at most 16 MiB |
| Compression | none (stored) or Deflate; each entry at most 100 times smaller than its contents |
| Entry names | UTF-8, at most 255 bytes, relative, `/` between folders, no `..`, `.` or empty parts, no drive, no control characters, no two that differ only in upper and lower case |
| Entries | plain files: no links, no encryption, no overlapping entries |
| A package folder | at most 4,096 entries at its top; the files it needs must be plain files (not links) with exactly the right names |
| `README.md`, `LICENSE` | UTF-8 text |
| `icon.png` | a PNG of at most 256 KiB. The block manager shows it when it is at most 512 by 512 pixels (a larger one is not shown); an animated PNG shows its first picture |

An archive is read in memory, never unpacked to disk. `vep blocks pack` writes archives that keep every rule.

### 6.3 Identity

A package's **identity** is the BLAKE3 hash of its archive's bytes, 64 hexadecimal digits; a folder's is that of the archive it packs into, which is always the same for the same files (names sorted, no compression, a fixed time and fixed permissions). `vep blocks validate`, `pack`, `install` and `info` print it. The editor uses it to tell two packages of one block apart: installing the same identity again changes nothing, a quarantine (section 7) holds for one identity, and a crash record names identities.

### 6.4 Where installed blocks live

Installed packages are kept in the `blocks` folder of the settings folder (section 2.1), each as `<id>.chadfx-block`, beside `state.json` (which blocks are turned off or quarantined, and why) and `in-flight.json` (the crash record, section 8.3). The editor's own blocks are folders in `blocks/builtin` beside the program (the installer puts them there).

A `.chadfx-block` file copied into that folder by hand is read like an installed one, within a second if the editor is running ("Read again" in the block manager reads at once), provided it is named after its block: `acme.glow.chadfx-block` must hold the block `acme.glow`. It goes through every check; one that breaks a rule is listed as broken, with why.

**Replacing** writes the new package over the old file in one step, so a crash leaves the old one or the new one, never half of either. One version of a block is installed at a time.

The editor reads at most **1,000 installed packages**: the first 1,000 files of the folder named `<id>.chadfx-block`, sorted by name. A package beyond them is not read at all: not listed, not drawn, not played, and the block manager and `vep blocks list` say so (`user_folder_too_many`, with how many packages the folder holds). Installing a new block into a folder that already holds 1,000 packages is refused with the same code, and nothing is written: remove a block you no longer use first. Replacing a block that is already installed still works.

---

## 7. When a block cannot be used

A project never loses a node because of its block. Whatever happens to the block, the node stays in the project exactly as it was saved, and works again when its block can be used again.

### 7.1 A node's status

Every node, wherever it is, has a status, the same in every list (the inspector, the block manager, the timeline, the export dialog and report, the AI's text, `vep`):

| Status | Meaning |
|---|---|
| `available` | Used |
| `core` | The editor's own speed node (section 10); always there |
| `missing` | No block with this id is installed |
| `needs_newer` | The node was made with a newer version of the block (a higher `format`) than the one installed |
| `cannot_convert` | The installed block's migrations cannot convert the node's settings; or a picture block's `shape` setting holds something other than a shape (only a hand-edited file can hold that) |
| `disabled` | The block is turned off in the block manager |
| `quarantined` | The editor turned the block off because it misbehaved, with the reason |
| `invalid` | The installed package breaks a rule; the block manager says which |
| `not_available_yet` | An effect of an older project that has no block yet (`chadfx.v2.*`) |
| `wrong_place` | The block is usable, but not where the node sits (a sound effect on a picture clip, a picture source on a sound track, a filter as a transition) |

A node is judged in that order: the block's status in the catalogue, then the place, then whether its settings convert.

### 7.2 What a node that cannot be used does

Nothing, in the preview and in exports alike:

- a filter is skipped;
- a source draws nothing, or plays silence;
- a transition is a cut;
- a blend gives way to an earlier usable blend of the stack, or to the normal one.

Where it is said:

- the **inspector** keeps the node's section with why, its stored settings read only, and the controls still allowed: switch, move and remove on a clip; switch and remove in a track's or the sequence's list;
- the **block manager** lists every node of the open project that cannot be used, with its clip and track;
- the **export dialog** lists, before you export, every node the export would leave out, and the export's **report** says so again afterwards;
- the **AI's description** of the timeline names it as missing;
- **`vep`** says it in a warning (`warning: left out: ...`), and `vep effect list` marks it.

A node that cannot be used may be moved, switched, reordered or removed, but its settings cannot be changed (`block_unavailable_params_locked`), so nothing is lost by accident.

### 7.3 A block's status

| Status | Meaning |
|---|---|
| `ok` | Usable |
| `invalid` | Its package breaks a rule (a package copied in by hand, or a rule of a newer editor) |
| `disabled` | Turned off by you |
| `quarantined` | Turned off by the editor because it misbehaved: the editor stopped unexpectedly or the graphics device was lost while it was in use, or its frames were far too slow (section 8.3). It stays off, across restarts, until "Enable again", or (usually) until another package of it is installed (2.8) |

A sound block that faults is not quarantined: it is off for that session only (section 5.6), and plays again in the next.

### 7.4 Problems while running

Blocks that run into trouble while frames are drawn or sound is mixed are counted and reported, like a damaged media file: in the preview's list of block problems and in an export's report. The kinds:

| Kind | Meaning |
|---|---|
| `not_guarded` | A block you installed was not drawn: what drew the frame had nowhere to keep the crash record |
| `refused_on_gl` | Not drawn: the graphics device works through OpenGL (section 8.3) |
| `not_recorded` | Not drawn in these frames: the crash record could not be written |
| `slow_compile` | Not drawn for the rest of the run: its program took more than 2 seconds to compile |
| `compile_failed` | Not drawn for the rest of the run: the graphics driver could not compile its program |
| `slow_frame` | Its part of these frames took longer than the slow-frame limit |
| `quarantined` | Turned off by the editor because its frames were too slow |
| `device_lost` | Turned off by the editor because the graphics device stopped working while it was in use |
| `left_out_of_draw` | Not drawn in these frames because the draw it belongs to could not hold it: its stack is too much for one draw (more than 64 blocks, more settings than the card's uniform buffer holds, more code than a draw may have), or the program of its own draw could not be made. The message says which limit. It is the stack's size, not the block's fault, so it never leads to a quarantine, and the editor's own blocks are counted too |
| `sound_trapped` | Off for the session: the sound block stopped with a trap |
| `sound_out_of_fuel` | Off for the session: it used up a call's fuel |
| `sound_error_code` | Off for the session: it returned an error code |
| `sound_not_finite` | Off for the session: it produced a sample that is not a finite number |
| `sound_not_started` | Not played: it could not be started |

### 7.5 Older and newer formats

- A node of an **older** format is converted by the block's migrations, in memory, everywhere; the project changes only when a setting of the node changes (section 3.8).
- A node of a **newer** format is `needs_newer` until a version of the block with that format is installed.
- A manifest of a newer **manifest format** (`"manifest": 2` one day) is refused by this editor with that reason.

---

## 8. The safety model

### 8.1 What a block cannot do

A block is untrusted code from anyone. The editor holds it to this:

- **No files, no network, no system.** A picture block's code gets its settings, a few numbers about the frame, and the pictures the editor binds for it; it declares no bindings or variables of its own. A sound block's module imports nothing, so it can compute only inside its own memory. Neither can read or write a file, reach the network, read a clock, or call the editor.
- **Nothing runs before it is checked.** A package is read in memory with every limit of section 6.2 and never unpacked to disk; its manifest is checked without running anything; its code is checked whole (sections 4.9 and 5.6) and composed with the editor's own code once, at install. The question a double-click asks shows only the manifest's text; nothing of the code is decoded before you answer.
- **It cannot hang or crash the editor.** Every loop of a picture block runs on a budget and every call of a sound block on fuel. The checkers run behind guards that turn any failure inside them into a refusal, the shader checker on a thread of its own with a stack large enough for the deepest code it accepts. A frame or a mix call never waits for a block (section 8.3).
- **It cannot disguise itself or take another's place.** Ids starting with `chadfx.` and `core.` are refused for user packages; a package must be named after its block in the blocks folder; an id is never used as a file path, and removing never deletes a file outside that folder. Text from a package is shown as plain text, with control and direction-changing characters as visible codes, names isolated and the id beside them; the icon is decoded by the editor, within size and memory limits, never by the window.
- **It is read only from a drive or a network share.** The window's ways of installing (the block manager, a drop, "Open with", a hand-off) and `vep blocks install`, `validate` and `pack` refuse paths in Windows' device namespace (`\\.\`, `\\?\GLOBALROOT`, volume paths) and a server's `pipe` share, because reading those can wait without end. They ask one rule, before anything is read; `vep` first makes a typed path whole as Windows would open it, so a device name such as `NUL` or `CONIN$` is refused too.

### 8.2 What the checks prove

The checks make a block's cost bounded and its result the same on every computer:

- picture code is limited in size, nesting, data and straight-line cost, its loops share a fixed budget, and anything the editor's software renderer cannot reproduce exactly is refused, so a block's picture can be proven without a graphics card;
- sound code is limited in memory, locals and tables, its time is fuel, and the same module with the same input gives the same bits on every computer.

### 8.3 What happens while it runs

For picture blocks you installed (the editor's own blocks are trusted, though they go through the same checks):

- **The crash record.** Before the editor hands a frame with such blocks to the graphics card, it writes their names and identities to `in-flight.json`, and clears them once the card is done. If the editor stops unexpectedly, or the graphics device is lost, the blocks the record holds are quarantined before anything is drawn again, and at the next start after a crash. A frame that held them is drawn again without them, never with them.
- **The slow-frame tripwire.** Each draw with such a block is timed on its own, by the card's clock where it has one. A block's share of a frame over 250 ms is a strike; three strikes in a row quarantine it. A share over 1 second, or one draw over 1 second, quarantines it at once, and the frame is drawn again without it. On larger pictures (a 4K export) draws are cut into bands the width of the picture and of at most 1920 × 1080 pixels in all (a 4K frame is four bands of 540 rows), each its own submission, so no submission comes near the graphics driver's watchdog; a block's share is counted as a full-HD band's worth, so the limits mean the same at every size. On the software adapter (no graphics card) the tripwire turns a block off for the run only, because every frame is slow there.
- **Compiling.** A program with such a block is made and compiled on a worker, each step held to 2 seconds; a slower one is left out for the rest of the run, and no frame waits for it. Programs are compiled ahead when a project opens.
- **OpenGL.** Such blocks are not drawn when the graphics device works through OpenGL, where a misbehaving shader can take the whole program with it.
- **A stack too large for one draw** (more than 64 blocks, more settings than the card's uniform buffer holds, more code than a draw may have) loses the blocks that do not fit, never the frame. Each node left out is named, once, with which limit: by the block manager for the frame on screen, in the preview's list of block problems and an export's report (`left_out_of_draw`, section 7.4), and by `vep render-frame`, `vep export-still` and `vep export`.

For sound blocks: fuel, memory and faults as in section 5.6. The player mixes sound on a thread of its own, so a slow block never holds up the picture or the transport; after play or a seek the picture waits at most 250 ms for the first sound.

### 8.4 What is accepted, and what is open

These are known and recorded; none of them lets a block reach files, the network or the system, or crash the editor.

**Time and cost**

- Fuel counts instructions, not time. Some instructions take longer than others for the same fuel: arithmetic on denormal numbers is about 9 times slower per unit of fuel than plain instructions, and calls of a function with 1,024 locals about 5 times. Measured on an i9-13900, the worst one valid node can do with one stereo slice's fuel is about 3.5 ms, against 0.4 ms for plain code; eight tracks of the worst valid block play 15.5 times slower than real time. That is slow, never a hang. Whether to price floating-point instructions higher or allow fewer locals is the owner's decision.
- Many different blocks, each under the tripwire, can still slow a frame together.
- Blocks fused into one layer share one loop budget, so a hostile block can spoil its own layer's picture.

**Results**

- NaN and infinite colours are not defined: a graphics card and the software renderer may draw them differently. Frames with them still come back whole.
- After a sound block faults it is off from the next call, so the preview and an export can differ for that block by up to one call. Near the mix's 512 MiB memory budget, which instances may grow depends on which are alive, so the two can differ there too.

**Containment**

- A sound block that faults in every session is never quarantined, because sound keeps no crash record (a record per call would cost one to two milliseconds of disk writing on the player's thread). No fault of a valid module reaches past the interpreter; the remaining risk is a defect inside wasmi itself. The interpreter's own stack is outside the 512 MiB budget, though bounded for each module.
- Folder packages could hold hard links, which cannot be detected safely on Windows; a `.chadfx-block` file cannot.
- One crash record per settings folder: two editors (or the editor and `vep`) using one folder are last-writer-wins.
- A compile still running when the editor closes normally leaves its record, and the next start quarantines that block: a false alarm, on the safe side. A frame planned just before another thread's quarantine can still use the block once.

**Not tried**

- Everything was proven on Windows, on an NVIDIA RTX A4000 (Vulkan, Direct3D 12, OpenGL) and the software adapter. A card that holds less than 64 KiB of uniforms was simulated, not tried. No Mac, no Linux.

### 8.5 How this is tested

Each protection has tests that fail without it: in the editor's source, `crates/blocks/tests` (packages, manifests, the shader and module checkers, `security.rs`, `garbage.rs`, `outside_text.rs`, `contained_picture.rs`, `contained_sound.rs`), `crates/render/tests` (`block_safety*.rs`, `contained_picture.rs`), `crates/audio/tests` (`block_safety.rs`, `block_cases.rs`, `block_memory.rs`, `contained_sound.rs`), `ui/src-tauri/tests` (`blocks_hostile.rs`, `contained_picture.rs`, `contained_sound.rs`, `contained_sound_player.rs`) and `crates/cli/tests` (`contained_picture.rs`, `contained_sound.rs`, `device_paths.rs`). The end-to-end tests in `ui/e2e` install the templates into the running app, use and export them, turn a picture block off after a simulated crash, and install hostile sound blocks (a trap, an out-of-fuel loop, and the costliest the fuel allows), which the preview names and the export leaves running.

---

## 9. Every refusal, by rule code

Every problem the editor reports about a block has a **rule code**. `vep --json blocks validate` shows it as `"code"`; the block manager shows the sentence and where. The codes are stable; the sentences may be improved.

### 9.1 Reading a package

| Code | What it means | What to do |
|---|---|---|
| `package_not_found` | The file or folder cannot be read | Check the path and that you may read it |
| `archive_too_large` | The `.chadfx-block` file is over 16 MiB | Make the package smaller |
| `archive_too_many_entries` | The archive has over 64 entries | Pack only what the block needs (`vep blocks pack` does) |
| `archive_damaged` | The archive is not a readable zip, or an entry is damaged | Pack it again with `vep blocks pack` |
| `archive_duplicate_name` | Two entries have the same name, or names that differ only in upper and lower case | Rename one |
| `archive_overlap` | Entries overlap one another (a zip-bomb trick) | Pack it again with `vep blocks pack` |
| `archive_name_not_utf8` | An entry's name is not UTF-8 | Use UTF-8 names |
| `archive_name_too_long` | An entry's name is over 255 bytes | Shorten it |
| `archive_unsafe_name` | An entry's name is absolute, names a drive, climbs out with `..`, has empty or `.` parts, a backslash, NUL or control characters | Pack with `vep blocks pack`, which writes plain names |
| `archive_symlink` | An entry is a symbolic link | Packages hold plain files only |
| `archive_encrypted` | An entry is encrypted | Pack without encryption |
| `archive_compression` | An entry uses compression other than none or Deflate | Pack with `vep blocks pack` |
| `archive_file_too_large` | An entry is over 4 MiB (an icon over 256 KiB) | Make the file smaller |
| `archive_ratio` | An entry would grow more than 100 times (a zip-bomb trick) | Store such a file uncompressed, or make it smaller |
| `archive_size_mismatch` | An entry holds a different size than it declares | The archive is damaged or made to mislead; pack it again |
| `manifest_missing` | There is no `block.json` at the top (or it is inside a folder of the archive) | Pack the folder's contents, not the folder (`vep blocks pack` does) |
| `manifest_not_utf8` | `block.json` is not UTF-8 | Save it as UTF-8 |
| `manifest_json` | `block.json` is not valid JSON or not a manifest (an unknown field, a wrong type) | Fix the place the message names; check spelling against section 3 |
| `code_file_missing` | The manifest names a code file the package does not have | Add the file, or fix the name in `video.source` or `audio.module` |
| `folder_too_many_entries` | The folder has over 4,096 entries at its top | Keep the block in a folder of its own |
| `folder_duplicate_name` | The folder has several files of one needed name in different case | Keep one |
| `file_name_case` | A needed file's name differs in upper and lower case | Use the exact name, such as `block.json` |
| `file_symlink` | A needed file is a symbolic link | Use a plain file |
| `file_not_regular` | A needed file is not a plain file | Use a plain file |
| `file_too_large` | A file is over 4 MiB (an icon over 256 KiB) | Make it smaller |
| `extra_not_text` | `README.md` or `LICENSE` is not UTF-8 text | Save it as UTF-8 |
| `icon_too_large` | `icon.png` is over 256 KiB | Make it smaller |
| `icon_not_png` | `icon.png` is not a PNG | Save it as PNG |
| `package_too_large` | The files read hold over 16 MiB together, or the packed archive would | Make the files smaller |
| `pack_failed` | The package could not be packed | Read the reason; usually a disk problem |

### 9.2 A block in one `.wgsl` file

| Code | What it means | What to do |
|---|---|---|
| `lone_file_too_large` | The file is over 4 MiB | A block's code is at most 64 KiB anyway |
| `lone_not_utf8` | The file is not UTF-8 | Save it as UTF-8 |
| `lone_no_header` | The file does not start with a `/* chadfx-block` comment: a shader but not a block | Add the header with the manifest (section 4.10), or make a package |
| `lone_marker_line` | The word `chadfx-block` is not on the same line as `/*` | Write `/* chadfx-block` on the comment's first line |
| `lone_header_not_first` | Other text or a comment comes before the header | Move the header to the top; only blank lines may come before it |
| `lone_header_unclosed` | The header is never closed | End it with `*/` after the JSON |
| `lone_header_nested_comment` | The header holds `/*` | Remove it, or make a package |
| `lone_header_cut_short` | The JSON ends before the header does, often because a text holds `*/` | Remove the `*/` from the text, or make a package |
| `lone_header_empty` | The header holds no manifest | Put the manifest's JSON in it |
| `lone_header_json` | The header is not a manifest (an unknown field, bad JSON), with line and column | Fix it; an icon, README or licence cannot come with a `.wgsl` file |
| `lone_second_header` | The file holds a second `/* chadfx-block` comment | One block per file |
| `lone_sound_kind` | The header's kind is a sound kind | Make a package with a `.wasm` module |
| `lone_names_file` | The header names a file (`video.source`, `audio.module`) | Leave `source` out: the file is its own shader |
| `lone_no_code` | The block would run no code (no stages or passes, a fade, a hardware blend) | Name its stages, or make a package of `block.json` alone |

### 9.3 The manifest's header, text and settings

| Code | What it means | What to do |
|---|---|---|
| `manifest_version` | `manifest` is not 1 (a newer editor's block, or a mistake) | Write `"manifest": 1`, or use a newer editor |
| `id_syntax` | The id is not `namespace.name` in lower case (also for a node naming such an id) | Section 3.2 |
| `id_reserved_core` | The id starts with `core.` | Use your own namespace |
| `id_reserved_builtin` | The id starts with `chadfx.` | Use your own namespace |
| `id_reserved_not_available_yet` | The id starts with `chadfx.v2.` | Use your own namespace |
| `id_builtin_namespace` | A block of the editor's own folder has an id not starting with `chadfx.` | Only for the editor's own blocks |
| `id_device_name` | The namespace is a Windows device name (`con`, `nul`, `com1`...) | Use another namespace |
| `version_too_long` | The version is over 64 bytes | Shorten it |
| `version_semver` | The version is not like `1.2.0` | Write major.minor.patch |
| `format_zero` | `format` is 0 | Formats start at 1 |
| `text_empty` | A text is empty or blank | Write it |
| `text_too_long` | A text is longer than its limit (section 3.1, 3.4) | Shorten it |
| `text_control_characters` | A text holds tabs, control characters, or line breaks where they are not allowed | Remove them; only descriptions may have line breaks |
| `text_missing_english` | A translated text has no `en` entry | Add English |
| `text_too_many_languages` | A text has over 64 languages | Keep 64 at most |
| `text_language_tag` | A language tag is not like `en`, `fr` or `pt-BR` | Fix the tag |
| `homepage_url` | `homepage` does not start with `https://` or `http://` | Fix it |
| `params_too_many` | Over 64 settings | Keep 64 at most |
| `param_id` | A setting's id breaks the rules (section 3.4) | Rename it |
| `param_id_duplicate` | Two settings have one id | Rename one |
| `param_default_type` | A default is not a value of its type | Section 3.4's table shows each type's JSON |
| `param_default_invalid` | A default is outside the range, not a choice, or a malformed shape or curve | Fix the default or the range |
| `param_range_type` | `min` or `max` on a type that has no range | Only numbers, vectors, colours and curves have ranges |
| `param_range_components` | A per-component `min` or `max` has the wrong number of values | One number, or one per component |
| `param_range_order` | `min` is greater than `max` | Swap them |
| `param_step_type` | `step` on a type that has none | Only numbers, vectors and colours |
| `param_step` | `step` is 0 or less | Make it more than 0 |
| `param_unit_type` | `unit` on a type that has none | Only numbers, vectors and colours |
| `param_components_type` | `components` on a type without components | Only `vec2`, `vec3` and `color` |
| `param_components_count` | Wrong number of component labels | One per component |
| `param_component_label` | A component label is empty, over 16 bytes, or holds control characters | Shorten or fix it |
| `param_options_type` | `options` on a setting that is not an `enum` | Remove them |
| `param_options_missing` | An `enum` has no options | Add at least one |
| `param_options_too_many` | Over 64 options | Keep 64 at most |
| `param_option_value` | A stored choice is not 1 to 64 letters, digits, `_`, `.` or `-` | Fix it |
| `param_option_duplicate` | A choice is listed twice | Remove one |
| `param_keyframable_transition` | A transition's setting says `"keyframable": true` | Transitions' settings are never animated |
| `param_role_kind` | A role on a block that is not a filter | Roles are for filters |
| `param_role_media` | A picture role on a sound block, or the reverse | Section 3.5 |
| `param_role_type` | A role on a setting of the wrong type | Section 3.5 |
| `param_role_duplicate` | Two settings have one role | Keep one |
| `param_fold_missing` | The block folds `params` and a setting has no `fold` | Give every setting `add`, `multiply` or `last` |
| `param_fold_type` | `add` or `multiply` on a setting that is not a number, a vector or a colour | Use `last` |
| `param_fold_unexpected` | A setting has a `fold` but the block does not fold `params` | Remove it |
| `param_must_be_hidden` | A picture block declares a `curve`, `text`, `blob` or `artifact` setting that is not hidden | Shaders cannot see it; make it `"hidden": true` |

### 9.4 The manifest's sections and migrations

| Code | What it means | What to do |
|---|---|---|
| `fold_blend` | A blend's `fold` is not `last` | Leave it out, or write `last` |
| `fold_kind` | `fold` on a block that is not a filter | Remove it |
| `default_for_kind` | `default_for` on a kind that cannot be that default | Section 3.5 |
| `default_for_origin` | `default_for: placement` on a block you installed | Only the editor's own blocks |
| `default_for_scale` | The default placement block has no `scale` role | Only the editor's own blocks |
| `video_missing` | A picture block has no `video` section | Add it (section 4.2) |
| `video_unexpected` | A sound block, or a fade, has a `video` section | Remove it |
| `audio_missing` | A sound block has no `audio` section | Add it (section 5.4) |
| `audio_unexpected` | A picture block has an `audio` section | Remove it |
| `audio_warmup` | `warmup_ms` is over 4,000 | Section 5.4 |
| `audio_latency` | `latency_samples` is over 192,000 | Section 5.4 |
| `audio_tail` | `tail_ms` is over 30,000 | Section 5.4 |
| `code_file_name` | A code file's name is not a plain name ending in `.wgsl` or `.wasm`, at most 64 bytes | Rename it |
| `code_file_missing_name` | Stages or passes but no `source` | Name the WGSL file |
| `code_file_unused` | A `source` but no stages or passes | Declare the stages, or remove `source` |
| `stage_duplicate` | A stage is listed twice | Remove one |
| `stage_not_allowed` | A stage the kind cannot have | Section 4.2 |
| `stage_missing` | A stage the kind needs is missing | Add it to `stages` |
| `fixed_function_kind` | `fixed_function` on a block that is not a blend | Remove it |
| `fixed_function_origin` | `fixed_function` on a block you installed | Only the editor's normal blend; write a `blend` stage |
| `fixed_function_with_code` | `fixed_function` with code, stages, passes or settings | It has none |
| `passes_not_allowed` | Passes on a block that is not a filter | Only filters have passes |
| `passes_too_many` | Over 4 passes | Keep 4 at most |
| `pass_name` | A pass's name breaks the rules of a setting's id | Rename it |
| `pass_name_stage` | A pass is named like a stage | Rename it |
| `pass_name_duplicate` | Two passes have one name | Rename one |
| `pass_inputs` | Both `input` and `inputs`, neither, or an empty list | Give one of them |
| `pass_inputs_too_many` | A pass reads over 4 pictures | Keep 4 at most |
| `pass_input_unknown` | A pass reads a picture that is neither `layer` nor an earlier pass's output | Fix the name or the order |
| `pass_output` | A pass writes `layer`, or a badly named picture | Give its output a name of its own |
| `pass_output_read` | A pass reads the picture it writes | Write another one |
| `pass_scale` | `scale` is outside 0.25 to 1 | Fix it |
| `pass_scale_changes` | A later pass gives an intermediate another scale | Keep the first scale |
| `intermediates_too_many` | The passes write over 4 intermediates | Reuse some |
| `transition_missing` | A transition has no `transition` section | Add it with its style |
| `transition_unexpected` | A block that is not a transition has one | Remove it |
| `transition_style` | A sound transition says `fade` | Sound transitions are `mix` |
| `transition_curve_missing` | A fade lacks one of its four curves | Add it |
| `transition_curve` | A curve breaks the rules (2 to 64 points, progress from 0 to 1 strictly rising, opacity 0 to 1) | Fix it (section 4.7) |
| `transition_curve_unexpected` | A mix has curves | Remove them |
| `migrations_too_many` | Over 64 migrations | Keep 64 at most |
| `migration_steps_too_many` | A migration has over 64 steps | Split it |
| `migration_versions` | A migration starts at 0, does not go up, or goes past `format` | Fix `from` and `to` |
| `migration_chain` | Migrations do not follow one another, or the last does not end at `format` | Each starts where the one before ends |
| `migration_param` | A step names a badly spelt setting | Fix the name |
| `migration_rename_same` | A rename to the same name | Remove it |
| `migration_leaves_param` | A rename leaves a setting the block does not declare | Declare it, or remove it in a later step |
| `migration_unknown_param` | A step's setting ends as one the block does not declare | Fix the name |
| `migration_step_type` | `scale` or `offset` on a setting that is not a number, or `enum` on one that is not an enum | Fix the step |
| `migration_enum_value` | An `enum` step maps to a value that is not a choice | Map to an existing choice |
| `migration_default_type` | A `default` step's value is the wrong type or out of range | Fix the value |

### 9.5 Picture code

| Code | What it means | What to do |
|---|---|---|
| `wgsl_not_named` | The manifest names no WGSL file | Set `video.source` |
| `wgsl_missing` | The WGSL file is not in the package | Add it |
| `wgsl_too_large` | The source is over 64 KiB | Make it smaller |
| `wgsl_not_utf8` | The source is not UTF-8 | Save it as UTF-8 |
| `wgsl_not_ascii` | A character outside comments is not ASCII | Use ASCII in code; anything goes in comments |
| `wgsl_comment_unterminated` | A `/*` comment is never closed | Close it |
| `wgsl_directive` | An `enable`, `requires` or `diagnostic` directive, or 16-bit floats | Remove it; values are 32 bits |
| `wgsl_reserved_name` | A name starts with `vep_` or `Vep` and is not one of the prelude's | Rename it |
| `wgsl_name_builtin` | A declaration hides a WGSL built-in name | Rename it |
| `wgsl_expression_too_long` | A statement has over 2,048 operators (a minus sign in a list of numbers counts) | Split it |
| `wgsl_too_many_else_ifs` | A function has over 256 `else if` | Use a `switch` or split it |
| `wgsl_declaration_too_long` | One declaration is over 256 KiB once joined | Split it |
| `wgsl_too_many_declarations` | Too many declarations (a stray `;` counts) | Remove some |
| `wgsl_text_too_large` | A draw's whole text is over 8 MiB | Make the code smaller |
| `wgsl_syntax` | The code cannot be read | Fix the place the message names |
| `wgsl_too_deep` | Nesting too deep (127 blocks, about 200 levels of expression) | Split it into functions |
| `wgsl_recursion` | A function calls itself, directly or not | Use a loop |
| `wgsl_parser_stopped` | The shader parser stopped on this code (a `select` of a constant vector built from smaller vectors) | Write one number per component, or name the vector with `let` first |
| `wgsl_invalid` | The code is not valid WGSL | Fix the place the message names |
| `wgsl_global` | A module-scope variable | Use a function's own variable, a setting or a constant |
| `wgsl_binding` | A resource binding | The editor binds everything; read pictures with `vep_sample` |
| `wgsl_storage` | A storage buffer, `arrayLength`, or an array without a fixed size | Not available to blocks |
| `wgsl_workgroup` | Workgroup memory | Not available to blocks |
| `wgsl_pointer` | A pointer to memory other than the function's own variables | Point only to your own variables |
| `wgsl_texture` | A texture or sampler, a texture read or write, `textureSample` | Use `vep_sample`, `vep_load` and `vep_input_size` |
| `wgsl_atomic` | Atomics | Not available to blocks |
| `wgsl_barrier` | Barriers | Not available to blocks |
| `wgsl_subgroup` | Subgroup operations | Not available to blocks |
| `wgsl_derivative` | `dpdx`, `dpdy`, `fwidth` and variants | Not available to blocks |
| `wgsl_discard` | `discard` | Return a transparent pixel instead |
| `wgsl_entry_point` | An entry point (`@vertex`, `@fragment`, `@compute`) | Write only the stage functions |
| `wgsl_override` | An `override` constant | Use a `const`, or a setting |
| `wgsl_forbidden` | Ray tracing or cooperative matrices | Not available to blocks |
| `wgsl_type_too_large` | An array or struct over 16 KiB | Make it smaller |
| `wgsl_too_many_functions` | Over 64 functions | Merge some |
| `wgsl_stage_missing` | A declared stage or pass has no function | Write it with exactly its signature (section 4.3) |
| `wgsl_stage_signature` | A stage or pass function's signature is not exactly the contract's | Copy the signature from section 4.3 |
| `wgsl_input_not_available` | A `region`, `place`, `pixel` or `source` stage reaches `vep_sample`, `vep_load` or `vep_input_size` | Read pictures only in passes, blends and transitions |
| `wgsl_too_costly` | A stage's straight-line cost is over 4,096 units | Do less, or use a loop (which the budget pays for) |
| `wgsl_folded_constant` | A maths function called with constant arguments only | Give the argument a name with `let` first |
| `wgsl_continue_skipped` | A loop's `continuing` reads a value a `continue` skipped | Name the value before the `continue`, or use a `var` |
| `wgsl_no_return` | A function can end without returning its value (a `break` out of a `switch`) | Add a `return` after the `switch` |
| `wgsl_too_much_data` | Over 4 Mi 32-bit values in one draw | Use fewer or smaller arrays |
| `wgsl_draw_too_costly` | The whole draw's code costs over 2^18 units | Make the code smaller |
| `wgsl_too_many_blocks` | Over 64 blocks with code in one draw (at run time: the rest are left out) | Use fewer blocks in one stack |
| `wgsl_uniforms_too_large` | The draw's settings are over 64 KiB, or over what the card holds (at run time: the blocks that do not fit are left out) | Use fewer or smaller settings |
| `wgsl_compose_failed` | The block could not be joined with the editor's code | Report it: it should not happen to a block that passed the other checks |
| `wgsl_check_failed` | The checker stopped unexpectedly or could not follow a name | Report it |
| `code_check_failed` | The code could not be checked | Report it |

### 9.6 Sound code

| Code | What it means | What to do |
|---|---|---|
| `wasm_not_sound_block` | A WebAssembly module on a picture block, or a sound block naming none | Only sound blocks run WebAssembly |
| `wasm_not_module` | The file is not a binary WebAssembly module | Build a `.wasm` file (not text format) |
| `wasm_invalid` | The module is not one the editor can run (SIMD, 64-bit memory, several memories, wasmi's limits, or damaged) | Build for `wasm32v1-none` without SIMD |
| `wasm_start_function` | The module has a start function | Do that work in `vep_prepare` |
| `wasm_imports` | The module imports something | Build every function into the module (maths with `libm`) |
| `wasm_too_many_locals` | A function declares over 1,024 locals | Split the function |
| `wasm_too_many_elements` | Element segments hold over 1,000 function references | Use fewer |
| `wasm_export_missing` | An export the kind needs, or `memory`, is missing | Section 5.1 |
| `wasm_export_type` | An export has the wrong signature, or `memory` is not a memory | Section 5.1 |
| `wasm_export_unknown` | An export starts with `vep_` but is not one of the kind's | Remove it, or check the block's kind |
| `wasm_memory_too_large` | The memory starts above 64 pages (4 MiB): the stack and the `static` data together | With Rust, link with `-C link-arg=-zstack-size=65536`; if you do, a `static` (a delay line, a table) is too large: make it smaller, or grow the memory in `vep_prepare` (section 5.6) |
| `wasm_abi_version` | `vep_abi` does not return 1 | Return 1 |
| `wasm_trial_failed` | The test run on silence failed (a trap, out of fuel, an error code, not-finite output, bad buffers) | Read the reason; run your block on silence yourself |

### 9.7 A block in a project

These come when a change to a project (from the window, `vep` or later the AI) would write something a block does not allow. Nothing is written.

| Code | What it means | What to do |
|---|---|---|
| `block_unavailable` | The block of the node cannot be used (its status says why) | Install, turn on or update the block |
| `block_unavailable_params_locked` | The change would alter the settings of a node whose block cannot be used | Make the block usable first; moving, switching and removing still work |
| `block_wrong_place` | The node's kind cannot be used there | Section 1's table |
| `node_version` | The node's settings follow another format than the installed block's | Let the editor convert them (it does on any edit) |
| `param_unknown` | A setting the block does not declare | Check the id |
| `param_wrong_type` | A value of the wrong type | Section 3.4 |
| `param_not_finite` | A NaN or infinite number, in a value or an easing | Use a finite number |
| `param_not_option` | A choice that is not one of the setting's options | Use an option's value |
| `param_bad_shape` | A shape with a negative size or radius, or a path of fewer than two points | Fix the shape |
| `param_bad_curve` | A curve of fewer than two points, or with x not strictly rising | Fix the curve |
| `param_out_of_range` | A value outside `min` and `max` | Stay within the range |
| `param_not_keyframable` | Keyframes on a setting that cannot be animated | Give it one value |
| `keyframes_unsorted` | Keyframe times that do not strictly rise | Sort them |

### 9.8 The editor's folders

These are notes about the folders, shown in the block manager ("What the editor noticed while reading its blocks") and by `vep blocks list`.

| Code | What it means | What to do |
|---|---|---|
| `user_file_name` | An installed package file is not named after its block | Name it `<id>.chadfx-block`, or install it with the editor |
| `user_folder_too_many` | The blocks folder holds more than 1,000 packages (only the first 1,000 by name are read) or over 4,000 entries; or a new block was refused because the folder already holds 1,000 packages | Remove the blocks you no longer use, then install again |
| `block_duplicate` | Two packages give one block id; the first is kept | Remove the other |
| `builtin_folder_missing` | The editor's own blocks were not found | Reinstall the editor; for `vep`, set `VEP_BUILTIN_BLOCKS` |
| `builtin_folder_name` | A folder of the editor's own blocks is not named after its block | Reinstall the editor |
| `builtin_folder_too_many` | The editor's blocks folder holds too many entries | Reinstall the editor |
| `default_placement_duplicate` | Several blocks claim to be the default placement block | Only the editor's own may |
| `default_transition_duplicate` | Several usable blocks claim to be the default transition of one media | The editor's own wins; drop the claim |
| `state_unreadable` | `state.json` could not be read; it was set aside, and every installed block was turned off to be safe | Turn on what you want again |
| `state_unwritable` | `state.json` could not be written; the message says what it still holds | Check the folder's space and permissions |
| `blocks_quarantined_after_crash` | The last run stopped while these blocks were in use, so they were turned off | "Enable again" if you trust them |
| `in_flight_unremovable` | The crash record could not be removed | Check the folder's permissions |
| `block_id` | An internal request named a malformed block id; nothing changed | Report it |

---

## 10. The line between the core and blocks

The owner's brief asked to decide which things stay in the editor's core because they are about **time and placement** rather than pixels or sound, and which become blocks. The decision, as built:

**The rule.** The core decides *when* and *where on the timeline* things happen. A block decides *what happens to the picture or the sound* at one moment the core gives it: one frame, or one slice of 128 sound frames. A block never sees the timeline, other clips, other moments or the project.

**What stays in the core, and why:**

| Stays in the core | Why |
|---|---|
| Exact rational time, the frame and sample grids, clip placement on tracks (where a clip starts, how long it lasts, which part of its media it shows), tracks, sequences, markers | They are the timeline itself. Every edit, undo, the journal and crash recovery are made of them, and they must work identically whether or not any block is installed |
| The time remap: a clip's speed, reverse and frame blending (`core.time_remap`, the one core node) | It decides which moment of the media a clip shows at a time, which is time, not pixels: a block sees one moment and cannot choose another. It stays a node in the clip's list at the same position, because saved journals find nodes partly by position |
| Clip sources that are not generated: a media file, an adjustment clip, silence | They are references to media or to the timeline, not processing |
| Transition timing: which clip's transition applies at a cut, when it starts, how long it lasts, its progress | Timing of the timeline. The transition block decides only how the two pictures or sounds mix |
| The stack order, folding, roles, the default placement of a layer, the normal "over" compositing, decoding into linear light, the output pass (clipping and encoding), mixing tracks, mute and solo, the final clip of the sound | They are the frame the blocks work in. Roles let the core's own features (fit to frame, layer opacity, fades) work with any block without knowing it |
| Undo, the journal, recovery, the project file | They must never depend on which blocks are installed: a journal replays the same without any block |

**What became blocks:** transform, crop, opacity (compositing), colour, mask, the blend modes and the normal blend, the cross dissolve and the wipe, solid colour, blur and glow, gain, pan, the crossfade, the tone, the three-band EQ. The transform is "placement", but placement **inside the frame**, which is geometry of pixels; the core keeps the default placement and finds the transform through roles, so "fit to frame" needs no knowledge of any block.

The old effect list also named effects that were never drawn or played: blur, curves, LUT, text, a custom shader, EQ, compressor and time-stretch. Blur and EQ are now new blocks (`chadfx.blur`, `chadfx.eq3`); the old projects' nodes of all eight stay `not_available_yet` (`chadfx.v2.*`), so they do not start doing something they never did. A custom shader is what a block you write now is. When built, curves, LUT and the compressor are blocks of the kinds above. Text needs something the core does not offer yet (a way to lay out and draw text), which is the next task after this work. A time-stretch that keeps pitch takes more or less media than the timeline span it fills, which a block of this version cannot do (it is given exactly its slice): the time part stays in the core's time remap, and pitch correction would be a sound filter with a declared latency.

---

## 11. The `vep blocks` commands

`vep blocks --help` says all of this in short. Every command takes `--json` before `blocks` (`vep --json blocks list`) and then prints one JSON object. Exit code 0 means success, 1 a problem.

| Command | What it does |
|---|---|
| `vep blocks list` | Every block the editor knows, built-in and installed: id, version, kind, origin and status (`ok`, `invalid`, `disabled`, `quarantined`), and notes about the folders |
| `vep blocks info ID` | One block: what it does, its settings with types, defaults and ranges, where it is, its identity, and every problem if it cannot be used |
| `vep blocks catalog` | Every block that can be used now, with its description, settings and roles: what the AI reads |
| `vep blocks validate PATH` | Check a package folder, a `.chadfx-block` file or a block's `.wgsl` file without installing it, and explain every problem (exit code 1 when there are problems). `--builtin` checks it as one of the editor's own |
| `vep blocks pack PATH [OUT]` | Check a package folder or a block's `.wgsl` file and write it as one `.chadfx-block` file (by default beside it, with its name). `--builtin` as for `validate` |
| `vep blocks install PATH` | Check and install a package folder, a `.chadfx-block` file or a block's `.wgsl` file for this user. `--replace` replaces another installed version; `--project FILE` says first what replacing means for that project |
| `vep blocks remove ID` | Remove an installed block (not a built-in one). `--project FILE` refuses while that project uses it, unless `--force` |
| `vep blocks disable ID` | Turn a block off, built-in or installed |
| `vep blocks enable ID` | Turn it on again |
| `vep blocks enable-again ID` | Lift a quarantine: use a block the editor turned off because it misbehaved |

`vep` uses the same folders as the editor (section 2.1) and the same words. It never touches the crash record of a running editor. `install`, `validate` and `pack` read a block's file or folder only from a drive or a network share, and refuse any other path at once (section 8.1).
