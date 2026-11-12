# No-GPU picture proof: ng-review-platform.md

## findings

- **title:** The same recipe gave different bytes on different thread counts, and between two runs on the same count
  **severity:** critical
  **proven:** true
  **fixed:** true
  **detail:** WGSL forbids a `continue` that goes past a `let` which the loop's `continuing` block reads. naga accepts it, parser and validator alike. On that path the read took whatever another pixel had left in that register. A 256x256 scratch recipe gave FNV hashes 250c75d1... on 1 thread, 247b01c3... on 2, 6f106cc5... and 444438d9... on 7 (two runs), e317b955... and 013d2e31... on 16. Function results and the `break if` condition did the same. Fix in crates/cpu-render/src/compile.rs: a function that reads such a value now gives `Error::Unsupported` naming it when the function is reached. The check works from the program text, so it does not matter whether a run takes that path. Kept as tests: a_value_a_continue_went_past_is_refused (10 shapes), every_other_loop_is_not_disturbed, the_check_is_on_the_program_and_needs_nothing_to_run. To look for more of this kind, I ran all of cpu-render's tests once on a throwaway copy with every word above a program's arguments overwritten before each run, and everything passed. Those tests include the 351 scenes, the composed modules and the WGSL operator tests. That run is not a permanent test.
- **title:** The check that refuses platform-folded constants could be hidden from, so the hole it closes was still open
  **severity:** critical
  **proven:** true
  **fixed:** true
  **detail:** The check compares the text with the IR, but its look through the text disagreed with naga's lexer. A `//` comment ended only at line feed for the scan, while naga also ends it at VT, FF, CR, U+0085, U+2028 and U+2029. So `// note\rconst K = sin(0.5);` was ACCEPTED with sin folded by the parser. A file with old Mac line endings would hide every call in it. Blanks between the name and the bracket also hid the call: U+0085, U+200E, U+200F, U+2028, U+2029 and VT. Twelve forms were proven accepted in crates/cpu-render/src/front.rs. Fix: comment ends and blanks now match naga's lists exactly. Two new tests try every character up to U+3100 against naga itself, plus six characters from higher up, and pin naga's own lists.
- **title:** The same hole hid the limits, so one shader could end the whole process with a stack overflow
  **severity:** critical
  **proven:** true
  **fixed:** true
  **detail:** The limits that protect naga's recursion use the same scan. `// note\r` followed by a million `+ 1.0` made every limit count nothing, and the process died with exit code 3221225725 (0xC00000FD, stack overflow), which no catch_unwind can stop. The same text without the `\r` was refused cleanly. After the fix it is refused with an ordinary error (measured). Test: the_limits_see_past_every_end_of_a_comment.
- **title:** radians, degrees, clamp and saturate were accepted when folded, though they use std calls the hard rules ban
  **severity:** high
  **proven:** true
  **fixed:** true
  **detail:** naga folds them with `to_radians`, `to_degrees` and `f32::clamp`. Rust 1.99's own documentation says the first two have precision that 'varies by platform, Rust version', and says `clamp` returns either zero non-deterministically when zeros of two signs meet. The README claimed these have one right answer. `clamp(-0.0, 0.0, 1.0)` and `saturate(-0.0)` were accepted and folded to -0, where this crate's rule says +0. All four are now on the refused list, with two narrow exceptions that are needed. `radians(0.0)` stays allowed because crates/blocks/src/wgsl/compose.rs:191 writes it for every clip with no transform, and zero times any factor is zero. Calls whose arguments are all plain numbers with no minus sign stay allowed because such zeros cannot meet, and tests/wgsl_ops.rs requires `clamp(2.0, 0.0, 1.0)`. Tests: folded_angles_and_limits_are_refused_unless_nothing_can_differ, a_folded_zero_angle_is_the_rule_of_this_crate.
- **title:** `enable f16` with constants only was accepted, contrary to the README
  **severity:** low
  **proven:** true
  **fixed:** true
  **detail:** `enable f16; const K = f32(1.1h);` parsed, folded to 1.0996094 through the `half` crate, and left no 16-bit type for the validator to see. `half` takes the processor's instruction where one exists. I found no case where the bytes differ, but the README says nothing but 32 bits exists. It now gives `Error::Unsupported`. The block checker already refuses `enable`. Test: sixteen_bit_constants_are_refused_even_when_nothing_of_them_is_left.
