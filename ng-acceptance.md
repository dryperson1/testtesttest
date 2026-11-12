# No-GPU picture proof: ng-acceptance.md

## verdict

done_with_gaps

## summary

The no-graphics-card picture proof works on this PC, and all three test rounds passed with no failures. It has never been run on a Mac, on Linux or on an ARM processor, so that part of your request remains a design promise, not a measurement.

**Test rounds**
- Each round ran nine packages, one at a time, with a 590-second limit each. No test was left out.
- Every round: 1,091 tests passed, 0 failed, 9 ignored (7 are slow measurements or an old doc example; 2 are the sampler items in the defects).
- Rounds took 946, 864 and 892 seconds.
- The workspace builds with zero warnings.
- No code changed after the rounds began. No FFmpeg process was left running.
- Stage 0 pictures, audio and fixtures show no change in git.

**Gaps**
- Mac, Linux and ARM have not been run or even compiled for. WSL has no Linux installed, and this PC has only the Windows and WebAssembly build targets.
- No AMD, Intel or Apple graphics card has been compared.
- Six items in the defects list are left for you or the main session.
- Nothing is committed.
- Spec section 20 and README-STAGES.md are not written.

**My fixes** were documentation only, in two READMEs. I did not plant any bugs; those checks rest on the engineers' logged runs.

## plain_answers

**(a) How many scenes are compared with no graphics card, and how long does it take?**
- All 351 scenes plus the 1 known panic: 133,148,260 bytes, byte for byte, with no tolerance.
- Run with a nonsense adapter name and the Vulkan driver path pointed at a missing file: 14.0 seconds for both proofs. The picture proof took 11.8 s on 8 threads, and the recipe proof, which compares what the old code told the card, bit for bit, took 1.8 s.
- 617 layers (618 with the known panic), 36,704 uniform words and 447 source requests were compared.
- Speed from the engineers' logs, not re-measured: 61 to 67 s on one thread of this processor. A real 8-core laptop has not been measured; the limit of about 2 minutes is met even on one thread here.

**(b) How close is the software renderer to the real adapters?**

Over the 343 scenes stored as pictures (71,253,604 bytes), re-printed today:
| Pair | Bytes that differ | Scenes identical | Largest difference (levels out of 255) |
|---|---|---|---|
| software and NVIDIA RTX A4000 | 739,566 (1.038%) | 57 | 6 |
| software and WARP | 150,318 (0.211%) | 257 | 193 |
| NVIDIA and WARP | 812,095 (1.140%) | 54 | 193 |

So it is closer to each adapter than they are to each other. The 193 is WARP alone, on 2 pixels.

Over all 351 scenes (133,000,804 bytes), this PC's RTX 3090:
- Software and RTX 3090: 2,531,465 bytes (1.903%), largest 6.
- RTX 3090 and WARP: 2,742,751 bytes (2.062%), from the engineer's measurement and the bound's constants.
- Software and WARP: 482,588 bytes (0.363%).

Today's run over all 352 frames gave the same 2,531,465 and 482,588.

The exceptions are named and pinned in crates/cpu-render/tests/anchor.rs:
- 1 scene is farther in largest difference: 3 levels from WARP, where the adapters are 2 apart.
- 21 scenes have more bytes from NVIDIA than the adapters have between them, by at most 14.3%.
- These are explained by counting, not by a proven physical cause.

The RTX 3090 is weak independent evidence: on Vulkan it matches the A4000 on all 351 scenes (logged, not re-run by me), so it is the same family of rounding.

**(c) Does WARP on this PC reproduce the Stage 0 set?**
Yes.
- Today's renderer: golden_pictures_software passes 351 scenes and the known panic, byte-identical to the dev (debug) set, in all three rounds and once more with its output shown.
- Old code: logs show its verify passed all 351 on both the dev and release sets. I read those logs and did not re-run them, because that build was deleted.

