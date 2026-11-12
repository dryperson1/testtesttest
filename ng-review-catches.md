# No-GPU picture proof: ng-review-catches.md

## findings

- **title:** The frozen recipes were frozen only by a promise
  **severity:** high
  **proven:** true
  **fixed:** true
  **detail:** On a throwaway copy I changed word 43 in today's code and in all 617 layers of recipes.json, then ran make_portable_references --write. Every test passed: portable_references, old_recipes, anchor, no_gpu_recipes, no_gpu_pictures, same_card and golden_pictures_software. The hash in references.json is written from whatever the file holds, so it cannot notice. Fix: the new test the_recipes_are_frozen_byte_for_byte in crates/golden/tests/portable_references.rs pins the file's BLAKE3. The same change now fails that test. This matters most after Stage 3, when no_gpu_recipes is retired. I also recomputed the file from scratch: git archive stage-1-done, plus the recorded diff, plus the capture program, gives the same 889,535 bytes as the file in the repository.
- **title:** Nothing noticed if a shipped crate linked the software renderer
  **severity:** high
  **proven:** true
  **fixed:** true
  **detail:** On a throwaway copy I added cpu-render as a normal dependency of crates/export, of ui/src-tauri, and through a platform table in crates/render after its dev-dependencies. All the tests that looked at this passed, including the_software_renderer_is_only_a_development_dependency and the lock-file walk in card_vs_cpu. The old check split one manifest at the first [dev-dependencies]. Fix, in no_gpu_pictures.rs: the_software_renderer_is_only_a_development_dependency now reads every workspace manifest, with every table and platform, for cpu-render and golden. what_ships_holds_neither_the_software_renderer_nor_the_references runs cargo tree on vep-app and vep. All three planted cases now fail both tests. On the real tree cargo tree -p vep-app -e normal holds neither crate, and render and render-recipe are present.
- **title:** The picture proofs could not see the neutral colour migration rule
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** Today a neutral node changes no number, because the renderer always runs the colour maths. So removing the rule from clips or from sequences, adding a node to lists that already have colour, or switching the node off left no_gpu_recipes, no_gpu_pictures and same_card green. Only core-model and core-project noticed, even though the proof header says the migration is part of what it covers. Fix: the new test the_migration_gave_the_documents_their_neutral_colour_nodes in no_gpu_pictures.rs writes the rule out separately and checks all 17 Stage 0 documents, with 269 nodes on sequences and 463 on clips. All four planted changes now fail it. Giving the neutral node an exposure was already caught by the proofs. Two parts of the rule cannot be seen from Stage 0: a list whose only colour node is disabled, and an adjustment clip with no colour node. No Stage 0 document has either. core-model's fixtures catch both.
- **title:** The card's alpha blend factors were tested by nothing
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** On a throwaway copy I set the alpha source factor to zero, and separately the alpha destination factor to one, in gpu.rs. same_card, card_vs_cpu, recipe_on_card, golden_pictures_software, render and reuse all stayed green. No picture of a frame can see these, because the output pass ignores the composite's alpha. Fix: the new test over_adds_the_source_and_keeps_the_destination_in_colour_and_alpha_on_both_renderers in recipe_on_card.rs draws into a 16-bit float target with values it holds exactly, on the card and in the software renderer. Both planted changes now fail it. It also passes on WARP. The software renderer's alpha factors were already caught by its own tests.
- **title:** The check that no graphics device was opened missed a wgpu instance
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** On a throwaway copy, frame_recipe called wgpu::Instance::new for every scene. Both proofs passed and printed that no graphics device was opened, because graphics_device_opened only knows the renderer's own device. Importing a wgpu type into graph.rs also went unnoticed. Fix: nothing_these_proofs_go_through_names_the_graphics_library searches the plan, recipe and test sources, ignoring comments and strings. this_program_names_no_graphics_library, in both programs, reads the program's own file on Windows. The planted changes now fail. Opening the device from the plan was already caught.
- **title:** Operating system maths could enter a frame's numbers unnoticed
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** On a throwaway copy I added powf to fraction() in graph.rs. Nothing noticed: it gives the same bytes on this PC and would only show on another machine, which breaks the claim that no_gpu_recipes runs anywhere. Fix: a_frames_numbers_use_nothing_that_rounds_differently_elsewhere searches the plan and recipe files, core-time and core-model for pow, exp, log, trigonometry, hypot, cbrt and mul_add. The planted call now fails it. The test also checks that none of the 22 mask path coordinates in Stage 0 is a negative zero, so graph.rs's std f64::min and max cannot affect the words these proofs compare.

