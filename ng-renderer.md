# No-GPU picture proof: ng-renderer.md

## summary

crates/render-recipe and crates/cpu-render are finished. Both build with zero warnings. All 245 tests pass in the dev profile and in release.

**Software renderer**
- It interprets naga IR through a compact compiled form, with no graphics card.
- It covers everything the block checker allows, except calls the WGSL parser has already folded (known_gaps).

**Planted-bug check**
- I re-ran it on throwaway copies, widened from 38 to 217 bugs.
- The widening exposed about 50 real test gaps, now closed. Most were rules about the order of arithmetic.
- Final result: 201 caught by a failing test, 8 noticed only because the run did not end, 8 controls that cannot change a byte. The 8 hangs would hang a real `cargo test` too (known_gaps).

**Results**
- All 351 scenes draw in 61 s on one thread, 13 s on 8 threads, 7 s on 32, in the dev profile.
- Compared with stored pictures: 0.21% of bytes differ from WARP and 1.04% from the RTX A4000. WARP and the A4000 differ from each other by 1.14%.

**Portability evidence**
- Mac, Linux and ARM have NOT been run; WSL has no distribution here.
- As the nearest substitute I added tools/wasm-check. It builds these crates' own source for WebAssembly and runs it under Node: 32-bit addresses, another code generator, software fused multiply-add, floor and square root.
- All 359 frames (351 scenes plus 8 new arithmetic frames) and all 23 libm pinned hashes came out identical to native. This runs on the same physical processor, so it says nothing about how ARM rounds.

**Decision for Stage 3:** a block that passes the block checker but calls `sin(0.5)` or `max(1.0, 2.0)` with constant arguments is refused here (known_gaps). Someone must decide whether the checker refuses it too.

## contract

## Adding it to a crate
`render-recipe = { workspace = true }` and `cpu-render = { workspace = true }` in `[dev-dependencies]`. cpu-render's only non-dev dependencies are `render-recipe`, `naga =30.0.1` (wgsl-in) and `libm =0.2.16`. It has one dev-dependency, `blocks`.

## render-recipe (no dependencies)
- `Format { Rgba8Unorm, Rgba16Float }`; `.bytes_per_pixel() -> usize`; `.name() -> &'static str`
- `Image { name: String, width: u32, height: u32, format: Format, pixels: Arc<Vec<u8>> }`
- `Target { name, width, height, format }`
- `ImageId(pub u32)`, `TargetId(pub u32)`
- `TextureSource { Image(ImageId), Target(TargetId) }`
- `AddressMode { ClampToEdge, Repeat, MirrorRepeat }`, `FilterMode { Nearest, Linear }`
- `Sampler { address_u, address_v, min_filter, mag_filter, mipmap_filter }`, with const `Sampler::LINEAR_CLAMP`
- `Resource { Uniform(Vec<u8>), Texture(TextureSource), Sampler(Sampler) }`
- `Binding { group: u32, binding: u32, resource: Resource }`
- `Blend { Replace, PremultipliedOver }`
- `Draw { label: String, shader: String, wgsl: Arc<str>, vertex_entry: String, fragment_entry: String, vertex_count: u32, bindings: Vec<Binding>, blend: Blend }`
- `Pass { label: String, target: TargetId, clear: Option<[f64; 4]>, draws: Vec<Draw> }`
- `Step { Pass(Pass), Copy { from: TargetId, to: TargetId } }`
- `Recipe { images: Vec<Image>, targets: Vec<Target>, steps: Vec<Step>, output: Option<TargetId> }` (Default)
- `Recipe::check(&self) -> Result<(), RecipeError>`, `RecipeError::message(&self) -> &str`, `MAX_SIDE: u32 = 65536`
- `Recipe::encode(&self, EncodeOptions) -> Vec<u8>`, `Recipe::decode(&[u8]) -> Result<Recipe, RecipeError>`
- `EncodeOptions { pixels: bool, wgsl: bool }`, with const `EncodeOptions::ALL`

Rules: targets start as all zeros, steps run in order, a pass may not read the target it draws into.

### Frozen recipe format
Little-endian; magic `VEPRCP1\n`, then flags u8 (bit 0 pixels held, bit 1 program text held), then images, targets, steps and output. The full byte layout is the module comment at the top of crates/render-recipe/src/codec.rs, and a test pins it. Encoding is deterministic.

