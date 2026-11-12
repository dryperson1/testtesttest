# No-GPU picture proof: ng-review-wgsl.md

## findings

- **title:** A value with @interpolate(linear) came out wrong wherever a cut met different w
  **severity:** medium
  **proven:** true
  **fixed:** true
  **detail:** The fix is in crates/cpu-render/src/raster.rs. `clip` cut every interpolated value at `t`, the share of the way along the edge in clip space. That is right for perspective-correct values. A linear value is straight on the screen, so it must go by `s = t * w_b / w`, as Vulkan says of noperspective.

Proof against real hardware, using the same recipe:
- Pixel 8 of a row: RTX 3090 0.2654, WARP the same, this renderer 0.0961. The screen formula gives 0.2656.
- Over 400 random triangles (409,600 pixels), the largest difference from the RTX 3090 was 0.369, on 8,733 pixels beyond 0.02.

After the fix:
- Largest difference from the RTX 3090 is 0.00098 on every channel, with 0 pixels beyond 0.02. That includes a vertex behind the eye.
- WARP also agrees on the depth cut. Behind the eye WARP gives NaN for this value, so that case has no comparison on WARP.

Equal w keeps `t` itself, to the bit. At the plane of smallest w the formula uses that plane's exact w.

Tests kept:
- a_linear_value_is_linear_on_the_screen_after_a_cut
- a_linear_value_follows_the_screen_whatever_was_cut
- with_equal_w_a_cut_treats_both_kinds_of_value_alike
- in raster.rs: a_cut_keeps_a_linear_value_straight_on_the_screen, behind_the_eye_a_linear_value_goes_by_the_exact_w_of_the_cut, with_equal_w_both_kinds_of_value_are_cut_alike

No stored byte changed: layer.wgsl and output.wgsl use no linear value.
- **title:** Valid WGSL with @builtin(sample_index) or @interpolate(.., sample) was refused
  **severity:** low
  **proven:** true
  **fixed:** true
  **detail:** The README says center, centroid and sample are all the same here, and run.rs has code for sample_index. The validator refused these programs first, because the MULTISAMPLED_SHADING capability was off, so that code could not be reached. The RTX 3090 and WARP take these programs.

Fixed by turning the capability on in crates/cpu-render/src/front.rs. There is one sample per pixel, so nothing else changes.

Test: sample_interpolation_and_the_sample_index_are_wgsl.
- **title:** A struct or alias named like a built-in was refused with a false message
  **severity:** low
  **proven:** true
  **fixed:** true
  **detail:** `struct max { a: f32 }; ... max(0.5)` and `alias pow = f32; ... pow(0.5)` are valid WGSL, and a card draws them. The look through the text took them for calls folded with a maths library and refused them.

Fixed in crates/cpu-render/src/front.rs: the scan now records the names given by `struct` and `alias`.

Test: a_type_named_like_a_built_in_is_not_a_built_in. It also checks that real folded calls stay refused however the text is dressed up: the name in a comment, a member named `sin`, or a struct of another name.
- **title:** textureSampleBaseClampToEdge ignores a nearest sampler, which a card does not
  **severity:** low
  **proven:** true
  **fixed:** false
  **detail:** The WGSL function takes any filtering sampler and limits its coordinates first. naga passes that sampler through, and the RTX 3090 and WARP both used its nearest filter. This renderer always uses linear. The README states that rule and tests pin it, so I left it.

The fix is one condition in `texture::sample`: drop `&& !info.clamp_to_edge`. Tried on a throwaway copy:
- The proof test passes.
- tests/old_recipes.rs still passes, so no stored picture changes.
- tests/recipes.rs line 626 fails.
- The pinned frame "pictures, nearest" in tests/everything.rs fails.

I do not own those two tests or the README. The proof is kept as the #[ignore]d test base_clamp_uses_the_filter_of_its_sampler. Today's samplers are all linear, so no picture changes today either way.
- **title:** An explicit level above 0, or a gradient over one pixel, should choose the minification filter
  **severity:** low
  **proven:** true
  **fixed:** false
  **detail:** With min_filter different from mag_filter, the RTX 3090 and WARP take min_filter for textureSampleLevel above level 0 and for textureSampleGrad with a large gradient, even with one level. This renderer always takes mag_filter, and its README says so.