## contract_changes

- No product code, public API, recipe, shader or reference byte changed. References were not regenerated and the anchor table is unchanged.
- Only these files changed: crates/render/tests/no_gpu/mod.rs, crates/render/tests/no_gpu_pictures.rs, crates/render/tests/no_gpu_recipes.rs, crates/render/tests/recipe_on_card.rs, crates/golden/tests/portable_references.rs, crates/golden/data/render-portable/README.md.
- Test counts: no_gpu_pictures goes from 10 to 17, no_gpu_recipes from 13 to 14, recipe_on_card from 14 to 15, portable_references from 14 to 15.
- New test helpers in tests/no_gpu/mod.rs: graphics_libraries_this_program_names, code_of, has_word.
- New pinned values: RECIPES_BLAKE3 = 7492ef1baba983651de94efae21d76312ffc4b91a9c537e990926df165d4c566 and NEUTRAL_NODES = (269, 463).
- For Stage 3: when a frame's numbers pass through crates/blocks, add the packing code to the maths search in a_frames_numbers_use_nothing_that_rounds_differently_elsewhere. no_gpu_pictures counts layers from the first pass and treats image 0 as the white picture, so if a composed frame is shaped differently, change the counting and never the comparison. Its header now says this, replacing the earlier claim that it keeps working unchanged.

## checks

**Do the two proofs compare all 351 scenes with no adapter involved? Yes, on this PC.**
- I ran no_gpu_recipes and no_gpu_pictures with VEP_RENDER_ADAPTER set to a meaningless name and the Vulkan driver variables pointed at a missing file. Both passed: 351 scenes and the known panic, 133,148,260 bytes in the picture proof.
- While they ran, I read each process's loaded libraries every 10 ms (123 and 1,374 looks). No graphics or window library was loaded.
- Their import tables name no dxgi, d3d, opengl32, vulkan, gdi32 or user32. same_card, built from the same crate, does name them.
- no_gpu_recipes imports only floor, fmod and round from the C runtime maths, and all three are exact.
- On a throwaway copy, I made gpu::shared and wgpu::Instance::new end the process. Both proofs still passed, 14 and 17 tests.

**Five kinds of planted change, one at a time on a throwaway copy, each run once.** The planted-bug run covered 45 changes, with the unchanged copy passing first. Survivors were re-run after the new tests. Results are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\review\results.txt.

| Change | What caught it |
|---|---|
| Uniform value | no_gpu_recipes, no_gpu_pictures, same_card, golden_pictures_software |
| Uniform word no program reads | no_gpu_recipes only, correctly |
| Card sent too few uniform words | same_card, card_vs_cpu, recipe_on_card, golden_pictures_software |
| Shader line | no_gpu_recipes, no_gpu_pictures, same_card, golden_pictures_software |
| Comment, or a line no scene reaches | no_gpu_recipes only, correctly |
| Blend factor, recipe | no_gpu_recipes; also the picture and card tests when layers replace. An output pass that blends draws the same, so only no_gpu_recipes sees it |
| Blend factor, card colour | same_card, card_vs_cpu, golden_pictures_software |
| Blend factor, card alpha | recipe_on_card, one of three only before the fix; all three now |
| Blend factor, software renderer | cpu-render's own tests; colour also no_gpu_pictures and card_vs_cpu |
| Draw order, recipe or plan | no_gpu_recipes, no_gpu_pictures, same_card, golden_pictures_software |
| Draw order, card | same_card, card_vs_cpu, recipe_on_card, golden_pictures_software |
| Draw order, software renderer | cpu-render's own tests, no_gpu_pictures, card_vs_cpu |
| Neutral rule | see the neutral finding |

