# No-GPU picture proof: ng-card.md

## summary

Deliverables 4 to 7 are done, and `cargo test -p render` passes: 116 passed, 0 failed, 0 ignored, no compiler warnings. Nothing is committed and git state was not touched. Mac, Linux, ARM, a release build, the full workspace suite and any AMD, Intel or Apple graphics card were not run.

**4. tests/same_card.rs**
- On this computer's own card, it draws each frozen old recipe with the frozen old shaders through a new test-only path, tests/old_path/mod.rs, which shares no code with the renderer.
- It then draws the same scene with `Renderer` and compares byte for byte. It needs no stored pictures.
- All 351 scenes and the known panic match on the RTX 3090 (2.7 s) and on WARP.
- As a check on the old path itself:
  - It draws the stored WARP set on all 351 scenes.
  - It draws the stored RTX A4000 set on all 351 scenes when run on the 3090. That was one run on a throwaway copy.
- With no adapter it prints the reason and passes. I checked this by faking "no adapter" on the copy.

**5. tests/card_vs_cpu.rs**
- The card and the software renderer draw one recipe, and the test writes a row per scene.
- RTX 3090: 2,531,465 bytes differ (1.901%), 2,061 by 2 levels or more, 16 by 4 or more, largest 6.
- WARP: 482,588 bytes differ (0.362%), 2,972 by 2 levels or more, 18 by 4 or more, largest 193.
- Both are within the bound. These figures match what the old code measured earlier, byte for byte.

**A decision for the main session: I did not use the rule of anchor.rs as the failing bound.**
- That rule limits one-level differences scene by scene, and an honest card would fail it. On a scratch copy, rounding 16-bit targets to nearest or rounding 8-bit output at 0.4 moved whole flat scenes by one level (up to 75% of a scene), and never by two.
- So one-level differences are only limited over the whole run. Per scene the limits are on bytes off by 2 or more and by 4 or more. Each limit is a stated sum of measurements, and the anchor's rule is only reported, as a note.
- The sums assume a new make of card may be as far from a measured adapter as the two measured adapters are from each other.
- This bound failed all 8 wrong drawings I forced on the real card, and every scratch defect I tried beyond those it deliberately allows.

**6. Golden tests**
- golden_pictures.rs and golden_pictures_software.rs are unchanged and still run. On this PC the first prints that it compared nothing (the RTX 3090 has no set), and the second passes against WARP.

**7. Dependencies**
- cpu-render is only a dev-dependency.
- `cargo tree -p render -e normal` shows no cpu-render, and naga is brought in only by wgpu's own crates. A new test checks this.

**Planted bugs: 34 on a throwaway copy, each run once.**
- 33 behaved as predicted.
- The other showed that no test covered applying the per-scene limits. I added that test, and the bug now fails.

## contract

## New files
- `crates/render/tests/same_card.rs`, 11 tests.
- `crates/render/tests/old_path/mod.rs`, test-only.
- `crates/render/tests/card_vs_cpu.rs`, 12 tests.

Not changed by me: `crates/render/Cargo.toml` (the earlier engineer had already added `cpu-render` as a dev-dependency), `crates/render/src`, `crates/core-project`.

## tests/old_path (test helper, reachable only from tests via `mod old_path;`)
- `OldPath::open() -> Result<OldPath, String>`: chooses the adapter as stage-1-done did (high-performance, then fallback, `VEP_RENDER_ADAPTER=software` goes straight to fallback). Open it only after the renderer's device and only once, because opening devices at the same moment crashed before (`src/gpu.rs`).
- `OldPath::adapter_info(&self) -> String`: same format as `Renderer::adapter_info`.
- `OldPath::draw(&self, &render_recipe::Recipe) -> Result<(u32, u32, Vec<u8>), String>`: output RGBA8, tightly packed, top row first. Graphics-library errors come back as messages.