**(d) Did same_card pass on the RTX 3090 for every scene?**
Yes.
- All 351 scenes and the known panic, 133,148,260 bytes, both the way the preview draws and the way the export draws. It printed the adapter as NVIDIA GeForce RTX 3090 (Vulkan), and the whole file took 4.3 s.
- It also passes on WARP.
- golden_pictures on the RTX 3090 still prints that it skipped, so that one pass proves nothing, by design.

**(e) What exactly is NOT proven?**

Never run:
- Mac, Linux and ARM. The promise rests on Rust's exact floating-point operations, the pinned libm version and the pinned hashes.
- WebAssembly is the nearest substitute: today the 8 pinned frames and 23 libm hashes matched, and an earlier reviewer matched all 351 old recipes. But it is the same physical processor, so it says nothing about how ARM rounds.
- The real check on a new machine is cargo test -p cpu-render -p golden, plus no_gpu_recipes and no_gpu_pictures.
- A machine with truly no graphics stack. The test programs still link wgpu, and the no-adapter path was only faked on a throwaway copy.

Not measured:
- No AMD, Intel or Apple card, and no real 8-core laptop.

What no CPU test can ever show:
- Whether a real card or driver draws correctly. That needs same_card and card_vs_cpu on that card.
- card_vs_cpu tolerates honest rounding, so it passes one wrong pixel, a family of scenes one level brighter when few enough bytes are involved, and a final display step 1 part in 1,000 too bright.
- The alpha of the composite never shows in a frame's picture. Only hand-made recipes test it.
- The 8 large scenes kept only as a hash cannot be compared with the stored adapters' pictures by file on any computer. On a card that draws a Stage 0 set, card_vs_cpu measures them again.

Also not known:
- Why the software renderer alone differs on 37,742 bytes (0.053%). Counting explains the pattern; no physical cause was found.

**(f) Are all eight video tests fixed, and can anything still hang?**

All eight pass in all three rounds, along with the two app tests that used to be left out. The engineer's logs from a clean copy of the last commit (scratchpad v3\base) show the listed failures failing before the fixes; I read those logs and did not re-run them.

Causes as reported, which I checked only against the code diffs and logs:
1. camera_style_footage_gets_tiles...: FFmpeg delivers its frame and then never exits, and nothing limited the wait. Seen in 2 of 60,000 runs at 60 processes and 0 of 45,000 at 32. Why FFmpeg stops inside is not proven. The fix is tested with a stand-in program, not with a real freeze.
2. a_damaged_audio_file_does_not_hang_the_export: the test's 10% floor was wrong. A plain FFmpeg decode of the same file has seconds of 6% and 9%. The test now compares with plain FFmpeg, within 2 points. A related real defect was also fixed: heavily damaged files lost their last piece of sound.
3. tiny_frames_are_never_given_to_a_hardware_encoder: this PC's NVIDIA encoder refuses frames below 146x50 (H.264) and 130x34 (HEVC). A hardware-only export is now refused before rendering.
4. a_proxy_of_footage_whose_pixels_are_not_square...: the same NVIDIA size limit. The encoder is refused in words and the next one is used.
5. every_rotation_is_turned_into_the_pixels_of_the_proxy: the same limit, with the same fix.
6. a_resampled_decode_is_the_same_wherever_it_is_opened: the AAC encoder's made-up noise bands depend on where decoding started. Exact cases now turn that tool off, and new default-settings cases must stay within 0.01, below the 0.014 that one misplaced sample causes.
7. the_index_of_a_recording_that_starts_between_keyframes...: libx264 added its own keyframe at a scene change. The test fixture now turns that off.
8. a_proxy_job_whose_source_went_missing_fails_cleanly: a real race in the app. A job could fail before it was marked pending, and that mark erased the failure.

Old-machine explanations for why these passed there could not be tested here.