- **title:** Proof that a parsed module can hold a folded value that differs, and the earlier explanation was wrong about which library
  **severity:** low
  **proven:** true
  **fixed:** true
  **detail:** Measured on one PC with the same shader and the same naga 30.0.1. With feature `wgsl-in` alone, 12 of 12 functions folded exactly as the `libm` crate on 3,999 inputs each, for example `sin(0.61925f)` = 0.5804246. Adding the `stderr` feature, which only concerns error printing, moved all 12 to this PC's standard library (0.58042467). On x86-64 `min(0.0, -0.0)` and `min(-0.0, 0.0)` also give different zeros. So refusal remains right. This workspace's build happens to reach `libm` today, not the operating system as the README and front.rs said, so both explanations were corrected.
- **title:** A fused blend had only one test able to notice, and that test could be compiled the same way as the sum it checks
  **severity:** low
  **proven:** true
  **fixed:** true
  **detail:** On a throwaway copy, a planted `mul_add` in the blend passed old_recipes, everything, rounding_order and wgsl_ops: 16-bit storage hides it. Only run::tests::over_is_the_formula_in_its_order failed, and it compares against the same sum written in 32 bits, which any contraction would change as well. Added over_is_not_fused_whatever_the_build, which works the sum out in 64 bits. A planted fused blend fails it. Separately, the ordinary x86-64 build has no fused multiply-add instruction, so no run before this one could have shown contraction. A build with target-cpu=znver2 gave every pinned hash unchanged, in dev and release.

## contract_changes

- No public API changed and no byte changed. crates/golden/data/render-portable was not regenerated: old_recipes and anchor pass against it as stored.
- Now refused with `Error::Shader` when folded by the parser: `radians` and `degrees`, except a lone literal zero such as `radians(0.0)`; `clamp` and `saturate`, except when every argument is a plain number with no minus sign. The earlier list (sin to fma, min, max) is unchanged.
- Now refused with `Error::Unsupported`: any text with `enable f16`; and a function, when reached, that reads a value named in a loop's body in its `continuing` after a `continue` can have gone past the name. A `var` there is still allowed, and so is a `let` that only names something already in place, such as an argument.
- `let` remains the way out: naming the argument keeps the call out of the parser's hands, and this crate's own rule works it out.
- New file crates/cpu-render/src/portability_tests.rs, compiled only for tests and wired in crates/cpu-render/src/lib.rs. Code changed in crates/cpu-render/src/front.rs and compile.rs. One test added in run.rs.
- crates/cpu-render/README.md: the section on constants is rewritten, and it gains 'Values a continue skipped' and 'Which processors the rules hold on'. crates/render-recipe needed no change: its data is little-endian, floats are stored as bits, and it has no arithmetic.

## checks

Everything was run from D:\chadfx under a hard time limit. Scripts and logs are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\review-platform.

**After the last code change**
| Check | Result |
|---|---|
| `cargo build --workspace --all-targets` | exit 0, no warnings |
| `cargo test -p cpu-render -p render-recipe` | 283 passed, 0 failed, 0 ignored |
| `cargo clippy -p cpu-render -p render-recipe --all-targets` | no findings in these crates |

**Before the last test-only additions** (one fused-blend test, the literal pins and two extra test cases). The renderer's code did not change after these runs.
| Check | Result |
|---|---|
| `cargo test -p cpu-render -p render-recipe -p golden` | all passed, 1 older doc example ignored |
| `cargo test -p render --test no_gpu_pictures --test no_gpu_recipes --test card_vs_cpu --test same_card` | 54 passed, on the RTX 3090 |
| Pinned-hash tests built with `target-cpu=znver2`, release | all passed |
| Same tests with cpu-render at opt-level 0, dev | all passed |
| WebAssembly check on the 351 old recipes | "Everything is the same", 23 libm hashes the same, 83 s |

