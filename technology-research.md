# Plug-in sandbox technology report: untrusted effect blocks (shaders, audio, packages, text)

**Abbreviations.** `$REG` = `C:/Users/Mauro/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`. Project paths are relative to `C:/Users/Mauro/Documents/CLAUDESTART`.

**Machine facts.** rustc 1.99.0 (2026-09-28). Dev machine: i9-13900, 128 GB, RTX A4000 (driver 596.36). It is much stronger than the target laptop, so every timing below is optimistic.

**Nothing in the project was changed.** Two scratch measurement programs were built offline from the local cargo cache, with no downloads:
- `docs/blocks-rewrite\plugbench\src\main.rs`
- `docs/blocks-rewrite\dspbench\main.rs`

---

## 0. Recommendations at a glance

| Question | Recommendation |
|---|---|
| 1. Shaders | **Video blocks:** validate with the `naga` that already ships inside wgpu (`wgpu::naga`), using one fixed capability set. Blocks supply only functions; the host writes all bindings and entry points.<br>**Pixel effects:** fuse them into one fragment shader by text concatenation with host-assigned name prefixes.<br>**Loop limit:** add a host "loop budget" by rewriting the naga IR. A working prototype is measured in 1.6.<br>**After a lost device:** quarantine the block before any retry.<br>**Compiling:** do it lazily on a worker thread and cache in memory, keyed by a hash of the composed source. |
| 2. Audio sandbox | **wasmi 2.0.0** (pure-Rust interpreter, MIT/Apache-2.0). Turn on fuel, use no imports, turn off SIMD and relaxed SIMD, and enable the `deterministic` feature. |
| 3. Rust → wasm | Use the **`wasm32v1-none`** target (not installed here). Build blocks in a separate workspace with a pinned toolchain. Check the `.wasm` files in, with a staleness test like `schema::stale`. |
| 4. Packages | **zip 9.1.3** with `default-features = false`. Read entries into memory only; never extract by name. Apply strict count, size, ratio, name and symlink limits. Identity is a BLAKE3 hash. No signing in v1. |
| 5. Text | **cosmic-text 0.19** (shaping via harfrust, rasterising via swash) into a CPU atlas the renderer samples. **glyphon 0.12** already targets wgpu 30. |
| 6. Prior art | Borrow ISF/DCTL's "host owns the kernel, block declares inputs", OpenFX's region-of-interest and time declarations, CLAP's stable parameter ids and latency/tail, LV2's "metadata readable without running code", and the browsers' "device loss is routine, and blocks get blamed". |

---

## 1. Shaders (wgpu 30.0.1 / naga 30.0.1)

### 1.1 What the code does today

- **Versions:** `wgpu 30.0.1` (Cargo.lock:5236-5237) and `naga 30.0.1` (Cargo.lock:2532-2533). `render` depends on `wgpu = "30"` (crates/render/Cargo.toml:14). naga is not a direct dependency.
  - wgpu re-exports it as `wgpu::naga` (`$REG/wgpu-30.0.1/src/lib.rs:318-325`).
  - naga features compiled in this build (`cargo tree -p render -e features -i naga`): `wgsl-in`, `spv-out`, `hlsl-out`, `glsl-out`. `wgsl-out` is off.
  - wgpu's `naga-ir` feature (`$REG/wgpu-30.0.1/Cargo.toml:75`) enables `ShaderSource::Naga(Cow<'static, naga::Module>)` (`$REG/wgpu-30.0.1/src/api/shader_module.rs:208-210`). Both are flags on crates already locked, not new crates.
- **One shared device:**
  - Features are empty and limits are the adapter's own (crates/render/src/gpu.rs:194-200).
  - Uncaptured errors are only printed (gpu.rs:205). The device-lost callback sets a flag (gpu.rs:206-208).
  - Errors are captured with error scopes (gpu.rs:157-171, 312-317). A lost device is replaced and kept, not destroyed (gpu.rs:13-16, 58-86).
- **Shaders are fixed at device creation:** two pipelines built with `include_str!` (gpu.rs:210-216).
- **Automatic retry after device loss:** `with_recovery` re-runs the failed frame once on a new device (crates/render/src/renderer.rs:447-479). Frames wait up to `FRAME_TIMEOUT = 60 s` (renderer.rs:22-25, 652).
- **The layer shader fuses everything.** Crop, sRGB decode, grade, mask and opacity are one fragment shader (crates/render/shaders/layer.wgsl:99-113), with parameters in one uniform struct (layer.wgsl:5-22).
- **The effect stack is folded, not applied in order.** The graph merges nodes (crates/render/src/graph.rs:228-242):
  - the last Transform or Crop wins;
  - opacity multiplies;
  - ColorMatrix composes;
  - Mask is replaced.

  A plug-in "stack in order" model will not be pixel-identical for stacks with two Transforms, or a Mask before a ColorMatrix, unless the migrated built-ins keep these folding rules.
- **Existing closed list:** a `Shader` kind already exists in the closed `EffectKind` list (crates/core-model/src/effect.rs:13-30). Its catalogue entry is "Custom WGSL fragment shader with four float uniforms" (crates/effects-registry/src/lib.rs:209-215). The renderer does not draw it, nor Blur, LUT or Text (crates/render/src/lib.rs:10-12).

### 1.2 Parsing and validating untrusted WGSL with naga 30.0.1

```rust
use wgpu::naga; // same crate wgpu uses; no new dependency
let module = naga::front::wgsl::parse_str(src)?;                 // front/wgsl/mod.rs:86-88
let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),                      // valid/mod.rs:60
        naga::valid::Capabilities::empty())                       // valid/mod.rs:88
    .validate(&module)?;                                          // valid/mod.rs:733-739