**Can the picture proof see every number?** I changed each of the 68 uniform words in turn and looked for the change in the bytes.
- Every word a program reads shows up, nearly all when off by a thousandth. The four flag words show at plus one.
- Only the 10 words no program reads never show.

**Is the record of the capture right?** A fresh capture from tag stage-1-done gives the same bytes as recipes.json. Both frozen shaders match the tag byte for byte.

**Is cpu-render out of what ships?** cargo tree --offline -p vep-app -e normal holds no cpu-render and no golden. The same holds for vep, also with --all-features (283 and 147 crates listed).

**Results on the real tree after the last change**
| Command | Result |
|---|---|
| cargo build --workspace --all-targets | exit 0, 0 warnings |
| cargo test -p render -p golden -p cpu-render -p render-recipe --no-fail-fast | 490 passed, 0 failed, 3 ignored (2 in another engineer's wgsl_spec_tests, 1 old sound.rs doc example) |
| cargo clippy -p render -p golden --all-targets | no findings in my files; older ones elsewhere |
| VEP_RENDER_ADAPTER=software, same_card, recipe_on_card, card_vs_cpu | all pass; the old path draws the Stage 0 WARP dev set on all 351 scenes |
| cargo test --release, no_gpu_recipes and no_gpu_pictures | 14 and 17 pass |

**cargo test --workspace** did not finish in one run. It hit the 590 s limit in ingest's thumbs test, and I stopped it by process id with its children. No ffmpeg or ffprobe process was left running afterwards. I ran the rest package by package:
| Part | Result |
|---|---|
| Up to the limit | 1,655 passed, 7 failed, 13 ignored |
| export, playback, jobs | 253 passed, 0 failed, 2 ignored |
| vep, vep-app | 187 passed, 0 failed |
| media | 150 passed, 0 failed, 2 ignored |
| ingest thumbs alone | 26 passed, 0 failed |

The 7 failures were all in crates/cpu-render/src/wgsl_spec_tests.rs, a file another engineer was editing at the time. They pass in the final four-package run above.

The earlier parts ran before the final four-package run, while other engineers were still editing. Stage 0 data shows no change in git. The throwaway copy and its build folder are deleted, and free space on C: is 340 GB.

**Not run:** Mac, Linux, ARM, any AMD, Intel or Apple card, ui npm tests and e2e, and one uninterrupted cargo test --workspace.

## remaining_risks

- Mac, Linux and ARM have not been run or compiled for. The no-adapter evidence above is from Windows only. this_program_names_no_graphics_library reads the program file only on Windows and prints that it did not check elsewhere; the source search runs everywhere.
- no_gpu_pictures and old_recipes import pow, sin and similar from the Windows C runtime, as far as I can tell for naga's constant folding. I did not trace those references. That they cannot affect a byte rests on cpu-render refusing folded calls, which I did not attack in this job. Whether naga folds anything inexact outside the refused list is for the software renderer review.
- The maths and graphics searches are text searches over named files. A call through a function pointer or another crate would escape them, and Stage 3 must add crates/blocks to the maths list.
- Stage 0 never sets green lift, gamma or gain in the output pass to anything but the default; red and blue do vary. That code path is shared with layers, where green does vary, and no scene can be added.
- Two parts of the neutral rule (a list with only a disabled colour node, and an adjustment clip with no colour node) are held only by core-model's fixtures, because no Stage 0 document has that shape.
- Before Stage 3, the alpha of the composite is invisible to every picture proof. The new test covers the blend arithmetic only on hand-made recipes, so any block that reads composite alpha needs its own proof.
- graphics_device_opened still reports only devices that opened successfully, so a failed attempt on a machine with no adapter is not counted. The source search covers this for the proofs.
- The workspace test evidence comes from separate runs, some while other engineers were editing. The acceptance checker should run it whole, in a window longer than 10 minutes or split by package.
- crates/golden/README.md and README-STAGES.md still do not mention this folder or these tests. I do not own them, so the main session should update them.