I built the fix and took it back out, because tests/everything.rs fails on its pinned frame "pictures, nearest". With the fix in the tree, old_recipes, anchor and recipes still passed. The change is about 25 lines; C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\review-wgsl\apply1.py holds it.

The proof is kept as the #[ignore]d test an_explicit_level_or_gradient_chooses_the_filter. It passed while the fix was in.

Thresholds differ between cards, so such a fix would be a stated rule rather than any one card's behaviour: NVIDIA ignored level 0.001 where WARP did not, and NVIDIA took min_filter at gradients (0.75, 0.5) in both directions where WARP did not.

## contract_changes

- No public function, type or signature changed. `raster::clip` gained a parameter, but it is internal.
- Now accepted: `@builtin(sample_index)` (reads 0), `@interpolate(perspective|linear, sample)`, and a `struct` or `alias` named like a built-in function.
- Changed bytes: only a value with `@interpolate(linear)` on a triangle that is cut, where w differs along the cut edge. Nothing in crates/golden/data/render-portable changed and it was not regenerated. old_recipes, anchor, the pinned hashes and render's no_gpu_pictures, no_gpu_recipes, card_vs_cpu, same_card and recipe_on_card all pass.
- Files changed: crates/cpu-render/src/raster.rs, run.rs (one argument), front.rs, lib.rs (wires the tests). New: crates/cpu-render/src/wgsl_spec_tests.rs. crates/render-recipe needed no change.
- crates/cpu-render/README.md needs these edits, and I do not own it. Under Clipping, after 'set exactly on the plane.', add: 'A value with `@interpolate(linear)` is straight on the screen, so where `w` differs from end to end it goes by `s = t * w_b / w` instead, `w` being the new vertex's own once it is set on the plane: `a + s * (b - a)`. Where `w` is the same at both ends, `s` is `t`.'
- README, under Built-in inputs, add: '`sample_index` is 0.'
- README, under Constants worked out by the parser, add: 'A `struct` or `alias` named like one of these hides it, and what looks like a call is a constructor, so it is not refused.'
- The deviations list says the renderer validates with `SHADER_FLOAT16_IN_FLOAT32`. It now also uses `MULTISAMPLED_SHADING`.
- tools/wasm-check's standard-library shim (fstd.rs) has no `std::iter`. Code in src must stay within what that file offers, or the WebAssembly check stops building. My first form of the clip fix hit this.

## checks

Everything was run from D:\chadfx under a hard time limit. Logs are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\review-wgsl.

**After the last code change**
| Check | Result |
|---|---|
| `cargo build --workspace --all-targets` (after touching both libs) | exit 0, 0 warnings |
| `cargo test -p cpu-render -p render-recipe --no-fail-fast` | 316 passed, 0 failed, 2 ignored (the two proofs above) |
| `cargo clippy -p cpu-render -p render-recipe --all-targets` | no findings in these crates |
| WebAssembly check on the 8 pinned frames | "Everything is the same", 23 libm hashes the same |

**Before the last small change**
That change rewrote the clip fix's line in another form, with the same arithmetic.
| Check | Result |
|---|---|
| `cargo test -p cpu-render -p render-recipe -p golden` | 365 passed, 0 failed, 3 ignored (the two proofs and one older doc example) |
| `cargo test -p render --test no_gpu_pictures --test no_gpu_recipes --test card_vs_cpu --test same_card --test recipe_on_card` | 69 passed on the RTX 3090 |

**Comparison with real hardware**
A throwaway program drew the same recipes on the card and in this renderer. It is not in the repository; its source is in review-wgsl\card\src. Adapters: RTX 3090 on Vulkan and WARP on DX12.
- **Built-ins:** 56,848 words through bit casts. 544 differ on the 3090 and 772 on WARP. I looked at samples of every kind:
  - pow of a negative base
  - sin near 0 and at huge inputs
  - pack4x8snorm(-0.5): this renderer follows the WGSL formula (-63), the card gives -64
  - f32 to integer overflow: card 0x7FFFFF80, here 0x7FFFFFFF
  - quantizeToF16 of a subnormal
  - mix(1e30, 3, 1): this renderer follows the WGSL formula
  - on WARP, sign and others that its own compiler gets wrong
  
  None was a defect here.