```

- **Capabilities.** `front::wgsl::Options.capabilities` defaults to `Capabilities::all()` (`$REG/naga-30.0.1/src/front/wgsl/parse/mod.rs:283-298`), so the parser accepts every `enable`. The **Validator** enforces capabilities: it "will reject any Module that would use capabilities not included" (valid/mod.rs:590-595). `Capabilities::default()` = `MULTISAMPLED_SHADING | CUBE_ARRAY_TEXTURES` (valid/mod.rs:259-262).
  - Use one fixed set (empty or default) on every machine, so a block accepted on one machine is accepted on all.
  - wgpu-core derives its own set from the device features (`$REG/wgpu-core-30.0.1/src/device/resource.rs:2373-2378`, then 2445-2450 via `wgpu-naga-bridge` `create_validator`, `$REG/wgpu-naga-bridge-30.0.1/src/lib.rs:189-197`). Since the project requests no features (gpu.rs:196), f16, subgroups and similar are rejected there too.
- **Stack space.** Parser recursion is capped at 200 (parse/mod.rs:343-352). Both naga and wgpu still warn that parsing "may consume a lot of stack space … particularly if … exposed to user input" (front/wgsl/mod.rs:75-85; `$REG/wgpu-30.0.1/src/api/device.rs:124-136`). Parse and create modules on a worker thread with a large stack (for example 8 MiB).
- **No recursion.** The WGSL frontend rejects recursive and cyclic declarations (front/wgsl/index.rs:137, 153). The validator requires a `CallResult` to name an earlier function (valid/handles.rs:643-647). So the call graph is acyclic and **loops are the only unbounded construct**.
- **Always use the checked path.** `Device::create_shader_module` uses `ShaderRuntimeChecks::checked()` (api/device.rs:139-144). `create_shader_module_trusted` is `unsafe` and must never be used for blocks (api/device.rs:165-186).

### 1.3 What wgpu and naga guarantee

`ShaderRuntimeChecks` (`$REG/wgpu-types-30.0.1/src/shader.rs:6-59`; `checked()` turns all on, 64-66) has these fields: `bounds_checks`, `force_loop_bounding`, `ray_query_initialization_tracking`, `task_shader_dispatch_tracking`, `mesh_shader_primitive_indices_clamp`, `int_div_checks` (guards divide-by-zero and `INT_MIN/-1`).

Bounds-check policy as compiled per backend:

| Backend | Indexing / buffers / image loads | Source |
|---|---|---|
| Vulkan | index `Restrict`; buffer `Restrict`, or hardware robustBufferAccess2; image `Restrict`, or robustImageAccess; binding arrays `Unchecked` | `$REG/wgpu-hal-30.0.1/src/vulkan/adapter.rs:2838-2851`. RBA2 is turned off on outdated Intel drivers: adapter.rs:3421 |
| Metal | `Restrict` when `bounds_checks` | wgpu-hal metal/device.rs:185-189 |
| DX12 | `restrict_indexing = bounds_checks` | wgpu-hal dx12/device.rs:293, 310 |
| GL/GLES | index and buffer **`Unchecked`**; image `ReadZeroSkipWrite` only on desktop GL ≥ 4.3 | wgpu-hal gles/device.rs:333-347 ("provided by glsl or not implemented yet") |

What the policies mean (`$REG/naga-30.0.1/src/proc/index.rs:7-60`): `Restrict` uses some in-bounds index; `ReadZeroSkipWrite` reads zero and drops the write. The WebGPU spec (§2.1.4) allows out-of-bounds reads to return another in-bounds value or zeros, and writes to land elsewhere in bounds or be dropped.

**Protected:** array, vector and matrix indexing; uniform and storage buffer access; `textureLoad`/`textureStore` coordinates; integer division.

**Not protected:**
- binding arrays (`Unchecked` everywhere), so blocks must never declare bindings;
- the GL backend in general, so disable untrusted blocks on GL;
- running time.

**`force_loop_bounding` does not prevent hangs.** It counts down from `u32::MAX` in a `uint2`, about 2^64 iterations (hlsl/writer.rs:195-221; SPIR-V back/spv/block.rs:280-300). It only stops compilers treating infinite loops as undefined behaviour.

### 1.4 What is not protected: long or infinite shaders

| OS | Behaviour | Source |
|---|---|---|
| Windows | TDR: the scheduler tries to preempt; "default timeout period … two seconds". With pixel-level preemption, a pixel already running cannot be interrupted. On recovery all video memory is purged and every D3D app must recreate its device. **5 or more hangs within 1 minute → bug-check 0x117 on the 6th** (TdrLimitCount 5, TdrLimitTime 60). Engine-reset timeouts can block the offending process's GPU access (0x142). | [MS TDR](https://learn.microsoft.com/en-us/windows-hardware/drivers/display/timeout-detection-and-recovery), [TDR keys](https://learn.microsoft.com/en-us/windows-hardware/drivers/display/tdr-registry-keys), [preemption granularity](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmdt/ne-d3dkmdt-_d3dkmdt_compute_preemption_granularity) |
| macOS | The command buffer is aborted with `kIOAccelCommandBufferCallbackErrorTimeout` (code 2); the duration is undocumented. After earlier faults, later submissions are refused with `kIOGPUCommandBufferCallbackErrorSubmissionsIgnored` ("for causing prior/excessive GPU errors", code 4). | [MoltenVK #2080](https://github.com/KhronosGroup/MoltenVK/discussions/2080), [Apple forum 762613](https://developer.apple.com/forums/thread/762613), [Unity tracker](https://issuetracker.unity3d.com/issues/ios-error-execution-of-the-command-buffer-was-aborted-due-to-an-error-during-execution-ignored-for-causing-prior-slash-excessive-gpu-errors-00000004-kiogpucommandbuffercallbackerrorsubmissionsignored-when-drawing-a-mesh-with-graphics-dot-drawmeshnow) |
| Linux | Depends on the driver:<br>• amdgpu `lockup_timeout` defaults to 2000 ms for all queues (current docs), `gpu_recovery` -1 (auto).<br>• i915 preempt timeout 640 ms, heartbeat 2500 ms.<br>• NVIDIA: Xid 8 "idle timeout" when a long job blocks a GPU that drives a display (forum reports of about 7.5 s). | [amdgpu params](https://docs.kernel.org/gpu/amdgpu/module-parameters.html), [i915 Kconfig.profile](https://kernelsources.org/source/xref/linux/drivers/gpu/drm/i915/Kconfig.profile), [NVIDIA forum](https://forums.developer.nvidia.com/t/ubuntu-22-04-1-xid-8-error-causing-x-to-freeze/290765) |
| Spec | "watchdog … for more than a few seconds"; device loss is the fallback. | [WebGPU §2.1.9](https://www.w3.org/TR/webgpu/) |

**How wgpu reports a loss.** The hardware layer maps `DXGI_ERROR_DEVICE_REMOVED/RESET` to `DeviceError::Lost` (wgpu-hal auxil/dxgi/result.rs:15-18) and `VK_ERROR_DEVICE_LOST` to `Lost` (vulkan/mod.rs:1592-1595). wgpu-core then marks the device invalid (wgpu-core resource.rs:698-707, 5415-5420). The callback receives `DeviceLostReason::{Unknown, Destroyed}` (wgpu-types lib.rs:554-559). It runs late, on the polling thread (gpu.rs:264-272), and **never says which shader caused the loss**.

**Can a hung shader freeze the desktop?** Yes, for up to the timeout (about 2 s on Windows). Every GPU app also loses its device, including the WebView2 interface.

**Critical interaction with today's code.** A hanging block today would:
1. hang the GPU;
2. trigger a loss and automatic re-draw (renderer.rs:466-476);
3. hang again on the next frame, since the player redraws every frame.

That loop can reach Windows' 5-hangs-in-60-s bug-check. Required:
- after any loss while untrusted blocks were in the submission, quarantine them **before** retrying;
- write a crash-safe "blocks in flight" marker to disk, so a crash or blue screen disables the block at the next start;
- add a slow-frame tripwire well under 2 s (for example 250 ms at preview size), instead of relying on the 60 s `FRAME_TIMEOUT`.

### 1.5 Combining host WGSL with block WGSL

| Option | Pros | Cons |
|---|---|---|
| **A. Text concatenation with host-assigned prefixes** | No dependency. | A light lexer must rename every module-scope name. It must not rename members after `.`, struct-body members, or `@` attributes. |
| B. `naga_oil` 0.23.0 (2026-07-14, MIT/Apache, naga 30) | Real modules: `#import`, `#define_import_path`, `virtual`/`override` fns, outputs a `naga::Module`. | New dependencies (regex, data-encoding, indexmap…). Turns on naga `wgsl-out` (and `glsl-in/out` unless default features are off). One release per naga major, so updates are coupled. |
| C. Merging naga IR modules by hand | Most robust. | Arena and handle remapping for types, constants and expressions: hundreds of lines. |

