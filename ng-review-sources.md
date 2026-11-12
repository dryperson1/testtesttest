# No-GPU picture proof: ng-review-sources.md

## findings

- **title:** A renderer that could not open a device passed every test in crates/render
  **severity:** critical
  **proven:** true
  **fixed:** true
  **detail:** Every test that needs a card treated any error from Renderer::new as 'this computer has no card', and passed. A failed pipeline comes back as RenderError::Device, so it was covered too.

Proof on a throwaway copy, two plants:
- Gpu::create returning RenderError::Device: cargo test -p render exited 0, 125 passed, 0 failed.
- A real validation error (a binding's visibility changed in gpu.rs): same_card, card_vs_cpu, golden_pictures, golden_pictures_software, recipe_on_card and render all passed. The log shows 'SKIPPED: this computer has no graphics adapter...', though cargo hides that line unless --nocapture is used.

Fix: only RenderError::NoAdapter skips, at all 9 skip sites in crates/render. Those are same_card, card_vs_cpu, golden_check, common::can_render, threads, render, bad_requests, first_use and the unit tests in src/renderer.rs. Anything else now fails.

After the fix the first plant fails 65 tests and the second fails 12 in same_card and card_vs_cpu. New test in both files: an_adapter_the_renderer_cannot_use_is_a_failure.
- **title:** The eight hash-only scenes had no anchor: a bias confined to them passed every no-card test
  **severity:** high
  **proven:** true
  **fixed:** true
  **detail:** I planted one-level biases in a copy of cpu-render.

Caught: everywhere, on one level only, on dark levels only, and a rounding point moved to 0.5003. old_recipes failed for each. After --write regenerated the references, anchor failed too, and the rule itself failed, not only the pinned numbers.

Not caught: one level brighter on only the 1280x720 scene, or only on the 1920x1080 noise scenes. After --write, anchor passed 9 of 9. For the 1280x720 plant I also ran the other tests: cpu-render, golden and render-recipe passed 365 tests and no_gpu_recipes and no_gpu_pictures passed. Only card_vs_cpu on a real card noticed. So the comment 'cannot move without this test saying so' was false for 8 of 351 scenes.

Fix:
- anchor.rs now pins the BLAKE3 of references.json (these_are_the_references_that_were_measured). The plant now fails it.
- card_vs_cpu gains where_this_card_draws_a_stored_set_the_large_scenes_measure_what_was_recorded. It re-measures the constants that were measured once by hand wherever the card draws a Stage 0 set exactly. It passes on the RTX 3090 and on WARP in a dev build; the plant fails it.
- **title:** The way the export draws was never compared with the old code
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** same_card only used render_into, the preview's path. The export uses layers_at, submit_plan and collect_into, which has its own code between the plan and the card.

Plant: submit_plan ignoring the background it is given. cargo test -p render passed 125 of 125.

Today's export always passes black, so this could not show in the product yet. It is still a gap in the rule that preview equals export.

Fix: same_card compares both ways on all 351 scenes and the known panic (every_old_recipe_and_the_way_the_export_draws_give_the_same_bytes_on_this_card). The export plant now fails only that test, and a matching plant in the preview path fails only the preview test.
- **title:** The record of how recipes.json was made was frozen only by a promise
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** recipes.json itself was already pinned. The two files the 'made again from the tag' claim rests on were not.

Plant: editing capture_recipes.rs.txt left portable_references passing.

Fix: the_capture_is_on_record pins both capture files by BLAKE3. Editing either file now fails it.
- **title:** The card bound allowed a 2x2 picture to be wrong throughout, and allowed alpha to differ
  **severity:** low
  **proven:** true
  **fixed:** true
  **detail:** Two looseness problems in card_vs_cpu:
- The limit of 31 bytes off by 4 levels or more was larger than the whole output.minimum_1x1 picture, which has 16 bytes.
- Alpha could be off by one on a whole family of scenes, within the limit over the run, though the output program writes 1.0 on every pixel.

My recount found alpha never differs in any pair of stored pictures, or in the 8 large scenes.

Fix: the by-4 limit is at most a sixteenth of a picture's bytes, and no alpha byte may differ. damage_to_every_picture_goes_beyond_the_bound has new cases for both. Removing either rule on the copy makes that test fail. Both still pass on the RTX 3090 and WARP.
- **title:** A dev-set assertion claimed the same scenes but checked only the count
  **severity:** low
  **proven:** false
  **fixed:** true
  **detail:** the_dev_profile_sets_tell_the_same_story's comment says the rule holds with the same scenes, yet it only compared the number of scenes. It now compares the names, and it passes. No planted change shows a failure this catches, so this is a small tightening rather than a proven defect.

## contract_changes

- No product behaviour, recipe, shader, reference picture or anchor table changed. references.json and recipes.json are byte-identical. No regeneration was needed.
- crates/render/src/renderer.rs changed only in the doc comment of Renderer::new and in its unit tests' skip helper. There is no public API change.
- Rule for all tests in crates/render: only RenderError::NoAdapter may skip. Any other error from Renderer::new fails the test.
- New pinned values:
- REFERENCES_BLAKE3 = df095e57dc747b1f042d1c7ee5f09fc84896fe32a346472e0c0038bceec6d503 in crates/cpu-render/tests/anchor.rs
- CAPTURE_PROGRAM_BLAKE3 = 186f056dcedba3d6cebea92b958988ea31181d6570624501e1a9569abc915bc2 in crates/golden/tests/portable_references.rs
- CAPTURE_CHANGES_BLAKE3 = beaee84ff08337c223b60035c3bf63dca14f145018313dec266ff17a1189339e in crates/golden/tests/portable_references.rs
- If anyone deliberately regenerates references.json, these_are_the_references_that_were_measured fails until the anchor numbers and the 8 large scenes are measured again and REFERENCES_BLAKE3 is updated.
- New tests:
- same_card: every_old_recipe_and_the_way_the_export_draws_give_the_same_bytes_on_this_card, an_adapter_the_renderer_cannot_use_is_a_failure
- card_vs_cpu: where_this_card_draws_a_stored_set_the_large_scenes_measure_what_was_recorded, an_adapter_the_renderer_cannot_use_is_a_failure
- anchor: these_are_the_references_that_were_measured
- card_vs_cpu judge_scene has two new rules: an alpha byte may not differ, and the by-4 limit is the smaller of 31 and total/16. Row has a new field, alpha.
- Files changed:
- crates/render/src/renderer.rs
- crates/render/tests/same_card.rs, card_vs_cpu.rs, common/mod.rs, golden_check/mod.rs, bad_requests.rs, first_use.rs, render.rs, threads.rs
- crates/cpu-render/tests/anchor.rs
- crates/golden/tests/portable_references.rs
- crates/golden/data/render-portable/README.md (new section 'A second review of the proofs')

## checks

**Frozen recipes: did the old code really make them? Yes.**
- I ran git archive stage-1-done into an empty folder, applied capture/old-code-changes.diff, added capture_recipes.rs.txt, built with its own target folder and ran --capture. Result: the same 889,535 bytes as recipes.json.
- The tag's shaders match the frozen ones byte for byte, and the tag is commit c89816a.
- The capture writes formats, vertex counts, blend names, the clear colour and the sampler by hand rather than reading them from the old code. A drawing check covers those: the old path draws the WARP dev set on 351 of 351 (dev build) and the WARP release set on 351 of 351 (release build). With the adapter key forced on a copy, it draws the A4000 set on 351 of 351 on the RTX 3090.
- The old code's own --draw on the RTX 3090 matches the A4000 set on 351 of 351, including the 8 hash-only scenes. On WARP it matches the debug set on 351 and the release set on 346.

**Anchor numbers: recounted independently (Python with PIL and numpy, no shared code).**

| Pair | Bytes differing | Identical scenes | Largest | By 1 / 2 / 3 / 4+ |
|---|---|---|---|---|
| software / NVIDIA | 739,566 | 57 | 6 | 739,074 / 444 / 32 / 16 |
| software / WARP | 150,318 | 257 | 193 | 147,545 / 2,725 / 41 / 7 |
| NVIDIA / WARP | 812,095 | 54 | 193 | 808,426 / 3,559 / 86 / 24 |

- All 351 rows of anchor-table.tsv match.
- Odd one out: software 37,742, WARP 110,271, NVIDIA 699,519, all three apart 2,305.
- The 8 large scenes recomputed from fresh old-code drawings give LARGE_SOFTWARE_NVIDIA, LARGE_SOFTWARE_WARP and LARGE_NVIDIA_WARP exactly.

**Does same_card fail if Renderer draws differently? Yes, for what I planted.** Back-face culling and a nearest min filter on the card each failed same_card and card_vs_cpu. Writing no alpha fails only recipe_on_card, because no frame's picture can show it. The old path takes nothing from the render library, so it is independent where it needs to be.

**Does card_vs_cpu's bound mean anything? Yes, within the limits under remaining risks.** It failed the plants above. A signed bias rule looked like a tightening, but I measured it and rejected it, because it would fail honest cards: rounding up at 0.4 raises one channel of sources.half_decode by 0.86 of a level on average.

**After the last code change, on the real tree, each under a time limit:**

| Command | Result |
|---|---|
| cargo build --workspace --all-targets | exit 0, 0 warnings |
| cargo test -p render --no-fail-fast | 129 passed, 0 failed, 0 ignored, 68 s. Before the last comment edit and the one-line assertion change in anchor.rs; render's test code did not change after it |
| cargo test -p cpu-render -p golden -p render-recipe --no-fail-fast | 366 passed, 0 failed, 3 ignored, 28 s. Run before that assertion change |
| cargo test -p cpu-render --test anchor, after that change | 10 passed |
| same_card and card_vs_cpu on the RTX 3090, dev and release | pass (14 + 13) |
| same_card and card_vs_cpu on WARP, dev and release | pass (14 + 13) |

The WARP and release runs came before the last comment-only edit to card_vs_cpu. cargo clippy shows no new findings in the files I changed, by file count; the one note in render/tests/common/mod.rs and the one in src/image.rs match the older notes reported earlier.

Stage 0 data shows no change in git. No planted text is left in the real tree: I searched render/src and cpu-render/src for it. Three lib.rs files had their file time touched to force a rebuild, with no content change. Temporary copies were deleted (2.8 MB of scripts and logs remain in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\review-sources). Free space on C: is 341 GB. Git state was not touched.

**Not run:** Mac, Linux, ARM, any AMD, Intel or Apple card, cargo test --workspace, and ui tests.

## remaining_risks

- Mac, Linux and ARM have not been run or compiled for. There is no WSL distribution here. No AMD, Intel or Apple card has been compared.
- The same 'any error means no card' skip is in crates I do not own, and I did not touch it: crates/playback/tests (anamorphic.rs, proxy.rs seen) and ui/src-tauri/tests (export.rs, app.rs seen; others not read). In practice a device failure now fails cargo test -p render, so the workspace would not go green, but those tests still pass vacuously on their own. Someone who owns them should apply the same rule.
- card_vs_cpu still passes a whole family of scenes drawn one level brighter when few enough bytes are involved: 7 of 17 families done to NVIDIA's stored pictures, 13 of 17 to WARP's. I found no rule that separates this from honest rounding. same_card on the card itself is what covers it. This is stated in the test's comment and the README.
- The whole-run limit of 11.276% still rests partly on a scratch measurement of simulated rounding rules that is not in the repository.
- WARP coverage of same_card and card_vs_cpu is still by hand, using VEP_RENDER_ADAPTER=software. On this PC cargo test -p render alone compares the old path with no Stage 0 set, and prints that. golden_pictures_software does cover the Renderer on WARP automatically. I found no defect from this, so I added no test.
- a_card_that_misdraws_fails asserts each misdraw is noticed, on whatever card runs it. It was only ever run on the RTX 3090 and WARP. On a card with other rounding, a small misdraw such as 2 parts in 1,000 might cancel against that card's rounding and fail the test without any real fault.
- REFERENCES_BLAKE3 means any engineer who legitimately regenerates references.json must also update this constant. That is intended, but other engineers sharing this tree should know.
- crates/golden/README.md and README-STAGES.md still do not mention this folder or these tests. I do not own them, so the main session should update them.
- Planted-change checks only show the tests notice the defects I thought to plant. The real cause of the 37,742 bytes where only the software renderer differs is still not known; earlier work established that counting explains them, not a mechanism.