It carries out only what frozen recipes contain and refuses the rest with a message: `Step::Copy`, a pass with no clear, bindings other than uniform/texture/sampler at 0/1/2, samplers other than `LINEAR_CLAMP`, non-8-bit outputs. It must not change when Stage 3 changes the renderer.

## same_card.rs
Takes recipes from `golden::portable::Recipes::load()` and `recipe(scene)`, and today's frames from `Renderer::render_into` with `no_gpu::prepare`. The known panic is compared with what the old recipe of its twin draws.

- Failing pictures go to `<target>/tmp/same-card-failures/<what>.today.png` and `.old.png`.
- It prints `adapter: <name>` and `same card: on <name>, 351 scenes and 1 known panic(s) ...`.
- With no adapter it prints `SKIPPED: ...` and passes.

## card_vs_cpu.rs
One recipe from `render::recipe_at` goes to both `Renderer::render_recipe_into` and `cpu_render::Renderer::run`. Stage 3 note: it works unchanged as long as `recipe_at` gives drawable recipes. Composed block modules should call `register_module` on the software renderer in `software()` (a program within its budget gives the same bytes either way).

Outputs:
- A row per frame on stdout (seen with `--nocapture` or on failure).
- `<target>/tmp/card-vs-cpu/<adapter-key>.tsv`, with columns `scene bytes differ by1 by2 by3 by4up largest "nv-wa bytes" "nv-wa largest" farther`. `farther` is `yes` when the frame breaks the rule of anchor.rs; this is a note, not a failure. It reads `crates/golden/data/render-portable/anchor-table.tsv`.
- When the bound is exceeded: `<target>/tmp/card-vs-cpu/<scene>.card.png` and `.software.png`.

## The bound (constants at the top of card_vs_cpu.rs; `the_bound_is_the_sum_of_what_was_measured` recomputes them from the files)
| limit | value |
|---|---|
| one scene, bytes off by 2 levels or more | `max(total * 5904 / 921600, 16)` (0.641%) |
| one scene, bytes off by 4 levels or more | 31 |
| whole run, bytes off by 2 levels or more | 9,630 |
| whole run, bytes off by 4 levels or more | 54 |
| whole run, bytes that differ at all | 14,997,273 of 133,000,804 (11.276%) |

The module's comment says where each number comes from.

## Commands
- `cargo test -p render --test same_card -- --nocapture`
- `cargo test -p render --test card_vs_cpu -- --nocapture`
- Add the environment variable `VEP_RENDER_ADAPTER=software` to run either on WARP.

## For the main session
crates/golden/README.md, README-STAGES.md and docs do not mention these tests yet; I do not own them.

## deviations

- The failing bound is not literally the per-scene rule of crates/cpu-render/tests/anchor.rs. That rule limits bytes differing by one level per scene, and an honest card would fail it. Rounding rules within the specifications moved whole flat scenes by one level where NVIDIA and WARP happen to agree (up to 75% of a scene). The bound limits bytes off by 2 or more and by 4 or more, per scene and over the run, and limits one-level differences only over the whole run. Anchor's rule is still reported per frame in a `farther` column and a count, but does not fail.
- The whole-run limit on bytes that differ (11.276%) comes partly from a scratch measurement, not only from the anchor: 12,254,522 bytes from the worst combination of rounding rules tried, plus the adapters' own 2,742,751. The scratch script and results are kept in the scratchpad, not in the repository.
- The limits are sums (software to a measured adapter, plus adapter to adapter), which assumes a new make of card may be as far from a measured adapter as the two measured adapters are from each other. That assumption is mine. The sums use the larger software distance only where the module's comment says so.
- Eight scenes have no stored pictures in Stage 0, so their part of the bound is three constants in card_vs_cpu.rs, measured once on this PC. The files can only check the other 343 scenes.
- Beyond the brief: tests/old_path/mod.rs is a separate helper file, with a test that it uses nothing from the render library.
- Beyond the brief: a test in same_card.rs that where the adapter has a Stage 0 set, the old path draws it. It compares nothing on the RTX 3090.
- Beyond the brief: a_card_that_misdraws_fails feeds real wrong programs and instructions to the card, 8 runs over all scenes. That adds about 20 s on the RTX 3090 and about 70 s on WARP.
- Beyond the brief: a test walking Cargo.lock for what deliverable 7 asks. Its list of what brings naga in is wgpu, wgpu-core, wgpu-hal and wgpu-naga-bridge, because the lock file lists every optional dependency (wgpu names naga as an optional one). `cargo tree -p render -e normal -i naga --depth 1` shows only the last three.
- libm v0.2.16 does appear in `cargo tree -p render -e normal`. It comes through naga and num-traits via wgpu, not from cpu-render.
- The WARP runs were by hand with the environment variable. No separate test program forces WARP for these two files, so `cargo test -p render` covers only the machine's default adapter.