**Recommendation: A**, with these rules:

1. **Validate the block alone first.** Parse it after a stub prelude that declares the host API (for example `vep_sample(uv) -> vec4<f32>`, implemented with `textureSampleLevel`). Then check its IR:
   - no `global_variables` or `entry_points` beyond the prelude's;
   - no `enable` directives;
   - the required `main` signature;
   - no names starting with `vep_`.
2. **Compose.** The host generates the per-block parameter struct from the manifest, the bindings, the entry point and the call chain.
3. **Validate again,** apply the loop budget (1.6), and pass the result as `ShaderSource::Naga`.

### 1.6 Loop analysis: is it feasible? (measured prototype)

- **Static rejection** of unbounded loops cannot be done in general (it is the halting problem). It is feasible only for a narrow pattern: loop variable from a constant, compared to a constant, raised by a constant in `continuing`, never written in the body. That pattern is brittle.
- **Dynamic budget** (prototyped):
  - The prelude declares `var<private> vep_budget: u32 = N;`.
  - An IR pass puts `if vep_budget == 0 { break; } vep_budget -= 1;` at the top of every `Statement::Loop` body.
  - The rule for which expressions need `Emit` is at proc/mod.rs:342-353.
  - Because the global is per-invocation and shared across calls, **total** loop iterations per invocation are ≤ N, including nested loops and loops inside called functions.