When pixels or text are left out, `decode` gives empty `pixels` and `wgsl`. Fill them in from `Image::name` and `Draw::shader` before use: `check()` fails until they are filled.

## cpu-render
- `Renderer::new()`, `Renderer::with_options(Options)`
- `Renderer::register_module(&self, wgsl: &str, module: &naga::Module) -> Result<(), Error>`
- `Renderer::run(&self, &Recipe) -> Result<Frame, Error>`
- `Renderer::run_keeping(&self, &Recipe, keep: &[TargetId]) -> Result<Frame, Error>`
- A Renderer is Send + Sync and caches compiled programs by text, so keep one for many frames.
- `Options { threads: usize /* 0 = one per processor */, limits: Limits }`
- `Limits { steps_per_invocation: u64 /* default 2^24 */, steps_per_draw: u64 /* default 2^38 */ }`
- `Frame { width, height, format, bytes: Vec<u8>, kept: Vec<TargetValues> }`
- `TargetValues { target, width, height, format, values: Vec<[f32; 4]> }`, `.bytes() -> Vec<u8>`
- `Error { Recipe(String), Shader { shader, message }, Unsupported { shader, what }, Draw { draw, message }, StepLimit { draw, place, limit }, FloatEnvironment, Internal(String) }` (Display, std Error, PartialEq)
- `Shader::new(&str) -> Result<Shader, Error>`, `.with_limits(Limits)`, `.call(&str, &[u32]) -> Result<Vec<u32>, Error>`
- `float_words(&[f32]) -> Vec<u32>`
- Also exported: `f16_bits_to_f32`, `f32_to_f16_bits` (nearest, for shader functions), `f32_to_f16_bits_toward_zero` (how a target stores), `f32_to_unorm8`, `unorm8_to_f32`

crates/cpu-render/README.md is the definition of the arithmetic.

### Drawing a composed block module
crates/cpu-render/tests/composer.rs is a working example.
- Set `wgsl = Arc::from(composed.source.as_str())`, `vertex_entry = composed.vertex_entry`, `fragment_entry = composed.fragment_entry`, `vertex_count = composed.vertex_count`.
- Map `composed.bindings` by `BindingKind`: Uniforms to `Resource::Uniform`, Picture and Input(i) to `Resource::Texture`, the two samplers to `Resource::Sampler`.
- Call `renderer.register_module(&composed.source, &composed.module)` first. Otherwise the inserted loop budget is not in the draw: a hostile loop ends in `Error::StepLimit` instead of running out of budget. A block within its budget gives the same bytes either way.

### Comparing machines
- Pinned hashes that must match on any machine: tests/everything.rs, tests/threads.rs, tests/today.rs, tests/composed.rs, tests/libm_pins.rs. Run `cargo test -p cpu-render` on the new machine.
- WebAssembly check:
  - `cargo run -p cpu-render --example freeze_recipes -- <folder>`
  - `node crates/cpu-render/tools/wasm-check/check.js <folder> [work folder]`
  - `<folder>` holds `N.recipe` (encoded with `EncodeOptions::ALL`) and `list.txt` with lines `N<tab>16 hex digits of the FNV-1a of the output bytes<tab>name`.
  - The default work folder is `target/wasm-check`.
  - Needs the wasm32v1-none target and Node.

### Root Cargo.toml
Added only the two workspace dependency lines and `[profile.dev.package.cpu-render] opt-level = 3`.

## deviations