## measurements

- same_card, dev profile: all 351 scenes and the known panic byte-identical, 617 layers, 444 source pictures, 133,148,260 bytes. RTX 3090 on Vulkan: whole file 2.7 s, 11 tests. WARP: whole file about 25 s, with `any_change_to_what_the_old_recipe_draws_fails` re-run after its fix; only a type alias changed after that.
- Old path against stored sets: WARP dev set on all 351 scenes on WARP (a test). RTX A4000 dev set on all 351 on the RTX 3090 (once, on a throwaway copy, with the key forced).
- card_vs_cpu on the RTX 3090 (Vulkan), driver 32.0.16.1088, 352 frames: 2,531,465 bytes differ (1.901%), 2,061 by 2 or more, 16 by 4 or more, largest 6, 59 frames identical, 0 frames noted under the rule of anchor.rs. Whole file 22.9 s.
- card_vs_cpu on WARP (Dx12), 352 frames: 482,588 bytes differ (0.362%), 2,972 by 2 or more, 18 by 4 or more, largest 193, 260 frames identical, 1 frame noted. Whole file 88 s. Both byte counts match the capture engineer's earlier old-code drawings exactly.
- Measured basis, all 351 scenes, 133,000,804 bytes (differ / by 2+ / by 4+):
software and NVIDIA: 2,531,465 / 2,061 / 16
software and WARP: 482,588 / 2,972 / 18
NVIDIA and WARP: 2,742,751 / 6,658 / 36
The worst scene by 2+ is hd.noise_scaled_rotated_preview_640 (921,600 bytes): 2,709 for software and WARP, 3,195 for NVIDIA and WARP; 124 for software and NVIDIA.
Worst scene by 4+: 16 for software and NVIDIA (wide_noise.crop_transform_color_mask), 6 for software and WARP, 15 for NVIDIA and WARP.
The 8 hash-only scenes' share of those totals: 1,791,899/1,569/0, 332,275/199/11, 1,930,659/2,989/12.
- Other rounding rules tried on the software renderer, against itself, scratch copy (bytes differ, worst scene, by 2+):
16-bit stores to nearest: 6,189,954 (4.654%), 50%, 0
8-bit up at 0.4 of a level: 10,802,514 (8.122%), 75%, 0
8-bit up at 0.45: 6.129%, 75%, 0
8-bit up at 0.6: 5,036,781 (3.787%), 25%, 0
filter rounding point 0.4: 495,067 (0.372%), 4.5%, 649
filter rounding point 0.6: 497,936 (0.374%), 4.7%, 602
nearest plus 8-bit at 0.4: 8.984%, 75%, 0
nearest plus 8-bit at 0.6: 2.680%, 25%, 0
nearest plus 0.4 plus filter 0.4: 12,254,522 (9.214%), 75%, 841
- Defects on the same scratch copy, all beyond the bound (differ; by 2+; by 4+; worst scene by 2+ of 921,600):
pow a thousandth high: 25,876,765; 1,159; 640; 7,062
blend's keep a thousandth low: 1,712,444; 3,851; 800; 13,806
top-left rule reversed: 367; 366; 360; 2,288
pixel centre moved 1/32: 46,688,394; 39,944,958; 30,132,685; 656,800
filter coordinate dropping bits: 18,229,873; 3,902,638; 1,162,736; 139,022
snapping dropping bits: 2,790,376; 162,699; 1,415; 121,502
filter at 16 steps: 7,524,914; 4,248,746; 1,093,590; 307,954
snap at 16 steps: 6,556,005; 3,248,525; 860,378; 412,713
- Wrong drawings forced on the real card, RTX 3090, problems reported by the bound. Tested with the assertion:
last display step 2 parts in 1,000 too bright: 1 (14.98% differ)
every pow 1 part in 1,000 high: 5
red and blue swapped: 685
nearest filter: 148
not premultiplied: 614
not blended: 468
every layer one pixel right: 678
top layer lost: 662
Measured but not in the final test:
one level brighter: 34
3 parts in 1,000: 2
5 parts in 1,000: 123
half a level brighter: 2
Not noticed: 1 part in 1,000 too bright at the last display step, which makes 9.22% of bytes differ and 2,728 by 2 or more.
- Planted bugs, once each, on a throwaway copy with its own build folder, with an unchanged copy passing before each batch, 34 in all.
- Recipe, shader, plan and migration bugs (8) all failed the same_card proof. The 6 predicted not to fail the card_vs_cpu proof did not, because both renderers draw the same wrong recipe. These belong to same_card and the no_gpu proofs.
- Card-path bugs (9) all failed same_card. 8 were predicted to fail card_vs_cpu too and did; the ninth, pictures repeating at the edge, also failed it.
- Old path bugs (2), test bugs in same_card (3), wrong bound sums (3) and rule bugs (4) failed the predicted tests, with one surprise below.
- Software bugs: top-left reversed and pow off both failed card_vs_cpu's proof. Software rounding targets to nearest passed, by design.
- Fake no adapter: both files passed and printed the reason.
- The surprise: with `judge` not applying the per-scene limits, nothing failed. I added a test for that; on a re-run the bug fails `the_limit_of_a_scene_is_exact`, though the script's stale prediction still labels it a surprise.
Results: C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\proofs\plant\results.txt
- `cargo test -p render`, whole package: 116 passed, 0 failed, 0 ignored, 73 s: library 24, bad_requests 1, card_vs_cpu 12, first_use 1, golden_pictures 1, golden_pictures_software 1, many_renderers 4, no_gpu_pictures 10, no_gpu_recipes 13, recipe_on_card 14, refused 1, render 15, reuse 4, same_card 11, threads 4.
- Free space on C: 343 GB after deleting the scratch copies and their build folders.