Results (plugbench `loops`, Vulkan on the RTX A4000; 6 loops rewritten in 0.009 ms; naga validation passes; budget N = 1000):

| Shader (one fragment) | Without budget | With budget |
|---|---|---|
| `for i < 100000` | 100000 | 1000 |
| nested 1000 × 1000 | 1,000,000 | 999 |
| loop calling a function with a loop, 1000 × 1000 | 1,000,000 | 999 |
| `while i < 100000` | 100000 | 1000 |

- **Cost of the guard:** a 9×9 blur at 1920×1080 into Rgba16Float went from 0.64 to 0.65 ms (Vulkan) and from 0.65 to 0.73 ms (DX12).
- **Uniformity:** `textureSample` inside a loop still validates after the rewrite. Implicit derivatives in divergent control flow are still unreliable on hardware, so give blocks only explicit-level sampling.
- **The budget bounds iterations, not total work.** Also needed:
  - decrement by a static cost of the loop body (texture samples weighted heavily) instead of 1;
  - a per-pixel static cost cap that includes straight-line code;
  - tiling of 4K export passes, so each submission stays far below 2 s on a weak GPU.

### 1.7 Practical limits to enforce (suggested starting values)

| Item | Limit |
|---|---|
| WGSL source | ≤ 64 KiB per block; ≤ 64 functions |
| Declarations allowed | functions, consts, structs, aliases only. No globals, bindings, entry points, overrides, `enable`, storage buffers, atomics, workgroup memory, barriers, derivatives (`dpdx`/`fwidth`) or `discard` |
| Texture inputs | host-bound only; ≤ 4 per block (WebGPU minimum per stage is 16) |
| Parameters | ≤ 64 vec4 (1 KiB) per block, host-packed; ≤ 16 KiB per layer |
| Passes | point blocks fused into the layer pass. Neighbourhood blocks (blur): own pass, declared region-of-interest radius ≤ 64 px, ≤ 4 passes per block, Rgba16Float intermediates |
| Loop budget | for example 1024 cost units per invocation, plus a static per-pixel cap |
| Backends | no untrusted blocks on GL; validate against a fixed capability set (Capabilities::empty() or default), independent of the device's own limits (gpu.rs:197 uses the adapter's limits) |

### 1.8 Is runtime pipeline creation fast enough? (measured, ms)

Method: 8 cold runs per shader; the source differs each run to defeat caches. "Module" means `create_shader_module`.

| Shader | naga parse | naga validate | module | pipeline (Vulkan) | pipeline (DX12, FXC) | pipeline (WARP software) | same source again |
|---|---|---|---|---|---|---|---|
| project layer.wgsl (4.3 KB) | 0.34 | 0.06 | 0.34 | 11.2 | 21.8 | 11.8 | 0.35 / 0.82 / 0.80 |
| 3 fused point blocks | 0.16 | 0.03 | 0.15 | 6.0 | 9.8 | 4.9 | 0.2–0.6 |
| 9×9 blur loop | 0.12 | 0.02 | 0.10 | 4.8 | 26.6 | 12.3 | 0.2–0.6 |
| 24 fused point blocks (10 KB) | 1.03 | 0.19 | 1.07 | 19.3 | 40.3 | 21.0 | 0.7–1.3 |