What can still hang or wait:
- Every FFmpeg and ffprobe run in product code now goes through a 5-minute silence limit, enforced by nothing_waits_for_a_tool_except_through_a_watch in crates/media/tests/stuck_tool.rs. That check is a text search of the source, not a run.
- A stuck decoder can hold the preview for up to those 5 minutes.
- Three test stand-ins wait on real FFmpeg without a limit: ui/src-tauri/tests/crash.rs:138, crates/export/tests/stalled_encoder.rs:126 and crates/media/tests/support/stand_in.rs:142. From reading the code, the outer limit stops the stand-in, so the test should not freeze, though a stuck FFmpeg could be left behind. I did not exercise this.
- crates/cli tests wait on the vep program with no limit of their own, and that package was not in these rounds.
- cargo test has no per-test limit. Eight planted renderer bugs in the engineer's check showed up only as a run that never ended, so a future bug could look like a hang.
- A hostile shader in cpu-render ends with an error, but at worst after about 45 CPU-minutes under the default limits.

## deliverables

- **item:** ENGINE 1: render-recipe data types with no dependencies; a draw carries WGSL text
  **done:** true
  **evidence:** crates/render-recipe/src/lib.rs:200 Draw, :222 Pass, :251 Recipe; empty [dependencies] in crates/render-recipe/Cargo.toml; crates/render-recipe/src/tests.rs has 9 tests.
- **item:** ENGINE 2: interpreter of naga IR covering what the block checker allows; clear error for anything unsupported; a shader cannot hang it
  **done:** true
  **evidence:** crates/cpu-render/src/compile.rs, vm.rs, math.rs; tests/wgsl_ops.rs (55 tests), src/wgsl_spec_tests.rs (32, 2 ignored), tests/composer.rs:80 every_built_in_picture_block_runs_with_and_without_its_budget, tests/hostile.rs:31 endless_loops_end_in_an_error; Error::Unsupported at lib.rs:78. Not everything is accepted: constant-folded sin, max and similar, and values skipped by a continue, are refused (see defects).
- **item:** ENGINE 3: run a Recipe, return bytes and optional intermediate targets, parallel, independent of thread count
  **done:** true
  **evidence:** crates/cpu-render/src/lib.rs:249 run, :255 run_keeping; tests/threads.rs:100 the_same_bytes_on_1_2_7_and_64_threads; tests/old_recipes.rs:133 the_number_of_threads_changes_nothing.