## known_gaps

- Mac, Linux and ARM have not been run or compiled for. Both tests contain no platform-specific code. Whether they pass there is untested.
- No AMD, Intel or Apple graphics card has been compared. The bound rests on two adapter families, NVIDIA and WARP, plus rule variations I simulated in software. A real new card could exceed it for honest reasons. The test then fails with a message saying to look at the two pictures before believing either explanation.
- A card with coarser snapping or filtering than 8 fractional bits, which Vulkan permits, or one whose vertices snap downward, would fail. At 4 bits, the worst scene has 45% (snapping) or 33% (filtering) of its bytes off by 2 or more. Snapping downward gives 13%. Nobody has measured whether a real card does this; Apple's graphics is the one I would check first.
- What card_vs_cpu deliberately does not notice: a lone wrong pixel, or a few (WARP really has them at the corners of turned layers, 193 levels off); one level on one scene; a last display step 1 part in 1,000 too bright, which is about what rounding at 0.4 does. It would also pass a card that rounds targets to nearest, by design.
- card_vs_cpu by itself cannot see a wrong recipe, because the card and the software renderer draw the same one. same_card and the no_gpu proofs cover that.
- The 8 hash-only scenes' part of the bound is three constants measured once on this PC. The other 343 scenes are recomputed from files in a test, and the scratch rule-variant numbers are recorded only in the test's comment. Their script and results are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\proofs\bound (variants.py, results). The source copies and pictures were deleted, so re-running means rebuilding.
- Not investigated: two planted recipe changes, layers replacing instead of blending and an 8-bit composite, also failed the card_vs_cpu bound. So on those paths the card and the software renderer are farther apart than the bound. Today's recipes never ask for them. Stage 3 should be careful if it ever does, because the anchor measured nothing there.
- A machine with truly no graphics stack was not tested. The no-adapter path was checked by faking the error on a throwaway copy, and the test programs still link wgpu. `graphics_device_opened` plays no part here, since these tests are meant to open a device.
- WARP was not re-run after the last edits: `each_pixel` in card_vs_cpu.rs, type aliases in both files, and the added test lines in the_limit_of_a_scene_is_exact. WARP coverage is only by hand with the environment variable. Those edits touch only test code.
- golden_pictures.rs on this PC still prints that it compared nothing, so its pass proves nothing here. same_card and card_vs_cpu are what give this PC's card real comparisons.
- The direct old-path check against the RTX A4000 set on the 3090 was a one-off on a throwaway copy, not a repeatable test. It holds transitively through same_card plus the earlier engineer's scratch check.
- Whole workspace test suite not run, and no release build of these tests. Crates that depend on render were built, not tested.
- Older clippy notes remain in crates/render/src/image.rs:39 and tests/common/mod.rs:70, not mine, and in other crates. rustfmt would reflow the new tests' long lines; the crate's other test files have long lines too, so I left them.

