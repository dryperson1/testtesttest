# Built-in sound blocks

The editor's own sound blocks, written in Rust and compiled to WebAssembly,
and the small kit they are written with. This folder is its own cargo
workspace; the editor's root `Cargo.toml` excludes `blocks/`.

| crate | block | kind | what it does |
|---|---|---|---|
| `kit` (`block-kit`) | | | the contract as safe types and three export macros |
| `gain` | `chadfx.gain` | audio.filter | today's `10^(dB/20)`, smooth when animated |
| `pan` | `chadfx.pan` | audio.filter | today's equal-power pan (stereo only), smooth when animated |
| `crossfade` | `chadfx.crossfade` | audio.transition | today's straight-line crossfade, bit for bit |
| `tone` | `chadfx.tone` | audio.source | today's test sine, with frequency and amplitude |
| `eq3` | `chadfx.eq3` | audio.filter | three bands: low shelf, peak, high shelf (new) |

The compiled modules are checked in as `blocks/builtin/<id>/block.wasm`,
each with a `block.wasm.recipe` beside it.

## Building

From the repository root:

```text
CHADFX_WRITE_SOUND_BLOCKS=1 cargo test -p blocks --test wasm_stale
```

This builds the workspace into a temporary folder with the pinned compiler,
copies each module to `blocks/builtin/<id>/block.wasm` and writes its
recipe. It never downloads anything (`--offline`, and rustup is told not to
install toolchains); `Cargo.lock` must already list every dependency
(`--locked`).

Without the variable the same test only checks:

* always, that each stored recipe matches the sources;
* when the pinned compiler and the `wasm32v1-none` target are installed,
  that a fresh build gives exactly the checked-in bytes.

So after changing anything here (a block, the kit, `Cargo.toml`,
`Cargo.lock`, `rust-toolchain.toml` or `.cargo/config.toml`), run the
command above and keep the new files.

To build by hand: `cargo +stable build --release` in this folder (when
`rustc +stable -V` says 1.99.0); the modules are then in
`target/wasm32v1-none/release/<crate>.wasm`.

### The compiler

`rust-toolchain.toml` pins rustc 1.99.0 with the `wasm32v1-none` target.
rustup treats the name `1.99.0` as its own toolchain, separate from
`stable` even when stable is 1.99.0, so on a machine with only `stable`
installed a plain `cargo build` here asks for 1.99.0. The builder therefore
looks for any installed toolchain whose compiler reports `release: 1.99.0`
(first `1.99.0`, then `stable`) and runs cargo through it. To install the
exact toolchain: `rustup toolchain install 1.99.0 --target wasm32v1-none`.

### The recipe

`block.wasm.recipe` holds BLAKE3 of these files, sorted by path: this
folder's `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and
`.cargo/config.toml` (the compiler version and the flags), every file of
`kit/` and every file of the block's own folder (`target/` folders skipped,
Windows line endings read as plain ones), plus the build command. Changing
any of them marks the module stale. This README is not part of it.

### Flags

`opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`,
`strip = true`, no debug information, and a 64 KiB stack
(`-C link-arg=-zstack-size=65536`; Rust's default is 1 MiB, which would make
every instance start with 17 pages of memory). The modules contain no
names, no paths and no custom sections (a test checks).

## The contract (ABI version 1), in short

The full text is section 8 of the blocks contract; the editor's side is
`crates/blocks/src/wasm`.

* A module imports nothing at all and has no start function. Maths comes
  compiled in (the `libm` crate, re-exported by the kit), so every machine
  gets the same bits.
* It exports `memory` and `vep_abi`, `vep_prepare`, `vep_io`, `vep_params`,
  `vep_reset`, plus `vep_process` (filters), `vep_io2` and `vep_transition`
  (transitions) or `vep_generate` (sources), with exact signatures, and no
  other name starting with `vep_`.
* Memory starts at no more than 64 pages (4 MiB) and can grow to 16 MiB.
  Growing costs fuel (1 per 64 bytes, about 262,000 for the whole 16 MiB),
  so grow in `vep_prepare`, which gets the fuel of a full slice.
* Calls come in slices of at most 128 frames aligned to absolute positions
  on the timeline. Each call gets 100,000 fuel plus 2,000 per frame per
  channel; running out is a trap.
* Parameters arrive as f32: the values at the slice's first frame, then the
  values just after its last frame, so a block can ramp smoothly.
* A trap, a non-zero return or an output sample that is NaN or infinite
  turns the block off for the session; the editor then passes the sound
  through (filters), plays silence (sources) or uses its own straight-line
  crossfade (transitions).
* Blocks without memory (warm-up 0) sound exactly the same in preview and
  export. A block with memory declares a warm-up; after a jump the editor
  resets it and runs it over the preceding stretch first, so it sounds the
  same once that warm-up has passed (for `eq3`, measured: the output then
  differs from uninterrupted playback by less than 1e-12).

## Writing a block with the kit

```rust
#![no_std]
#![forbid(unsafe_code)]

use block_kit::{Block, Buffer, Filter, Slice, OK};

struct Half;

impl Block for Half {
    const PARAM_VALUES: usize = 0;
}

impl Filter for Half {
    fn process(slice: &Slice, io: &Buffer) -> i32 {
        for i in 0..slice.samples() {
            io.set(i, io.get(i) * 0.5);
        }
        OK
    }
}

block_kit::filter!(Half);
```

The crate is a `cdylib`. State that lasts between calls goes in `static`
cells from the kit (`StateF32`, `StateF64`, `StatesF64<N>`). The kit's
buffers hold 128 frames of up to 32 channels; `vep_prepare` refuses larger
setups. A panic becomes a trap.

There is no `unsafe` code anywhere here. Exporting a function under a fixed
name needs `#[unsafe(no_mangle)]`, which Rust counts as unsafe; it is
written only inside the kit's three export macros, and every crate still
compiles with `#![forbid(unsafe_code)]` (Rust does not apply that lint to
code that comes from another crate's macro). A block that wrote the
attribute itself would be refused by that lint.

### Other languages

Anything that produces a WebAssembly 1.0 module with these exports and no
imports works. Notes from the research: AssemblyScript needs the stub
runtime and `--use abort=` (its default build imports `env.abort`); C needs
upstream clang with `--target=wasm32 -nostdlib -Wl,--no-entry`; Zig uses
`-target wasm32-freestanding -fno-entry -rdynamic`. Faust's WebAssembly
output imports its maths from the host and is refused.