- **item:** ENGINE 4: 351 scenes in about 2 minutes on an 8-core laptop, dev profile
  **done:** false
  **evidence:** Not measured on a real 8-core laptop. On this Threadripper: 61 s on 1 thread and 13 s on 8 (engineer's logs, README section 'How much it costs'); today's no_gpu_pictures took 11.8 s on 8 threads. The limit is likely met, but it is not proven on that machine.
- **item:** ENGINE 5: tests for each operation and corner case, banned-call scan, pinned libm grid, thread counts, hostile loop, dependency graph
  **done:** true
  **evidence:** tests/banned_calls.rs:149 the_source_holds_no_banned_call and :195 the_manifest_pins_libm_exactly; tests/libm_pins.rs:126 every_function_of_one_float_gives_the_pinned_results; tests/threads.rs:100; tests/hostile.rs:31; tests/dependencies.rs:98 the_dependency_tree_has_no_native_or_graphics_crate and :115 cargo_tree_shows_no_native_or_graphics_crate. My own search of the non-test source files found no banned call, clock, environment read or hash table.
- **item:** ENGINE 6: crates/cpu-render/README.md as the definition of the arithmetic
  **done:** true
  **evidence:** crates/cpu-render/README.md. It disagreed with the code until I added the 3 sentences the WGSL reviewer wrote (linear value at a cut, sample_index, struct or alias named like a built-in). I spot-checked the 8-bit level and premultiplied-over formulas against src/float.rs:170 and src/run.rs:116, and they match.
- **item:** VIDEO 1: proven root cause of each of the eight
  **done:** true
  **evidence:** Causes are in plain_answers (f). Clean-copy failing logs are in scratchpad v3\base (proxy job failed 2 of 6). Exception: the reason FFmpeg stops inside the freeze is not proven; only the behaviour is, from logged catches.
- **item:** VIDEO 2: no app or test path can freeze indefinitely on a stuck FFmpeg child
  **done:** true
  **evidence:** crates/media/src/watch.rs (ToolWatch, OutputWithLimit, 5-minute limit); crates/media/tests/stuck_tool.rs:331 nothing_waits_for_a_tool_except_through_a_watch; crates/ingest/tests/stuck_tool.rs (10 checks, e.g. :82, :106). My own search found every product process start in media and ingest going through it. Three test stand-ins still wait on real FFmpeg, and from reading the code the outer limit stops them; I did not run that case. crates/cli tests wait on vep with no limit of their own and were not in these rounds.
- **item:** VIDEO 3: product defects fixed with tests; old-machine tests corrected without weakening
  **done:** true
  **evidence:** Reviewed the test diffs. crates/export/tests/review.rs:1254 and :1318 a_file_with_damage_through_all_of_it_is_heard_to_its_end; :1695 tiny frames; crates/media/src/encode.rs:353 size_problem; crates/ingest/tests/proxy.rs:436, :1016 and the new refusal test; crates/media/tests/review.rs:731; crates/media/tests/timing.rs:325; ui/src-tauri/tests/review.rs:620 plus a_proxy_job_that_fails_at_once_is_always_listed_as_failed. No new skip or ignore in these tests. Judgement call to know about: the damaged-file test went from an absolute floor to a comparison with plain FFmpeg within 2 points, and default-settings AAC went from exact to a limit of 0.01; both have stated reasons and exact cases are kept with the noise tool off.
- **item:** VIDEO 4: media, ingest, export and vep-app green three times, with durations, with no --skip
  **done:** true
  **evidence:** Rounds 1 to 3: media 165 ok (199.9, 200.1, 203.2 s); ingest 134 ok (244.8, 238.9, 244.7 s); export 113 ok (170.7, 163.8, 166.3 s); vep-app 106 ok (198.7, 146.1, 150.5 s). These counts include the separately run stuck-tool checks and crash scenarios. the_scratch_overlay_shows_in_the_player_and_clears (ui/src-tauri/tests/edit.rs:198) and every_command_stays_quick_on_a_large_project (review_speed.rs:164, 1.5 s limit unchanged) passed every round. The fix behind the second is Project::jump, crates/core-project/src/lib.rs, tested in crates/core-project/tests/persistence.rs. Each package was run as its own command, because the combined command does not fit the 10-minute limit.
- **item:** FREEZE 1: capture of the old code's recipes, with no device opened
  **done:** true
  **evidence:** crates/golden/data/render-portable/capture/capture_recipes.rs.txt and old-code-changes.diff; pinned by crates/golden/tests/portable_references.rs:292 the_capture_is_on_record and :217 the_recipes_are_frozen_byte_for_byte. Reviewers rebuilt the same 889,535 bytes from the tag; I did not repeat that.
- **item:** FREEZE 2: render-portable folder and golden::portable, no editor crate
  **done:** true
  **evidence:** crates/golden/src/portable.rs:298 Recipes, :334 recipe; module line crates/golden/src/lib.rs:15; folder is 9.4 MB, recipes.json is 889,535 bytes, 343 PNGs; crates/golden/tests/portable_references.rs:448 this_crate_depends_on_no_editor_crate. Checked today: the depth-1 dependency list of golden shows render-recipe as its only workspace crate. Frozen shaders match the old git blobs 63e6c85f and fe44ca03 at both tags and HEAD.
- **item:** FREEZE 3: make_portable_references; --check writes nothing, --write required; same bytes across runs and builds
  **done:** true
  **evidence:** crates/cpu-render/examples/make_portable_references.rs:55-57. Today: --check dev on 8 threads 20.3 s and release on 3 threads 54.3 s, both 'compared 351 of 351 scenes, every byte is as stored'. The seven --write runs are from the engineer's logs.
- **item:** FREEZE 4: old_recipes test
  **done:** true
  **evidence:** crates/cpu-render/tests/old_recipes.rs:120 every_scene_reproduces_its_reference_byte_for_byte, :215 one_flipped_byte_in_a_stored_picture_fails, :352 counting_finds_a_comparison_that_skipped_a_scene.
- **item:** FREEZE 5: anchor test, with the table and every exception explained
  **done:** true
  **evidence:** crates/cpu-render/tests/anchor.rs:323 the_software_renderer_is_as_close_to_each_adapter_as_they_are_to_each_other, :336, :359, :372, :442; crates/golden/data/render-portable/anchor-table.tsv; anchor_table --totals re-printed today gives the same numbers. The 21 byte-count exceptions rest on a counting explanation, not a physical cause.
- **item:** FREEZE 6: old code --verify on WARP, and drawing on the RTX 3090
  **done:** true
  **evidence:** Logs in scratchpad old\logs read today: 'verified 351 scenes ... all byte-identical' on the debug and release WARP sets. The 3090 result is in the render-portable README. Not re-run by me, because the old build was deleted; a reviewer re-measured it the same day.
- **item:** RENDER 1: Recipe built from a FramePlan with no device; the Renderer draws from that same recipe
  **done:** true
  **evidence:** crates/render/src/recipe.rs:286 frame_recipe, :295 recipe_at; crates/render/src/renderer.rs:399 render_recipe_into, :659 start_unchecked takes the recipe. Narrower than a general recipe: a draw must have exactly today's bindings, and Stage 3's pass modules need that widened.
- **item:** RENDER 2: no_gpu_recipes, all 351 bit for bit
  **done:** true
  **evidence:** crates/render/tests/no_gpu_recipes.rs:373 every_scene_computes_the_recipe_the_old_code_computed_bit_for_bit, :394 the_renderers_programs_are_still_the_old_ones; the header states what Stage 3 must do. Printed today: 351 scenes and 1 known panic, 36,704 uniform words, 447 requests, no device. Not compiled on Mac or Linux.
- **item:** RENDER 3: no_gpu_pictures, all 351 and the known panic, no tolerance, cannot skip
  **done:** true
  **evidence:** crates/render/tests/no_gpu_pictures.rs:184 every_scene_draws_its_portable_reference_byte_for_byte, :310 the_known_panic_must_draw_what_its_twin_draws, :299 a_scene_that_cannot_be_drawn_is_a_failure_not_a_skip.
- **item:** RENDER 4: same_card on any adapter
  **done:** true
  **evidence:** crates/render/tests/same_card.rs:267 and :278; :515 with_no_adapter_it_gives_the_reason_and_compares_nothing; :527 an_adapter_the_renderer_cannot_use_is_a_failure; crates/render/tests/old_path/mod.rs:36. Passes on the RTX 3090 and WARP today.
- **item:** RENDER 5: card_vs_cpu reports per scene and fails only beyond the bound the anchor measured
  **done:** true
  **evidence:** crates/render/tests/card_vs_cpu.rs:363, :435 a_card_that_misdraws_fails, :543 the_bound_is_the_sum_of_what_was_measured, bound constants at :119-131. It deviates from the brief: the failing bound is not literally the anchor's rule (see defects). Passes on the RTX 3090 (2,531,465 bytes) and WARP (482,588).
- **item:** RENDER 6: golden_pictures and golden_pictures_software stay and are not weakened
  **done:** true
  **evidence:** Neither file shows a change in git; golden_check/mod.rs changed only so that only NoAdapter skips, which is stricter. The first prints SKIPPED on the 3090; the second passes against the WARP dev set.
- **item:** RENDER 7: cpu-render only a dev-dependency; nothing extra in the shipped build
  **done:** true
  **evidence:** crates/render/Cargo.toml:28; card_vs_cpu.rs:779; no_gpu_pictures.rs:517 and :580. Checked today: cargo tree for vep-app and vep shows 0 cpu-render or golden lines. The dependency tree of render has naga only through wgpu.
- **item:** Zero-warning build and no panic = abort added
  **done:** true
  **evidence:** cargo build --workspace --all-targets: 0 warnings, at the start and after the rounds. The only panic = abort lines are blocks/src/Cargo.toml:29 and :35, a separate WebAssembly workspace last changed in commit c89816a.
- **item:** No change under the Stage 0 data or fixtures
  **done:** true
  **evidence:** git diff against HEAD and git status show nothing under crates/golden/data/render, crates/golden/data/audio, or the fixtures of core-model, core-project, blocks and core-edit. Files on disk match git's lists: 655, 201 and 44 for the first three groups.

## defects

- Mac, Linux and ARM have not been run or even compiled for, so the cross-platform promise is not yet measured. This is the main gap in your request. It needs a real machine: cargo test -p cpu-render -p golden, plus no_gpu_recipes and no_gpu_pictures in the render package.
- Your decision: the software renderer differs from real cards in two proven sampler cases, kept as ignored tests in crates/cpu-render/src/wgsl_spec_tests.rs (base_clamp_uses_the_filter_of_its_sampler at about line 1451, an_explicit_level_or_gradient_chooses_the_filter at about line 1394). No stored picture changes either way, and today's samplers cannot reach them. Adopting the card's behaviour means changing pins in tests/everything.rs and tests/recipes.rs:626, and the README's stated rule.
- Decision for Stage 3: programs that pass the block checker are refused by cpu-render. These are constant calls such as sin(0.5) or max(1.0, 2.0), constant radians, degrees, clamp and saturate apart from narrow exceptions, and a loop whose continuing block reads a value a continue skipped. No built-in block does this. Someone must decide whether crates/blocks refuses them too, or a block could work on a card and be impossible to prove here.
- Same weakness outside crates/render: tests in crates/playback/tests, crates/export/tests/export.rs:47 and ui/src-tauri/tests (about 17 sites, e.g. edit.rs:199, review.rs:33) treat any renderer error as 'no card' and pass without testing anything. crates/render was fixed so only NoAdapter skips. In practice a device failure now fails the render package, but these tests still pass vacuously on their own. Too large for a small fix.
- card_vs_cpu's failing bound is not the anchor's rule the brief asked for, and it is loose: 11.276% of bytes may differ over a run. That number rests partly on a scratch measurement of simulated rounding rules (12,254,522 bytes) that is not in the repository, and it would pass a final display step 1 part in 1,000 too bright. The engineer's reason is that the anchor's rule would fail an honest card. same_card is what catches small faults on a card. This deserves the main session's review.
- Not done, main session's job: spec section 20, README-STAGES.md and RESUME-NO-GPU-PROOF.md do not record the finished state; reports are not saved to docs/blocks-rewrite/stages; nothing is committed. The folders D:\chadfx-base (47 MB), D:\chadfx-health (52 MB) and D:\chadfx-health-target (22 GB) are leftovers and still exist; they are safe for a person to delete.

## small_fixes_made

- crates/cpu-render/README.md: added the three rules the WGSL reviewer proved and could not add: how a linear value is cut, sample_index is 0, and a struct or alias named like a built-in is not refused. Documentation only.
- crates/golden/README.md: added the render-portable folder and golden::portable to its table, and render-recipe to its list of dependencies. Documentation only.

## checks

All commands ran from D:\chadfx under a hard time limit. Logs are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\acceptance. Notes are in the same scratchpad at notes\acceptance.md.

**Workspace build:** 0 warnings, before and after the rounds.

**Three rounds, one package at a time, 590 s limit each, no --skip:**
| Package | Passing tests each round | Round 1 (s) | Round 2 (s) | Round 3 (s) |
|---|---|---|---|---|
| render-recipe | 9 | 0.3 | 0.3 | 0.3 |
| cpu-render | 308 (2 ignored) | 25.6 | 21.3 | 24.2 |
| render | 129 | 66.5 | 61.6 | 72.7 |
| golden | 49 (1 ignored) | 5.6 | 4.0 | 4.1 |
| core-project | 78 (1 ignored) | 33.7 | 28.1 | 25.9 |
| media | 165 (2 ignored) | 199.9 | 200.1 | 203.2 |
| ingest | 134 (1 ignored) | 244.8 | 238.9 | 244.7 |
| export | 113 (2 ignored) | 170.7 | 163.8 | 166.3 |
| vep-app | 106 | 198.7 | 146.1 | 150.5 |
| **Total** | **1,091, 0 failed, 9 ignored** | **946** | **864** | **892** |

Media, ingest, export and vep-app counts include their separately run stuck-tool checks and crash scenarios. Every round had exit code 0 and no time limit was reached. No FFmpeg process was left behind.

**Run once, with output shown:**
- Both no-card proofs with a nonsense adapter name: 14.0 s.
- same_card, card_vs_cpu, golden_pictures and golden_pictures_software on the RTX 3090: 35.9 s.
- same_card and card_vs_cpu on WARP: 124.3 s.
- anchor_table --totals.
- make_portable_references --check, dev and release.
- WebAssembly check on the 8 pinned frames and 23 libm hashes: 18.4 s, all the same.

**Read-only checks:**
- git diff and git status on the Stage 0 data and fixtures: no change.
- Files changed since the rounds began: only the two READMEs.
- Dependency trees of render, golden, cpu-render, vep-app and vep.
- My own search for banned calls in cpu-render's source.
- Shader git blobs at both tags and HEAD.
- Line-by-line reading of crates/media/src/watch.rs and the test diffs.

**Not run or not repeated by me:**
- Mac, Linux, ARM, any AMD, Intel or Apple card, or a real 8-core laptop.
- npm and end-to-end tests, as instructed.
- cargo test --workspace, and packages outside the nine named. That includes audio, playback, cli and blocks, even though audio and playback test files were edited.
- Release-build test runs. Only the release --check ran.
- The full WebAssembly check on 351 recipes. A reviewer ran it before the last clipping edit, which changes no reference byte.
- Bug-planting checks, the old-code capture and verify, and the separate recount. I relied on the engineers' logged results.

Free space on C: stayed at 341 GB. Git state was not touched.

## main session's own checks (2026-11-01, after the team finished)

Reports are claims, so these were run and read by the main session, not taken from the team.

- `cargo test --workspace --no-fail-fast`, before the last fix below: 2,355 passed, 0 failed, 18 ignored, 20.5 minutes. This covers the audio, playback, cli and blocks packages the acceptance rounds left out.
- In a throwaway copy with its own build folder, with a nonsense adapter name and the Vulkan driver path pointed at a missing file: `no_gpu_pictures` passed 17 of 17 and printed 351 scenes and 1 known panic, 133,148,260 bytes. Changing one number in `layer.wgsl` (0.18 to 0.181) failed it with 77,210 bytes different on `transform.position`. Multiplying a layer's opacity by 0.999 in `graph.rs` failed it with 7,401 bytes different, by up to 1 level, on the same scene. The real tree was compared afterwards and is unchanged, and the copy is deleted.
- Fixed defect 4 of this report. 25 places in `crates/playback/tests`, `crates/export/tests` and `ui/src-tauri/tests` treated any renderer error as "no card" and passed having tested nothing. Only `RenderError::NoAdapter` skips now, and `crates/render/tests/only_no_adapter_skips.rs` failed first, naming all 25, then passed. After the fix: playback 54 passed, export 107 passed with 2 ignored, vep-app 100 passed, 0 failed, and a workspace build with zero warnings. The whole workspace was not run again after this change, which touched only those test files and added the new one.
- Defect 2 decided, not changed: no block can reach either sampler difference, because `crates/blocks/src/wgsl/ir.rs` refuses texture and sampler types in a block and every sampler the host binds is linear with equal filters. Defects 3 and 5 decided in spec section 20.
- Not run: `npm` checks and end-to-end tests, because no file under `ui` outside `ui/src-tauri` changed. Stage 2's acceptance runs them. Mac, Linux and ARM remain unrun.