## checks

All run from D:\chadfx with hard time limits. Logs are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\proofs.

**After the last code change**
| Command | Result |
|---|---|
| `cargo build --workspace --all-targets -j 16` (after touching render's lib) | exit 0, 0 warnings |
| `cargo test -p render -j 16 --no-fail-fast` | exit 0, 116 passed, 0 failed, 0 ignored, 73 s, no warnings |
| `cargo test -p render --test card_vs_cpu --test same_card every_ -- --nocapture` | exit 0; prints the adapter name and the summary lines quoted in measurements |
| `cargo clippy --offline -p render --all-targets` | no findings in same_card.rs, card_vs_cpu.rs or old_path; older notes at image.rs:39 and tests/common/mod.rs:70 remain |

**Each run once, before the last small test-only edits**
| Command | Result |
|---|---|
| `VEP_RENDER_ADAPTER=software cargo test -p render --test card_vs_cpu -- --nocapture` | exit 0, 11 tests on WARP (the lock-file test and the added assertion came later, and need no adapter) |
| `VEP_RENDER_ADAPTER=software cargo test -p render --test same_card -- --nocapture --test-threads=1` | 10 passed on WARP, 1 failed because of my own test's choice of change; after the fix, `any_change...` passes on WARP alone |

**Dependency and data checks**
| Command | Result |
|---|---|
| `cargo tree --offline -p render -e normal` | no cpu-render; `-i naga --depth 1` shows wgpu-core, wgpu-hal, wgpu-naga-bridge; `-i cpu-render` prints nothing to show |
| `cargo tree --offline -p vep-app -e normal` | no cpu-render (0 lines) |
| `git status --short` on tests/golden_pictures.rs, golden_pictures_software.rs, golden_check, crates/golden/data/render, data/audio | no change |

**Throwaway copy, now deleted**
- 34 planted bugs, in three batches plus one re-run of the bug whose test gap I closed.
- One forced comparison of the old path with the A4000 set.
- The real tree never held a planted bug. `touch` changed only one file's time.

**Not run:** Mac, Linux, ARM, AMD/Intel/Apple cards, `cargo test --workspace`, `cargo test --release`.

**Files:** only crates/render/tests/same_card.rs, crates/render/tests/card_vs_cpu.rs and crates/render/tests/old_path/mod.rs were created. Git state was not touched.