- **Lazy compiling works:** 5–40 ms per pipeline, done on a worker thread. The device is shared across threads (gpu.rs:1-11).
  - Preview shows the frame without the block (or holds the last frame) until the pipeline is ready.
  - Export waits for it.
  - Compile ahead at project open for the blocks the project uses.
- **FXC risk on DX12.** No `dxcompiler.dll` is on PATH here, so wgpu fell back to FXC (`Dx12Compiler::Auto`, `$REG/wgpu-types-30.0.1/src/backend.rs:777-804`). FXC unrolls loops (the blur took 26 ms) and can take more than 10 s on some shaders ([Mozilla 1880415](https://bugzilla.mozilla.org/show_bug.cgi?id=1880415), [1885217](https://bugzilla.mozilla.org/show_bug.cgi?id=1885217)). Compile time is therefore also a denial-of-service vector: time out compiles and treat a slow compile as a refusal. Consider shipping DXC or preferring Vulkan.
- **Caching:**
  - In-memory map `blake3(composed WGSL + entry point + target format) → RenderPipeline`, per device, rebuilt after a device loss.
  - DX12 already has an in-memory compile cache (wgpu-hal dx12/mod.rs:1334-1339, device.rs:390-437). On Vulkan the driver's own cache gave about 0.35 ms for repeats.
  - wgpu's `PipelineCache` is `unsafe` (api/device.rs:726-732), Vulkan-only (wgpu-types features.rs:1163-1172; `util::pipeline_cache_key` util/mod.rs:163-175), and needs a feature the project does not request.

### 1.9 Pixel-exact migration (schema v2)

- GPU float results differ across devices and backends, so "pixel for pixel" can only be checked on the same machine, driver and backend.
- Built-in point effects must stay **in one fused pass in the same order**: crop, then sRGB decode, then grade, then mask and opacity (layer.wgsl:99-113). The folding rules in graph.rs:228-242 must also stay. Splitting into passes changes rounding (fp16 intermediates) and can change FMA contraction.
- Existing renderer tests compare with tolerances of ±1–2 (crates/render/tests/render.rs:57-63). A new byte-exact old-versus-new golden test is needed per backend, including WARP.

---

## 2. Audio sandbox: wasmi vs wasmtime

| | **wasmi** | **wasmtime** |
|---|---|---|
| Version | 2.0.0, 2026-09-01 ([crates.io](https://crates.io/crates/wasmi)) | 50.0.0, 2026-10-20 ([crates.io](https://crates.io/crates/wasmtime)) |
| Licence | MIT/Apache-2.0 | Apache-2.0 WITH LLVM-exception |
| Kind | Pure-Rust interpreter. New engine in 2.0, "~2.2x faster than Wasmi 1.0", "on par with Wasm3" ([blog](https://wasmi-labs.github.io/blog/posts/wasmi-v2.0)) | Cranelift JIT (also Winch, and the Pulley interpreter at Tier 2) |
| Platforms | Anything Rust compiles to; no executable memory | Tier 1: x86_64 Windows, macOS, Linux. **aarch64-apple-darwin is Tier 2** ([tiers](https://docs.wasmtime.dev/stability-tiers.html)) |
| macOS notarisation | Nothing needed | JIT needs `MAP_JIT` under hardened runtime, so the `com.apple.security.cs.allow-jit` entitlement ([Apple](https://developer.apple.com/documentation/xcode/configuring-the-hardened-runtime), [wasmtime #12076](https://bytecodealliance.github.io/zulip-archive/stream/225357-git-wasmtime/topic/wasmtime.20.2F.20issue.20.2312076.20Cranelift.3A.20Non-deterministic.20JIT.20.2E.2E.2E.html)) |
| Binary size | Not measured. Turning off `validate` saves "200-300kB" (changelog), so likely around 1 MB | C-API release with default features 19 MB; runtime without compiler 2.1 MB ([minimal](https://docs.wasmtime.dev/examples-minimal.html)) |
| Time cap | `Config::consume_fuel` gives a deterministic `OutOfFuel` trap. Fuel is tied to the input bytecode in 2.0; `Store::set_fuel` | Fuel (deterministic). Epoch interruption is "up to 2-3x" faster but "not deterministic" ([Config](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html)) |
| Memory | `StoreLimitsBuilder::memory_size`/`table_elements`/`instances`; `EnforcedLimits::strict()` for parsing | `Store::limiter`; reserves 4 GiB plus a 32 MiB guard per memory |
| Imports | Empty `Linker`; reject any `Module::imports()`; `allow_start_fn(false)`. WASI is a separate crate | Same; WASI is a separate crate |
| Determinism | `deterministic` crate feature (Wasm deterministic profile: canonical NaNs); turn off SIMD and relaxed SIMD | `cranelift_nan_canonicalization`, `relaxed_simd_deterministic`, fuel ([guide](https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html)) |
| Audits | SRLabs (0.31), Runtime Verification (0.36–0.38) | Continuous fuzzing on Tier 1 |

**Throughput.**

Native baseline, measured with dspbench on the i9-13900:

| Workload | Time per 1 s of 48 kHz stereo | Per unit |
|---|---|---|
| 3-band EQ (6 biquads) | 0.143 ms | 1.49 ns per sample per channel |
| Compressor (`log10` + `powf` per frame) | 0.69 ms | 14.4 ns per frame |

Interpreters versus native, from published benchmarks:
- Wasm3's CoreMark score is 11–12× below native and 4× below 2019-era wasmtime ([wasm3 Performance.md](https://github.com/wasm3/wasm3/blob/main/docs/Performance.md)).
- wasmi 2.0 claims parity with Wasm3.

So, as an estimate (wasmi itself was not run, because it is not in the local cache):

| Workload | Estimated time per 1 s of audio | Share of one core |
|---|---|---|
| EQ | about 1.5–2.2 ms | about 0.2% |
| Compressor | about 7–10 ms | about 1% |

The JIT would be close to native. Either is fine for music-only hype edits.

**What the host must do for preview and export to match:**
- The player mixes in 0.25 s chunks (ui/src-tauri/src/player.rs:647-656); export mixes in 1 s chunks (crates/export/src/audio_mix.rs:55-68). Even today the two differ by up to 0.04 at chunk seams (audio_mix.rs:133-135).
- Today the mix is stateless per piece: gain and pan are computed once (crates/audio/src/mixer.rs:111-131). The default spec is 48 kHz stereo (mixer.rs:9-17).
- Stateful blocks therefore need:
  - one instance per clip and block;
  - processing in fixed sub-blocks aligned to absolute sample positions (for example 128 frames), with fuel granted per sub-block, so traps do not depend on chunking;
  - parameters sampled at sub-block boundaries;
  - a declared pre-roll/tail used after seeks.
- On a trap: bypass the block, report it, and never crash.
- Determinism: transcendental functions must be compiled into the module (the `libm` crate), never imported from the host, because each OS's libm differs in the last digits.
- Wasm requires IEEE subnormals ([spec](https://webassembly.github.io/spec/core/exec/numerics.html)). IIR tails can hit slow denormals, so the SDK should flush small states.

**Recommendation: wasmi 2.0.0.**
- **Features:** `default-features = false`, plus `std`, `validate` and `deterministic`. Leave out `simd`. Add `wat` only if text blocks are wanted.
- **Config:** `consume_fuel(true)`, `wasm_simd(false)`, `wasm_relaxed_simd(false)`, `wasm_memory64(false)`, `wasm_multi_memory(false)`, `allow_start_fn(false)`, `enforced_limits(EnforcedLimits::strict())`.
- **Store:** `StoreLimits` memory_size ≤ 16 MiB.

Reasons: deterministic fuel, no JIT entitlement or W^X issues on macOS, equal footing on M1, small size and build. Wasmtime's speed advantage does not matter at these loads.

---

## 3. Built-in audio blocks in Rust → wasm

- **Not installed.** `rustup target list --installed` shows only `x86_64-pc-windows-msvc`. `rust-src` is not installed. No clang, zig, wat2wasm, wasm-opt or wasm-tools on PATH. `rust-lld.exe` is present in the toolchain.
- **Target: `wasm32v1-none`** (Tier 2). It "imports nothing from its host", enables only Wasm 1.0 plus mutable-globals, is `no_std` (core and alloc), and works on stable ([doc](https://doc.rust-lang.org/rustc/platform-support/wasm32v1-none.html)).
  - `wasm32-unknown-unknown` turns on multivalue, reference-types, sign-ext, and also nontrapping-fptoint and bulk-memory since Rust 1.87 ([doc](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)).
  - Install with `rustup target add wasm32v1-none`.
- **Size (estimate, not measured here):**
  - A biquad EQ module with `opt-level="s"`, LTO, `panic=abort` and `strip`: on the order of 1–2 KB.
  - Using `libm` powf/log10 for a compressor: a few KB more.
  - Linear memory matters more than file size. Rust links a **1 MiB stack by default** (`-z stack-size=1048576`, [rustc wasm spec](https://rust.googlesource.com/rust/+/HEAD/compiler/rustc_target/src/spec/base/wasm.rs)), so each instance starts at ≥ 17 pages (about 1.06 MiB). Use `-C link-arg=-zstack-size=65536` and `--max-memory`, and have the host reject modules whose initial memory is large.
- **Reproducible build and check-in:**
  - Put blocks in a separate `blocks/` cargo workspace. It is outside the `crates/*` member glob (Cargo.toml:3).
  - Pin the toolchain in a `rust-toolchain.toml` there. Use `--remap-path-prefix` and `panic=abort` so no paths are embedded.
  - Check in the `.wasm` file plus a recipe hash (BLAKE3 of sources, Cargo.lock, toolchain version and flags).
  - Add a staleness test modelled on `schema::stale` (ui/src-tauri/src/schema.rs:250-274; test at ui/src-tauri/tests/bindings.rs:26-31). It compares recipe hashes always, and rebuilds and compares bytes only when the pinned toolchain is present.
  - Golden audio tests should run through wasmi, not natively.
- **Toolchains for outside authors:**
  - **Rust:** rustup plus `wasm32v1-none`, plus a small host SDK crate.
  - **AssemblyScript** (Apache-2.0, needs Node): use the stub runtime and `--use abort=…`, because the default build imports `env.abort` ([example](https://t-katsumura.github.io/webassembly-examples-eratosthenes/assemblyscript/build-wasm)).
  - **C:** upstream LLVM clang `--target=wasm32 -nostdlib -Wl,--no-entry`. Apple's Xcode clang has no wasm-ld.
  - **Zig** (MIT, one download, also compiles C): `-target wasm32-freestanding -fno-entry -rdynamic`.
  - **Hand-written WAT:** via the `wat` crate.
  - **Faust:** has a wasm backend, but its output imports math functions from the host, so it would need deterministic host math imports or must be rejected.

---

## 4. Block packages

- **zip 9.1.3** (2026-10-27, MIT, MSRV 1.88). Releases are very frequent (9.0.5 to 9.1.3 in 10 days), so pin and review. Default features pull in aes, bzip2, deflate64, lzma, ppmd, time, zstd and xz; use `default-features = false, features = ["deflate-flate2-zlib-rs"]`.
- **Path safety.** `enclosed_name()` rejects NUL, absolute paths and escapes ([ZipFile](https://docs.rs/zip/latest/zip/read/struct.ZipFile.html)). A symlink-based escape in `extract` was CVE-2025-29787, fixed in 2.3.0. Safest rule: **never extract to disk.** Read whitelisted names with `by_index` into memory, and reject `is_symlink()`, encrypted entries, and any compression other than Stored or Deflate.
- **Bomb limits:**
  - archive ≤ 16 MiB;
  - `len()` ≤ 64 entries, checked before iterating;
  - declared `size()` ≤ 4 MiB per file and ratio ≤ 100:1;
  - always read through `Read::take(limit + 1)`, because declared sizes can lie;
  - total ≤ 16 MiB;
  - reject overlapping files (`has_overlapping_files`, [ZipArchive](https://docs.rs/zip/latest/zip/read/struct.ZipArchive.html)) and case-insensitive duplicate names.
- **Identity.** BLAKE3 of the archive bytes; blake3 is already a workspace dependency (Cargo.toml:20).
- **Signing.** With no store there is no trust root. A signature only proves "same publisher as last time" and transport integrity, not safety; the sandbox is the security boundary. Skip it in v1. Later, for update continuity or an "official ChadFX" key built into the app: minisign-verify 0.3.0 (MIT, no dependencies) or ed25519-dalek 3.0.0 (BSD-3-Clause). Built-in blocks are trusted through the app's own code signature.

---

## 5. Text, for a future host "text" capability

| Crate | Version (date) | Licence | Role |
|---|---|---|---|
| cosmic-text | 0.19.0 (2026-04-22) | MIT OR Apache-2.0 | Multi-line layout; shaping via harfrust 0.5; fonts via fontdb 0.23; rasterising via swash 0.2 (default feature); uses skrifa 0.40 |
| swash | 0.2.10 (2026-07-17) | Apache-2.0 OR MIT | Shaping, scaling, rasterising (colour glyphs too) |
| harfrust | 0.15.0 (2026-10-22) | MIT | HarfBuzz port; replaces rustybuzz in cosmic-text |
| rustybuzz | 0.20.1 (2024-11-12) | MIT | Older port, no release since 2024 |
| fontdue | 0.9.5 (2026-10-19) | MIT OR Apache-2.0 OR Zlib | Fast rasteriser, simple layout, no shaping |
| ab_glyph | 0.2.32 (2025-09-28) | Apache-2.0 | Outline loading and rasterising, no shaping |
| skrifa | 0.49.0 (2026-10-21) | MIT OR Apache-2.0 | Glyph scaler (fontations) |
| parley | 0.12.0 (2026-10-09) | Apache-2.0 OR MIT | Rich text layout |
| glyphon | 0.12.0 (2026-07-09) | MIT OR Apache-2.0 OR Zlib | wgpu text renderer: **wgpu 30.0.0 + cosmic-text 0.19**, etagere atlas |

**Design room to leave:** the host lays text out and rasterises it on the CPU (cosmic-text plus swash). That output is deterministic for the same font bytes and crate versions. The host uploads it to an R8/RGBA atlas, or a ready-made text layer, that blocks may *sample* through a host function. Per-word glyph positions from the layout drive word-by-word caption pop-in. Bundle OFL fonts so rendering does not depend on system fonts.

---

## 6. Prior art worth copying

- **Browser WebGPU (Chrome/Dawn, Firefox/wgpu):** untrusted WGSL is always validated and bounds-checked; GPU work runs in a GPU process; a watchdog produces device loss ([spec §2.1.9](https://www.w3.org/TR/webgpu/)). Borrow: device loss is a routine event that is attributed and leads to quarantine; never auto-retry blindly.
- **ISF:** a JSON header declares `INPUTS` (NAME, TYPE, DEFAULT, MIN, MAX, LABEL) and `PASSES` (TARGET, PERSISTENT, FLOAT, WIDTH/HEIGHT expressions) ([ref](https://docs.isf.video/ref_json.html)). Borrow: manifest-declared parameters and passes.
- **DaVinci DCTL:** the block is a pure per-pixel `transform(...)` function plus `DEFINE_UI_PARAMS`; the host owns the kernel ([ref](https://github.com/CommandPost/ResolveCafe/blob/main/docs/developers/dctl.md)).
- **Godot VisualShaderNodeCustom:** typed ports, and `_get_code(input_vars, output_vars)` returns a snippet the host wires in ([ref](https://docs.godotengine.org/en/stable/classes/class_visualshadernodecustom.html)). Borrow host-generated wiring.
- **Bevy naga_oil:** a module system with name isolation and `virtual`/`override` functions ([ref](https://github.com/bevyengine/naga_oil)). Borrow the concept, not necessarily the dependency.
- **OpenFX:** region-of-definition and region-of-interest actions, plus frames-needed ([ref](https://openfx.readthedocs.io/en/main/Reference/ofxProcessingArch.html)). Plug-ins are native and unsandboxed, so borrow only the declarations: RoI radius, temporal access, time-varying.
- **Blender:** compositor nodes are built in; add-ons extend through node groups. Borrow "a preset block is a graph of blocks, no code".
- **CLAP:** a parameter id "must never change"; flags STEPPED, PERIODIC, HIDDEN, BYPASS, ENUM; latency and tail extensions ([params.h](https://github.com/free-audio/clap/blob/main/include/clap/ext/params.h)).
- **LV2:** metadata lives in Turtle files separate from the binary, so hosts scan without loading code ([book](https://lv2plug.in/book)). Borrow: the manifest is readable without running anything; parameter ranges and units are declarative.
- **WAM 2.0:** JSON descriptor, `getParameterInfo`, sample-accurate `scheduleEvents` ([API](https://www.github.com/webaudiomodules/api)).

---

## 7. Final recommendations per question

1. **Shaders.**
   - Use `wgpu::naga` 30.0.1 with a fixed capability set, on a big-stack worker thread.
   - Blocks contribute functions only; the host composes one fused pass for point blocks by prefixed concatenation, and gives neighbourhood blocks their own passes with a declared RoI.
   - Bound loops with the IR loop budget (proven above), plus a static cost cap and export tiling.
   - Never use `create_shader_module_trusted`; disable untrusted blocks on GL.
   - Change device-loss handling to quarantine first, with a crash-safe in-flight marker and a slow-frame tripwire well under 2 s.
   - Compile lazily (5–40 ms measured), cache in memory by hash, compile ahead at project open, and consider DXC on Windows.
2. **Audio.** wasmi 2.0.0 with the configuration above (no imports, deterministic fuel per fixed sub-block, no SIMD, `deterministic` feature, a memory cap). Host-managed state per clip and block, chunk-invariant processing, a declared tail/pre-roll.
3. **Rust blocks.**
   - `wasm32v1-none` after `rustup target add`.
   - Small stack via a linker flag.
   - Separate pinned workspace; checked-in `.wasm` plus a recipe-hash staleness test.
   - Document Rust, AssemblyScript (`--use abort=`), Zig and clang for outsiders.
4. **Packages.**
   - zip 9.1.3 without default features.
   - In-memory reads of whitelisted names only.
   - Count, size, ratio, symlink, encryption and overlap checks.
   - BLAKE3 identity; no signing in v1.
5. **Text.** cosmic-text 0.19 plus swash into a CPU atlas exposed as a host capability. glyphon 0.12 (wgpu 30) as a reference or direct use.
6. **Prior art.** ISF/DCTL-style declarative manifests with the host owning entry points; OpenFX RoI and temporal flags; CLAP stable ids and latency/tail; LV2 code-free metadata; browser-style device-loss quarantine.