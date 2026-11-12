# Stereo width: a sound filter in Rust (template)

`example.width` makes stereo sound narrower or wider. Each frame is split
into what left and right share (the middle) and what tells them apart (the
side); the side is multiplied by `width`. Width 0 is mono, 1 leaves the
sound exactly as it is (to the bit), 2 doubles the difference between the
channels. With the switch `swap` on, left and right then change places.
Sound that is not stereo is left as it is. The width can be animated with
keyframes, and it then changes smoothly inside every slice of 128 frames.
It has no memory, so preview and export give exactly the same samples.

## The files

The editor reads three of them; the rest are how `block.wasm` is made and
are not packed:

- `block.json`: the description (kind `audio.filter`, the settings, the
  module `block.wasm`, warm-up 0 because the block keeps no memory).
- `block.wasm`: the compiled WebAssembly module.
- `README.md`: this file, shown by the block manager.
- `src/lib.rs`: the block, the file to change.
- `src/contract.rs`: the editor's contract for a filter (what it calls and
  where the samples and settings are), copied from the kit the editor's own
  sound blocks use, so this folder needs nothing else. Leave it as it is.
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml`:
  the build (target `wasm32v1-none`, a 64 KiB stack, small output).
- `block.wasm.recipe`: for the editor's own test, which checks that
  `block.wasm` was built from these files. Delete it in your copy.

## Make it yours

1. In `block.json`, change `id` (your own name before the dot, never
   `chadfx.` or `core.`), `name`, `description`, `author`, `license` and
   the `params`.
2. In `src/lib.rs`, set `PARAM_VALUES` and the positions (`WIDTH`, `SWAP`)
   to your settings, in the order of `params`: a number, switch or choice
   is 1 value, a vec2 2, a vec3 3, a colour 4. Then change `process`.
3. A block that keeps memory between calls (a filter with feedback, a
   delay) keeps it in `static` atomic cells, forgets it in `reset`, and
   declares in `block.json` how long it needs to settle (`warmup_ms`, at
   most 4000). `reset` gets the fuel of one call only: a loop over more
   than about 50,000 samples runs out of it, and the block is refused. For
   a long delay line, reset its counters instead and read what has not
   been written since as silence (the guide, section 5.4).
4. Maths (`sin`, `pow`...) comes from the `libm` crate, already a
   dependency: a block may import nothing from the editor.

## Build it

You need Rust with the `wasm32v1-none` target for your stable compiler:
`rustup target add wasm32v1-none --toolchain stable`. Name the
toolchain: run in this folder without it, rustup adds the target to the
compiler `rust-toolchain.toml` names (1.99.0), downloading that compiler
first, and the build below still lacks it. Then, from this folder:

```text
cargo +stable build --release --locked
```

then copy `target/wasm32v1-none/release/block.wasm` over `block.wasm`
(`copy target\wasm32v1-none\release\block.wasm block.wasm` on Windows).

`+stable` builds with your installed stable compiler. Without it, rustup
uses the compiler `rust-toolchain.toml` names, 1.99.0, and downloads it if
it is not installed. Any recent compiler builds the block; the checked-in
file was built by one that reports 1.99.0 (`rustc +stable -V`), and only
that one gives exactly the same bytes. Once `libm` 0.2.16 is in cargo's
cache, `--offline` builds with no network.

## Check, pack and install

With the editor's command-line tool, `vep`, from this folder:

```text
vep blocks validate .
vep blocks pack . example.width.chadfx-block
vep blocks install example.width.chadfx-block
vep blocks remove example.width
```

`validate` also runs the module once on silence, as installing does. In
the editor: Blocks in the toolbar, then "Install from file..." or "Install a
folder...". The filter then appears in the inspector's list of effects for
sound clips.