- **Coverage:** 2,268 turned and flipped quads on targets from 1x1 to 64x40, and 409,600 pixels of random triangles, match the 3090 exactly. On WARP 51 quads and 9 pixels differ.
- **Position, depth, facing, flat values:** same as the 3090, cut or not. WARP gives w where WGSL says 1/w in position.w.
- **Uniforms:** 28 members of a struct with every alignment rule read alike on both.
- **Other:** two-pass recipes and blending in both formats agree. The one blending difference is a card's clear of 0.5 to 8 bits. About 45 programs of assorted spellings gave the three acceptance findings.
- **Random programs:** 4,273 runs of integers and whole-number floats matched the 3090 word for word, and 1 differed. Worked by hand by the WGSL rules, this renderer is right on that one (seed 4490). I did not find out why the card differs.

**Attacked and found sound:** math built-ins against a table of known values, per-component on every vector size, all 340 swizzle patterns, matrix order in every shape, select, integer wrap and shift masks, struct layout and padding, pointers, control flow, parser folding against run time, the implicit level, clamp to edge, sampling beyond every edge, the top-left rule, flipped quads, and outputs of 1x1 and 2x2. The render-recipe codec, read through, gave no finding.

**Planted bugs:** 37 on one throwaway copy, and the unchanged copy passed first. All 37 were caught. The three whose code then changed form were planted again and caught again. The list and results are in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\review-wgsl\plant-results.txt.

**Not run:** Mac, Linux, ARM, `cargo test --workspace`, a release build of these tests, and the WebAssembly check on the 351 old recipes. Random programs on WARP stopped at once: its shader compiler failed with an internal error on one and the graphics library panicked. The graphics driver also ended the random-program process twice on the 3090; those batches gave no summary and are not in the counts.

**Disk:** throwaway copies and build folders deleted, 339 GB free on C:. Git state, Stage 0 data and files outside my paths were not touched.

## remaining_risks

- Mac, Linux, ARM, and AMD, Intel and Apple graphics have still never run this code.
- The main session must apply the README text in contract_changes. Until then the code and the document that defines it disagree on the linear cut.
- Someone who owns tests/recipes.rs and tests/everything.rs must decide the two proven differences on samplers: base clamp with a nearest sampler, and the minification filter by explicit level or gradient. Both change only samplers whose two filters differ or are nearest; no stored picture changes. Taking either needs a new pin for the frame "pictures, nearest", and base clamp also needs line 626 of tests/recipes.rs changed.
- An implicit-level `textureSample` that makes a picture smaller, on a sampler whose two filters differ, stays different from a card, because there are no derivatives here. Today's samplers cannot reach this.
- Measured differences from cards that WGSL allows, left as this crate's stated rules: a negative array index gives the first element here and the last on the card; `textureLoad` out of range gives the edge pixel here and 0 on the card; f32 to integer overflow is noted in the checks. A block leaning on any of these would look different on a card.
- From reading only, not measured: a uniform buffer shorter than the WGSL size of its type is accepted when the type ends in padding at top level. tests/recipes.rs pins 28 bytes as enough for a `mat2x3`, whose WGSL size is 32, and `array<vec3<f32>, N>` behaves the same way. No byte can differ, but a card may refuse the same recipe.
- Vertices about 1e13 pixels away or more lose their edge's position in the clip sums. A triangle with corners at 1e20 covered 228 of 256 pixels here, 256 on the RTX 3090 and 0 on WARP, and I worked out the right answer as about half. It is deterministic here. I did not fix it, because a card does no better.
- naga 30.0.1 panics on a constant `select` whose first argument is a vector built from a shorter vector (seen in the generated program panic-2694; no smaller case tried). Here it comes back as `Error::Internal`. wgpu raised it on the calling thread in the throwaway tool, so Stage 3 should keep that parse behind a guard. crates/blocks/src/code.rs guards validation; I did not check the parse at crates/blocks/src/wgsl/compose.rs:369.
- The random programs cover integers and whole-number floats only. Other float results were compared on the fixed list of built-ins and values, with a tolerance.
- The card comparisons are a one-off from a throwaway program. wgsl_spec_tests.rs keeps the rules they showed, not the comparison itself. card_vs_cpu.rs does not cover linear values or clipping.