The dev build with `target-cpu=znver2` ran earlier, before the last compile.rs tidy-up, and also passed. These pinned-hash tests are the library tests plus everything, threads, today, composed, libm_pins, old_recipes, wgsl_ops and rounding_order. The znver2 builds have real fused multiply-add instructions switched on.

**New tests that failed on the unfixed code:** 8 of the first 14 failed, and those 8 are the defect proofs. The other 6 passed before and after, and they guard rules that already held:
- the NaN rule (three sweeps)
- the remainder of constants
- the loops that must keep working
- a folded zero angle

Added later, also guards that held:
- the exact operations against software
- literals pinned to bits

**Planted bugs on one throwaway copy:** 47 of 47 caught, and the unchanged copy passed first. Three survived the first pass and showed real test gaps; I closed them and re-ran those three.
- A planted fused blend failed the new test.
- A planted fused dot product failed the pinned tests.
- A planted fused mix failed the pinned tests (wgsl_ops and everything).
- A planted fused interpolation failed a raster unit test.
- A planted fused filter changed nothing: its product is exact in 64 bits, so it is a control.

**Other targets:** `wsl -l -q` lists no distribution. The only installed targets are x86_64-pc-windows-msvc and wasm32v1-none, so `cargo check` for anything else cannot run. `rustc --print cfg` over all 331 targets shows the guard's condition holds only on i586-* and i686-unknown-uefi. A no_core compile confirmed it is active on i586-unknown-linux-gnu and not on i686, x86-64, ARM, RISC-V or WebAssembly.

**Checked and found sound, with no finding:**
- naga's mapping of built-in names is one to one.
- Every other folded function is exact.
- recipes.json holds no decimal floats.
- The frozen picture generators are integer-only.
- libm's machine-specific paths are x87-only, or have one right answer.

**Not run:** Mac, Linux, ARM, `cargo test --workspace`.

**Disk and data:** free space on C: is 339 GB. Stage 0 data, audio and fixtures are untouched. Git state was not touched. Scratch build folders are deleted.

## remaining_risks

- Mac, Linux and ARM have still never run this code. The promise there rests on the stated rules, on Rust's float guarantees and on the pinned hashes. The real check is `cargo test -p cpu-render` on such a machine. WebAssembly is the same physical processor.
- For Stage 3: crates/blocks (not mine) has no rule against the `continue` pattern, so a block can pass the block checker and be refused here. On a real card such a program's behaviour is unknown. I recommend the block checker refuse it too.
- For Stage 3: more programs that pass the block checker are now refused here, namely constant `radians` and `degrees` (except zero) and constant `clamp` and `saturate` (except non-negative plain literals), on top of the earlier list. The built-in blocks' own source passes these checks, as the composed tests show. Someone must decide whether the block checker refuses these as well.
- `max(1.0, 2.0)` is still refused although it is harmless, because tests/wgsl_ops.rs pins that refusal. The exception now used for `clamp` would fit if that test is ever relaxed.
- `%` between constants still goes through the C library's `fmod` on most targets. It is exact by definition and is compared with pure-Rust libm on about three million pairs, plus 200 through naga itself, on whichever machine runs the test. It is not refused.
- The compile thread has no float-settings check of its own. `Renderer::run` checks before compiling. `Shader::new` and `register_module` do not check first: on Linux a new thread inherits its creator's settings, so a thread flushing subnormals could fold a constant differently. This is unproven, and a test would need unsafe code that the crate forbids, in a file I do not own.
- Not fixed, not proven: on a 32-bit debug build only, the area sum in run.rs could overflow on a very large draw and give an error instead of bytes. 32-bit is not a product target.
- m68k is not refused. Its registers are the same kind as x87, and I did not look into whether Rust's compiler uses them. Adding it to the guard means changing a text pinned by tests/banned_calls.rs, which I do not own. It has no practical weight.
- That no stale read remains rests on my reading of naga's scoping rules plus one poison run over the test programs. Neither is a permanent test, and a program type not in the test set was not exercised.
- The block composer writes `radians(0.0)`, so that exception must stay unless compose.rs changes.