- Two 16-bit float rules, not one. Targets store toward zero (finite overflow becomes the largest finite value), because an RTX 3090 on Vulkan and DX12 and WARP on DX12 measured that way. An earlier attempt found this; rounding to nearest put this renderer about 6% of bytes from each card. `quantizeToF16` and `pack2x16float` round to nearest, ties to even. Both rules are stated in the README.
- cpu-render refuses `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `asin`, `acos`, `atan`, `atan2`, `asinh`, `acosh`, `atanh`, `exp`, `exp2`, `log`, `log2`, `pow`, `fma`, `min` and `max` when the WGSL parser folded them from constant arguments. naga folds those with whatever maths library the build links, so the result could differ per machine and leaves no trace in the IR. The message says to name the argument with `let`. This goes beyond 'covering everything the block validator allows'; see known_gaps.
- cpu-render validates with `Capabilities::SHADER_FLOAT16_IN_FLOAT32` where the block checker uses none, because `quantizeToF16`, `pack2x16float` and `unpack2x16float` need it. Values stay 32-bit.
- A step is a weighted count, not one instruction: large copies and slow instructions weigh more, so the limit means about the same time whatever a program does. Weights are stated in the README.
- Only `mag_filter` is used; `min_filter` and `mipmap_filter` are ignored. Pictures have one level and no scale is estimated. Today's sampler is linear for both, so this changes nothing for it.
- `discard` stops the invocation at once rather than continuing as a helper invocation; nothing visible differs. Fragment outputs at locations other than 0 are ignored.
- Beyond the brief in render-recipe: `Step::Copy`, `Recipe::check`, `Recipe::encode` and `Recipe::decode`.
- Beyond the brief in cpu-render: `register_module`; a dev-dependency on `blocks` so tests/composer.rs draws what the composer makes today (no graphics library comes with it, and a test asserts that); tests/everything.rs; examples/freeze_recipes.rs; tools/wasm-check. The tool is a nested folder cargo does not build or test.
- New limits beyond the brief, all giving errors rather than exhausting the machine: 2^20 vertices per draw, 2^28 words of vertex outputs per draw, 2^28 pixels per target or picture, 2^30 pixels across targets, 2^22 words per value or stack, 8 MiB of text, and parser-input limits that prevent stack overflows in naga.
- Banned-call scan is wider than asked: also integer `.min` and `.max`, `round`, `fract`, `signum`, `recip`, clocks, environment variables, thread identity, hash tables, and `target_feature`/`target_arch` except one guard that refuses to build on x87.
- Added `Error::FloatEnvironment`: drawing is refused on a thread whose processor settings flush subnormals or change the rounding direction.
- Direct calls (`Shader::call`) canonicalise NaN in float result words, by the result's type.

## measurements

- 351 scenes through cpu-render, dev profile, recipes made from today's frame plans, Threadripper 3970X: 61.35 s on 1 thread, 22.45 s on 4, 12.97 s on 8, 7.00 s on 32. Slowest scene about 4.9 s on 1 thread (hd.noise_color_mask). Same bytes on all four thread counts. An 8-core laptop was not available; the limit of about 2 minutes is met even single-threaded on this CPU.
- Agreement with stored pictures, 343 PNG scenes, 71,253,604 bytes: this renderer vs WARP 0.211% of bytes differ, 257 scenes identical, largest difference 193 (WARP alone, on one pixel at a rotated edge; next largest 4). This renderer vs RTX A4000 1.038%, 57 identical, largest 6. WARP vs A4000 1.140%, 54 identical, largest 193. One scene exceeds the cards' mutual distance and 1 level: color.sequence_contrast_saturation at 3 levels from WARP over 168 bytes, against 2 between the cards.
- WebAssembly under Node, one thread, using the crates' own source: 351 scenes plus 8 arithmetic frames all byte-identical to native x86-64 by FNV-1a hash, 93.4 s; 88.2 s on an earlier build with the machine quiet. All 23 libm pinned hashes identical, including `fmaf` through libm's software path.
- Step rate, dev profile, one thread: 101 to 1363 million steps a second by program kind (determinants slowest; pow/sin/atan2 fastest because those programs quickly reach NaN or infinity). So one invocation reaches the limit within about 0.17 s and a draw within about 45 minutes of one core.
- Planted bugs on throwaway copies, 217 in all, final code, all counted once. Caught by a failing test: 201. Noticed only because the run did not end within the 120 or 280 s limit: 8 (no step limit on loops; no step limit on calls; copies weigh nothing; looking at pixels costs nothing; a target too large is allocated; loops cost one step; arguments copied for free; continue goes to the top). Not noticed, marked as controls: 8 (sin plus nothing; reflect same; reflect in another order; cross in another order; a group that follows a failure is not stopped; no environment check per group; a non-float location not flat; a shader's errors not remembered). Each control is either arithmetically identical or affects only speed or an unreachable path. A first pass with the old tests left about 50 survivors, which led to the new tests. The list and results stay in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\final (make_mutants.py, mutate2.py, results.txt).
- Test counts: 245 in total across both crates, in the dev profile and in release.

## known_gaps

- Mac, Linux and ARM have not been run. WSL has no distribution on this PC, and no other Rust targets are installed besides wasm32v1-none. The WebAssembly check is the nearest substitute, but it is the same physical processor, so it says nothing about how ARM rounds. The promise for those platforms rests on Rust's floats being exact IEEE 754 there, on libm being the same source, and on the pinned hashes. The real check is `cargo test -p cpu-render` on such a machine.
- A block that passes the block checker but calls one of the listed functions with constant arguments is refused here. That includes a harmless `max(1.0, 2.0)`, because the text cannot tell it from `max(0.0, -0.0)`, and integer `min`/`max` too. No built-in block does this. Stage 3 should decide whether the block checker refuses it as well; on a graphics card such a constant is also folded with the machine's own maths library.
- Unsupported, with an error naming the feature: derivatives (`dpdx`, `dpdy`, `fwidth`), storage buffers, atomics, barriers, texture kinds other than `texture_2d<f32>`, depth comparisons, override constants, ray tracing, subgroups, 16- and 64-bit values. The block checker forbids these for blocks.
- Eight planted bugs are noticed only as a run that never ends, and `cargo test` has no per-test timeout. Such a defect would look like a hang, not a failure, and needs the project's usual hard time limit.
- tests/environment.rs changes the processor's float settings only on x86-64. ARM's equivalent is not tested. The check itself (four sums) runs everywhere. 'No environment check per group' is a control: the same thread is checked at the start of the run, and only something changing another thread's settings could tell.
- 32-bit x86 without SSE2 refuses to build, on purpose. 32-bit ARM with NEON was not considered.
- A hostile draw under the default limits ends at about 45 CPU-minutes at worst. Use `Limits` for tighter bounds in tests.
- The comparison with stored NVIDIA and WARP pictures and the speed measurement used a scratch program in the scratchpad's anchor folder, not a test in this repository. The anchor test and the frozen data folder crates/golden/data/render-portable belong to the next jobs and do not exist yet. Recipes used here came from today's frame plans, not from a capture of tag stage-1-done.
- The full workspace test suite was not run; only these two crates' tests and a workspace build. tests/composer.rs compiles against crates/blocks in the shared tree, so a mid-edit break in blocks would stop `cargo test -p cpu-render` from compiling.

## checks

All commands were run from D:\chadfx, each under a hard time limit.

| Command | Result |
|---|---|
| `cargo build -p cpu-render -p render-recipe --all-targets` (after touching both libs) | exit 0, 0 warnings |
| `cargo test -p cpu-render -p render-recipe --no-fail-fast` | exit 0, 245 passed, 0 failed, 0 ignored |
| `cargo test --release -p cpu-render -p render-recipe --no-fail-fast` | exit 0, 245 passed, 0 failed, 0 ignored |
| `cargo clippy --offline -p cpu-render -p render-recipe --all-targets` | no findings in the two crates (other crates' older findings remain; run just before the last test-only edits and the wording fix in fstd.rs) |
| `cargo build --workspace --all-targets` | exit 0, 0 warnings |
| `cargo tree --offline -p cpu-render -e normal --prefix none` | no `-sys` crate and nothing graphical; also asserted by tests/dependencies.rs. `--target all` cannot run offline here because crates for other platforms are not downloaded. |
| `node crates/cpu-render/tools/wasm-check/check.js <359 frozen recipes>` | 23 libm hashes and 359 recipes all the same, "Everything is the same." |
| `cargo run -p cpu-render --example freeze_recipes -- <folder>`, then check.js on those 8 frames | also the same |
| Planted bugs on throwaway copies (scratchpad final\mutate2.py) | see measurements |

**Not run:** Mac, Linux, ARM, a real 8-core laptop, or the full workspace test suite.

**Files and cleanup:** only crates/render-recipe, crates/cpu-render and the two lines plus profile section in the root Cargo.toml were edited by hand; cargo updated Cargo.lock on its own. Temporary copies and build folders (about 18 GB) were deleted, including the earlier attempt's anchor/target build folder, so that scratch program must be rebuilt to run again. Free space on C: was 327 GB after cleanup.
