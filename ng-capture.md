# No-GPU picture proof: ng-capture.md

## summary

All six deliverables are finished and checked, and the picture reference test now runs with no graphics card. The 351 scenes the old code computed are frozen in `crates/golden/data/render-portable`. The pure-Rust software renderer reproduces all 351 byte for byte, and that needs only the files in the repository. Nothing is committed: git state was not touched.

**Is the software renderer faithful to real hardware?** Yes, by the agreed rule. Over the 343 scenes stored as pictures, it differs from the NVIDIA RTX A4000 on 1.038% of bytes and from WARP on 0.211%, while those two differ from each other on 1.140%. All 54 scenes the two adapters draw alike, it draws alike. No interpreter or rasterizer defect came out, so `crates/cpu-render/src` was not changed.

**This PC:**
- WARP reproduces the Stage 0 WARP set on all 351 scenes, in dev and release builds.
- The RTX 3090 matches the A4000 byte for byte on all 351, including the 8 stored only as a hash. I re-checked this separately because only the README claimed it. So the software renderer is exactly as close to the 3090 as to the A4000, but this is a weak out-of-sample check: it is a second NVIDIA card with the same rounding.

**What I found and fixed in this session:**
1. A real defect that would have failed `cargo test --workspace`: a doc example in `crates/golden/src/portable.rs` named `cpu_render`, which `golden` must not depend on. The earlier attempt never ran a whole-package test, so it was missed.
2. The record of what was changed in the old-code copy left out one line of `Cargo.lock`. Added; applying the record to the tag now reproduces the copy's five changed files exactly.
3. The earlier explanation for bytes where both adapters agree and this renderer does not hinted at `pow`. Tests on a throwaway copy rule that out. The explanation that holds is counting: near a rounding boundary, three renderers usually have two possible values, so one is the odd one out. This renderer is the odd one out least often (37,742 bytes, against WARP 110,271 and NVIDIA 699,519). A new test pins it. What it does not give is a byte-level cause.
4. A README sentence claimed more than the data shows ("at or just above one half"); corrected.

**Planted bugs:** 8 of 8 fail both `old_recipes` and, after regenerating the references with the bug in place, `anchor`.

**Tests:** `golden`, `cpu-render` and `render-recipe` give 314 passed, 0 failed, 1 ignored (an older doc example in `sound.rs`), with no warnings.

**Not run:** Mac, Linux, ARM (no WSL distribution here) and the full workspace suite.

## contract

## Where things are
- `crates/golden/data/render-portable/`: `recipes.json`, `references.json`, `shaders/layer.wgsl`, `shaders/output.wgsl`, `images/<scene>.<hash16>.png`, `anchor-table.tsv`, `capture/capture_recipes.rs.txt`, `capture/old-code-changes.diff`, `README.md`
- `crates/golden/src/portable.rs` (module line in `crates/golden/src/lib.rs`)
- `crates/golden/tests/portable_references.rs` (14 tests)
- `crates/cpu-render/examples/make_portable_references.rs`, `crates/cpu-render/examples/anchor_table.rs`
- `crates/cpu-render/tests/old_recipes.rs` (12 tests), `crates/cpu-render/tests/anchor.rs` (9 tests)
- `crates/cpu-render/Cargo.toml` has `golden = { path = "../golden" }` as a dev-dependency.

## golden::portable (depends on render-recipe, no editor crate)
Constants:
- `RECIPES_FILE`, `REFERENCES_FILE`, `IMAGES_FOLDER`
- `RECIPES_BUDGET_BYTES: u64 = 5_000_000`, `FOLDER_BUDGET_BYTES: u64 = 30_000_000`
- `PNG_LIMIT: (u32, u32) = (640, 360)`, `MAX_LARGE_PNG: usize = 1_000_000`
- `LAYER_UNIFORM_WORDS: usize = 48`, `OUTPUT_UNIFORM_WORDS: usize = 20`

Functions:
- `portable_dir() -> PathBuf`
- `load_recipes() / load_recipes_from(&Path) -> Result<RecipeIndex, GoldenError>`
- `load_references() / load_references_from(&Path) -> Result<ReferenceIndex, GoldenError>`
- `uniform_bytes(&str, usize) -> Result<Vec<u8>, String>`
- `check_output(&SceneReference, u32, u32, &[u8]) -> Result<(), GoldenError>`; `check_output_in(&Path, &SceneReference, u32, u32, &[u8])`
- `keeps_png(u32, u32, usize) -> bool`
- `png_file(&str, &str) -> String`
- `reference_of(&str, u32, u32, &[u8]) -> Result<(SceneReference, Option<Vec<u8>>), GoldenError>`

Types (all serde with `deny_unknown_fields`):
- `RecipeIndex { format, about, made_by, composite_format, output_format, output_clear, white, sampler, layer_shader: ShaderEntry, output_shader: ShaderEntry, scenes: Vec<SceneRecipe>, known_panics: Vec<KnownPanicRecord> }`, with `.scene(&str)`
- `ShaderEntry { file, blake3, vertex_entry, fragment_entry, vertex_count, blend, target }`
- `SceneRecipe { name, document, sequence, time: Fraction, output: [u32; 2], sequence_size: [u32; 2], background: Vec<String>, layers: Vec<LayerRecipe>, output_uniforms: String, source_requests: Vec<SourceRequest> }`
- `LayerRecipe { clip, uniforms: String, picture: Option<PictureRecord> }`
- `PictureRecord { asset, time: Fraction, frame: i64, size: [u32; 2], pattern: Pattern, seed: u64, rgba_blake3 }`
- `KnownPanicRecord { name, document, sequence, time, message }`
- `ReferenceIndex { format, about, made_by, recipes_blake3, layer_shader_blake3, output_shader_blake3, scenes: Vec<SceneReference> }`, with `.scene(&str)` and `.to_text() -> Result<String, GoldenError>`
- `SceneReference { name, output, expected: picture::Expected }`
- `Recipes { dir, index, file_blake3, .. }`: `Recipes::load()`, `Recipes::load_from(&Path)` (each shader's hash is checked), `.recipe(&SceneRecipe) -> Result<render_recipe::Recipe, GoldenError>` (remakes every picture from the frozen generators and checks its hash; bad data gives an error, never a panic)

## File formats
- `recipes.json`, format 1: uniforms are space-separated words of 8 hex digits, each the bits of one f32 (`3f800000` is 1.0), read little-endian. 64-bit colours are 16 hex digits of their bits. 48 words per layer, 20 for the output pass.
- `references.json`, format 1: `made_by` holds the naga and libm versions; `expected` holds `rgba_blake3` and optionally `png` and `png_blake3`.
- `anchor-table.tsv`: columns `scene sw-nv sw-wa nv-wa alone`; each cell is `bytes/largest`, or `hash` for the 8 scenes with no picture.

## How a recipe is built
A 16-bit float composite the size of the output, cleared to the background. Each layer: 6 vertices of `layer.wgsl`, premultiplied over, with its uniforms and picture (a 1x1 white picture for a solid). Then an 8-bit output cleared to opaque black, drawn with 3 vertices of `output.wgsl`, replacing, reading the composite. Sampler `Sampler::LINEAR_CLAMP`; output target `TargetId(1)`.

## Using it
```rust
let recipes = golden::portable::Recipes::load()?;
let references = golden::portable::load_references()?;
let renderer = cpu_render::Renderer::new();
for scene in &recipes.index.scenes {
    let frame = renderer.run(&recipes.recipe(scene)?)?;
    golden::portable::check_output(references.scene(&scene.name).unwrap(), frame.width, frame.height, &frame.bytes)?;
}
```

## Commands
- `cargo run -p cpu-render --example make_portable_references -- --check|--write [--dir F] [--out F] [--threads N]`. Exit 0 means same or written, 1 means something differs, 2 means an error or no mode given. With no mode it refuses and writes nothing. `--check` writes nothing. `--write` writes only `references.json` and `images/` and removes pictures nothing refers to.
- `cargo run -p cpu-render --example anchor_table -- [--table] [--totals] [--third FOLDER]`. `--third` compares with a folder of `<scene>.png` drawn on another card, drawing the 8 large scenes so they can be compared too.

## Regenerating the capture (only if ever needed)
1. `git -c core.autocrlf=false archive stage-1-done` into a temp folder.
2. Apply `capture/old-code-changes.diff` with `git -c core.autocrlf=false apply`.
3. Copy `capture/capture_recipes.rs.txt` to `crates/render/examples/capture_recipes.rs`.
4. Run `capture_recipes --capture OUT.json` (opens no device) or `--draw FOLDER`, with its own `CARGO_TARGET_DIR`. `CAPTURE_BACKEND=dx12|gl` chooses the interface for `--draw`; `VEP_RENDER_ADAPTER=software` chooses WARP.

## For the main session
Everything is uncommitted. `crates/golden/README.md` and README-STAGES.md do not mention this folder yet; I do not own them.

## deviations

- PNG rule: the brief says a PNG up to 640x360 and hash-only above, as Stage 0 does. Stage 0's actual rule also keeps a larger picture's PNG when it is under 1 MB, so I followed Stage 0. Two 1920x1080 scenes keep a PNG (hd.smooth_plain, hd.smooth_transform_mask), giving 343 PNGs and 8 hash-only: the same scenes as Stage 0.
- crates/golden/Cargo.toml was edited (the render-recipe dependency) although it is not on the "You own" list. Deliverable 2 requires it. cargo tree -p golden still shows no editor crate.
- crates/cpu-render/src was not changed: no defect was found, so nothing there needed fixing.
- The anchor rule, as coded: per scene, the largest difference from either adapter must be at most what the adapters have between them, with one named exception; bytes from WARP must be at most the adapters' distance, with no exception; bytes from NVIDIA must be at most 14.3% above it, with 21 named scenes. An exception that goes away, or a new one, also fails the test. The 14.3% is the measured worst case (843 against 738), with the reason stated in the test.
- Beyond the brief: capture --draw also ran through DX12 and OpenGL on the RTX 3090 to see how much the graphics interface matters (CAPTURE_BACKEND in the temporary copy only, recorded in the diff).
- Beyond the brief: PictureRecord also records pattern and seed, and each scene records its sequence size, so pictures can be remade from the frozen generators and damage is detected.
- The planted-bug check ran in two batches of 4 on one temp copy and one CARGO_TARGET_DIR, so each batch stops within 10 minutes. The first start of batch 2 planted nothing: the fresh copy kept old file times, so cargo reused the previous bug's build as the "unchanged copy". I fixed the script (it now touches the copied sources) and batch 1 was unaffected.
- Beyond the plan, after the code was finished, on that same throwaway copy: about 18 further variants of pow and the filter, to test the earlier explanation, which turned out wrong. Also one planted change against the new test. The real tree was never changed for any of these; the file time and the patterns in math.rs and texture.rs were checked afterwards.
- Added a test the brief did not list: this_renderer_is_the_odd_one_out_least_often. Added Cargo.lock to the file list in the_capture_is_on_record. Both exist because the explanation and the record now rely on them.
- Long runs were not repeated because their logs exist and the code they cover has not changed since (cpu-render/src last changed 01:03, references written 01:55): seven determinism runs, WARP verify, card drawings, wasm check. I re-checked their results by comparing files instead.

## measurements

- ANCHOR, 343 scenes kept as pictures, 71,253,604 bytes, release sets. Recounted with a separate Python PNG reader sharing no code with the Rust test: all 343 rows of anchor-table.tsv and all totals match.
software vs NVIDIA RTX A4000: 739,566 bytes (1.038%), 57 scenes identical, largest 6; by 1: 739,074, by 2: 444, by 3: 32, by 4+: 16
software vs WARP: 150,318 bytes (0.211%), 257 identical, largest 193; by 1: 147,545, by 2: 2,725, by 3: 41, by 4+: 7
NVIDIA vs WARP: 812,095 bytes (1.140%), 54 identical, largest 193; by 1: 808,426, by 2: 3,559, by 3: 86, by 4+: 24
The 54 scenes the adapters draw alike: the software renderer draws all 54 alike.
The 193: on transform.rotation, WARP alone, 6 bytes on 2 pixels (248,34 and 7,109); software and NVIDIA agree exactly there.
- WHO IS THE ODD ONE OUT, over the 343 scenes: software 37,742 bytes (0.053%), WARP 110,271, NVIDIA 699,519, all three apart 2,305. These sum exactly to each distance above. By scene, software is the odd one out more often than WARP in 21, less often in 62, equally in 260, and more often than NVIDIA in 0. 268 scenes have no such bytes. Two rotated noise scenes hold 72% (hd.noise_scaled_rotated_preview_640 17,342; wide_noise.magnified_rotated 9,906). By channel r/g/b/a: 12,725 / 11,536 / 13,481 / 0. Software higher on 26,653, lower on 11,089. All but 9 are by one level (8 by 2, 1 by 3).
- EVERY SCENE FARTHER THAN THE ADAPTERS ARE FROM EACH OTHER (22).
In largest difference (1): color.sequence_contrast_saturation is 3 levels from WARP, adapters 2. Five green bytes in column 39, rows 17 to 21: software 0,3,6,12,16; NVIDIA 0,3,7,12,16; WARP 2,5,9,14,18. A hard edge in a dark area under contrast, with the filter position near a step boundary. The earlier scratch experiment moved the rounding point to 0.4 and reproduced WARP's bytes, and to 0.6 and reproduced NVIDIA's; that run is logged and not repeated.
In bytes from NVIDIA (21), which by counting are exactly the scenes where software is the odd one out more often than WARP. Each row: scene, sw-nv, nv-wa, software alone, WARP alone, NVIDIA alone, all apart.
transform.scale_up 2347 2286 61 0 2286 0
transform.track_only 843 738 212 107 631 0
transform.scale_noise 2153 2151 44 42 2109 0
transform.scale_up_noise 1876 1837 94 55 1782 0
color.sequence_exposure 1764 1707 144 87 1620 0
transition.out_next_before_start 2693 2671 83 61 2610 0
transition.in_with_track_effects_mid 1883 1879 74 70 1809 0
sources.other_aspect_decode 3001 2706 343 48 2658 0
sources.tiny_decode 2593 2527 137 71 2456 0
sources.rotated_90_stored 2534 2451 123 40 2411 0
output.height_100 1044 977 159 92 885 0
output.stretched_300x100 1836 1801 183 148 1652 1
keyframes.bezier_rotation_quarter 1091 1075 84 68 1007 0
keyframes.bezier_rotation_three_quarters 1010 1009 84 83 926 0
keyframes.fractional_times 1081 1075 97 91 984 0
wide_noise.crop_transform_color_mask 41370 41163 1035 828 40302 33
edges.off_grid_time 1952 1946 41 35 1911 0
edges.track_negative_crop_noise 2863 2843 84 64 2779 0
checker.mask_feather_anamorphic 643 638 9 4 634 0
checker.mask_feather_rotated_stored 1108 1061 52 5 1056 0
checker.mask_feather_other_decode_noise 5162 5137 101 76 5061 0
Together: software alone 3,244, WARP alone 2,075. Of the 3,244, 3,200 take the adapters' value when the composite moves by 1 or 2 of its smallest steps or the filter coordinate by a small part of a step; the other 44 are at most 13 in a scene. Worst ratio 14.2% (transform.track_only), bound 14.3%.
- NEARBY RULES TRIED ON A SCRATCH COPY: rule | from NVIDIA | from WARP | software alone | scenes farther in bytes from NVIDIA (and, where measured, in level).
as defined | 739,566 | 150,318 | 37,742 | 21, 1
weights from unsnapped vertices | 863,390 | 202,027 | 124,237 | 51, 10
filter coordinate dropping bits | 2,535,210 | 2,113,209 | 1,903,286 | 314, 315
filter coordinate in 32 bits | 741,916 | 150,079 | 38,782 | 21, 1
filter rounding point 0.45 | 734,431 | 168,381 | 44,208 | 24, 9
filter rounding point 0.49 | 735,255 | 152,191 | 36,549 | 17, 1
filter rounding point 0.51 | 745,069 | 151,871 | 41,217 | 24, 2
filter rounding point 0.55 | 768,166 | 166,456 | 59,817 | 37, 4
pow as exp2(y log2 x), 16 bytes change | 739,568 | 150,312 | 37,740 | 22, 1
pow as exp(y ln x) | 739,564 | 150,320 | 37,742 | 21, 1
pow lower by 1 in 2^22 | 744,289 | 155,349 | 42,619 | 33
pow lower by 1 in 2^20 | 741,321 | 158,236 | 42,580 | 29
pow lower by 1 in 2^18 | 727,366 | 176,644 | 44,805 | 20
pow higher by 1 in 2^20 | 745,373 | 157,682 | 44,326 | 89
filter sum in 32 bits, 14 bytes change | 739,560 | 150,320 | 37,740 | 21
filter as two 32-bit interpolations, 7 bytes change | 739,565 | 150,313 | 37,739 | 21
filter result cut to 12 fractional bits | 764,120 | 297,224 | 123,437 | 27
filter result cut to 14 fractional bits | 729,778 | 163,041 | 39,207 | 8
filter result cut to 16 fractional bits | 736,668 | 148,982 | 35,626 | 17
filter result cut to 18 fractional bits | 738,845 | 149,406 | 36,925 | 18
filter result cut to 20 fractional bits | 739,429 | 150,084 | 37,557 | 20
filter result rounded to 12 bits | 925,767 | 377,011 | 244,163 | 70
filter result rounded to 16 bits | 758,924 | 171,129 | 57,827 | 41
filter result rounded to 20 bits | 739,535 | 150,335 | 37,735 | 20
Cutting to 16 bits is the only variant closer to both adapters, by 0.4%, 0.9% and 5.6% respectively. It was not adopted.
- OLD CODE ON THIS PC. WARP DLL version 10.0.26100.9818, the version of the Stage 0 sets.
make_v2_goldens --verify with VEP_RENDER_ADAPTER=software, dev: "verified 351 scenes on microsoft-basic-render-driver-dx12-debug: all byte-identical to the references, source requests identical" (9.0 s). Release: the same on the -release set (8.6 s).
--verify on the RTX 3090 prints "skipped" and compares nothing, which is the original problem.
capture --draw hashes against the Stage 0 sets: RTX 3090 Vulkan, release and dev: 351 of 351 identical to the A4000 set (8 of 8 hash-only). RTX 3090 DX12: 340 of 351. RTX 3090 OpenGL: 321 of 351. WARP dev: 351 of 351 against the WARP dev set, 346 of 351 against the WARP release set.
- THIRD CARD, all 351 scenes, 133,000,804 bytes. Pair: bytes (percent), identical scenes, largest.
software / RTX 3090 Vulkan: 2,531,465 (1.903%), 58, 6. Re-measured now with anchor_table --third: same numbers.
software / RTX 3090 DX12: 2,531,468 (1.903%), 59, 6
software / RTX 3090 OpenGL: 2,531,378 (1.903%), 59, 6
software / WARP (dev drawing): 482,588 (0.363%), 259, 193
RTX 3090 Vulkan / DX12: 28, 340, 1
RTX 3090 Vulkan / OpenGL: 212, 321, 1
RTX 3090 DX12 / OpenGL: 186, 329, 1
RTX 3090 Vulkan / WARP: 2,742,751 (2.062%), 55, 193
RTX 3090 DX12 / WARP: 2,742,752 (2.062%), 56, 193
RTX 3090 OpenGL / WARP: 2,742,675 (2.062%), 56, 193
The rule applied to the 3090 over all 351: 22 scenes have more bytes from the 3090 than 3090-WARP (the 21 above plus hd.noise_color_mask by one byte), worst 14.2%; one scene farther in largest difference (the same one); 0 scenes farther from WARP in bytes.
- THE 8 LARGE SCENES, measured on this PC only (they cannot be compared by file). Cells are bytes/largest: software-3090, software-WARP, 3090-WARP.
hd.noise_plain: 0/0, 0/0, 0/0
hd.noise_scaled_rotated: 819,656/2, 158,650/125, 891,566/125
hd.noise_color_mask: 69,837/1, 45/1, 69,836/1
hd.noise_half_decode: 111,541/1, 11/1, 111,552/1
hd.noise_anamorphic: 3,420/1, 0/0, 3,420/1
hd.noise_dissolve_mid: 221,431/1, 79/1, 221,510/1
hd.noise_sequence_color: 194,803/1, 47/1, 194,814/1
hd.noise_rotated_output_720: 371,211/2, 173,443/142, 437,961/142
- DETERMINISM. Seven separate processes of make_portable_references --write: dev on 1, 8, 8, 32 threads (67.2, 19.8, 19.8, 13.2 s), release on 3, 16, 16 threads (29.4, 12.3, 12.3 s). I compared their folders with the project today: 0 image files differ, references.json identical in all seven. --check: dev 14.7 s, release 12.2 s, both "every byte is as stored"; re-run today, dev, 8 threads, 20 s, same result, after the profile change. Capture: two dev processes and one release process byte-identical to recipes.json. wasm-check on the old recipes: 351 recipes the same as native in 83.9 s, 23 libm hashes the same; same physical processor.
- THE DATA. 351 scenes, 617 layers (444 with a source picture), 447 source requests, 1 known panic. Folder 9,026,112 bytes (budget 30 MB), recipes.json 889,535 bytes (budget 5 MB), references.json 135,548 bytes, 343 PNGs, 8 hash-only. Old shaders: same git blobs 63e6c85f and fe44ca03 at baseline-before-blocks, stage-1-done and HEAD. Temp copy compared against all 1,632 files of the tag: only Cargo.lock and 4 render files differ, plus the one new program; applying the record reproduces those 5 exactly.
- PLANTED BUGS, on a throwaway copy. Each planted bug failed every_scene_reproduces_its_reference_byte_for_byte. After regenerating references with the bug, each failed anchor, including the_software_renderer_is_as_close_to_each_adapter_as_they_are_to_each_other.
The 8: filter coordinate drops bits; top-left rule reversed; target stores to nearest; 8-bit rounding at 0.4; pow off by a thousandth; blend's keep off by a thousandth; snapping drops bits; pixel centre moved by 1/32 pixel.
Result 8 of 8. Per-bug logs are mut-<name>-old.log and mut-<name>-anchor.log in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\old\logs. The new odd-one-out test was checked separately on one planted change (pow as exp2/log2): it fails, 37,740 against 37,742.
- FULL TABLE, verbatim from crates/golden/data/render-portable/anchor-table.tsv (each cell is bytes/largest). The test the_stored_table_is_the_measurement pins that file against the pictures.
scene sw-nv sw-wa nv-wa alone
transform.empty_node 0/0 0/0 0/0 0/0
transform.position 132/1 184/1 314/1 1/1
transform.scale 401/1 220/1 429/1 96/1
transform.scale_up 2347/1 61/1 2286/1 61/1
transform.scale_zero 0/0 0/0 0/0 0/0
transform.rotation 2469/1 521/193 2564/193 213/1
transform.rotation_90 0/0 8/1 8/1 0/0
transform.rotation_large_negative 2355/1 407/1 2378/1 192/1
transform.anchor 0/0 0/0 0/0 0/0
transform.anchor_rotation_scale 727/1 101/1 732/1 48/1
transform.anchor_outside 0/0 0/0 0/0 0/0
transform.opacity 2701/1 0/0 2701/1 0/0
transform.all 1188/1 264/1 1202/1 125/1
transform.two_nodes 4316/1 200/1 4324/1 96/1
transform.clip_and_track 2193/1 220/1 2205/1 104/1
transform.track_only 843/1 319/1 738/1 212/1
transform.scale_noise 2153/1 86/1 2151/1 44/1
transform.scale_up_noise 1876/1 149/1 1837/1 94/1
transform.rotation_noise 6848/2 5855/2 10393/2 1140/1
transform.all_noise 2958/1 1209/1 3632/2 267/1
crop.left 0/0 0/0 0/0 0/0
crop.right 0/0 0/0 0/0 0/0
crop.top 0/0 0/0 0/0 0/0
crop.bottom 0/0 0/0 0/0 0/0
crop.all_edges 0/0 140/1 140/1 0/0
crop.left_right_overlap 0/0 0/0 0/0 0/0
crop.top_bottom_overlap 0/0 0/0 0/0 0/0
crop.feather_is_inert 0/0 0/0 0/0 0/0
crop.empty_node 0/0 0/0 0/0 0/0
crop.two_nodes 0/0 0/0 0/0 0/0
crop.clip_and_track 0/0 318/1 318/1 0/0
crop.with_transform 846/1 151/1 855/1 71/1
crop.all_edges_noise 0/0 0/0 0/0 0/0
crop.with_transform_noise 2460/2 1087/2 3069/2 236/1
blend.composite_opacity 1833/1 0/0 1833/1 0/0
blend.composite_times_transform 1587/1 0/0 1587/1 0/0
blend.composite_clip_and_track 1975/1 0/0 1975/1 0/0
blend.composite_empty_node 72/1 0/0 72/1 0/0
blend.param_normal 1386/1 0/0 1386/1 0/0
blend.param_add 1386/1 0/0 1386/1 0/0
blend.param_multiply 1386/1 0/0 1386/1 0/0
blend.param_screen 1386/1 0/0 1386/1 0/0
blend.param_overlay 1386/1 0/0 1386/1 0/0
blend.param_soft_light 1386/1 0/0 1386/1 0/0
blend.param_hard_light 1386/1 0/0 1386/1 0/0
blend.param_darken 1386/1 0/0 1386/1 0/0
blend.param_lighten 1386/1 0/0 1386/1 0/0
blend.param_difference 1386/1 0/0 1386/1 0/0
blend.node_normal 1386/1 0/0 1386/1 0/0
blend.node_add 1386/1 0/0 1386/1 0/0
blend.node_multiply 1386/1 0/0 1386/1 0/0
blend.node_screen 1386/1 0/0 1386/1 0/0
blend.node_overlay 1386/1 0/0 1386/1 0/0
blend.node_soft_light 1386/1 0/0 1386/1 0/0
blend.node_hard_light 1386/1 0/0 1386/1 0/0
blend.node_darken 1386/1 0/0 1386/1 0/0
blend.node_lighten 1386/1 0/0 1386/1 0/0
blend.node_difference 1386/1 0/0 1386/1 0/0
blend.track_node_blend 2032/1 0/0 2032/1 0/0
blend.node_mask_is_inert 3067/1 0/0 3067/1 0/0
color.exposure_up 3827/1 0/0 3827/1 0/0
color.exposure_down 3412/1 0/0 3412/1 0/0
color.contrast_up 995/1 0/0 995/1 0/0
color.contrast_down 2402/1 0/0 2402/1 0/0
color.saturation_zero 3120/1 0/0 3120/1 0/0
color.saturation_up 1812/1 1/1 1813/1 0/0
color.temperature_warm 4433/1 0/0 4433/1 0/0
color.temperature_cool 1990/1 0/0 1990/1 0/0
color.tint_magenta 6038/1 0/0 6038/1 0/0
color.tint_green 592/1 0/0 592/1 0/0
color.lift 4966/1 0/0 4966/1 0/0
color.gamma 7954/1 0/0 7954/1 0/0
color.gain 2713/1 0/0 2713/1 0/0
color.empty_node 144/1 0/0 144/1 0/0
color.exposure_contrast_saturation 2500/1 0/0 2500/1 0/0
color.all_parameters 5818/1 0/0 5818/1 0/0
color.beyond_white 461/1 0/0 461/1 0/0
color.two_nodes 2457/1 0/0 2457/1 0/0
color.clip_and_track 1549/1 0/0 1549/1 0/0
color.sequence_exposure 1764/1 231/1 1707/1 144/1
color.sequence_contrast_saturation 1681/1 168/3 1760/2 44/1
color.sequence_all_parameters 1862/1 254/2 1968/2 74/1
color.sequence_two_nodes 15040/1 235/2 15135/2 70/1
color.sequence_disabled_node 5437/1 0/0 5437/1 0/0
color.sequence_keyframed 3450/1 0/0 3450/1 0/0
color.composite_keeps_values_above_one 144/1 0/0 144/1 0/0
color.all_parameters_noise 1007/1 2/1 1009/1 0/0
mask.rect 0/0 0/0 0/0 0/0
mask.rect_feather 202/1 0/0 202/1 0/0
mask.ellipse 4/1 0/0 4/1 0/0
mask.ellipse_feather 412/1 0/0 412/1 0/0
mask.path 0/0 0/0 0/0 0/0
mask.path_open 237/1 0/0 237/1 0/0
mask.path_bounding_box_seed 72/1 0/0 72/1 0/0
mask.corner_radius_is_inert 0/0 0/0 0/0 0/0
mask.invert 0/0 0/0 0/0 0/0
mask.opacity 951/1 0/0 951/1 0/0
mask.invert_feather_opacity 920/1 0/0 920/1 0/0
mask.default_shape 110/1 0/0 110/1 0/0
mask.ellipse_zero_radius 0/0 0/0 0/0 0/0
mask.clip_and_track 99/1 0/0 99/1 0/0
mask.track_shape_not_a_shape 116/1 0/0 116/1 0/0
mask.on_transformed_layer 849/1 124/1 849/1 62/1
mask.with_crop 202/1 86/1 268/1 10/1
mask.keyframed_shape_holds 0/0 0/0 0/0 0/0
mask.keyframed_feather 250/1 0/0 250/1 0/0
mask.rect_feather_noise 185/1 0/0 185/1 0/0
mask.ellipse_feather_noise 236/1 0/0 236/1 0/0
transition.in_cross_dissolve_start 144/1 0/0 144/1 0/0
transition.in_cross_dissolve_mid 1400/1 0/0 1400/1 0/0
transition.in_cross_dissolve_last 3010/1 0/0 3010/1 0/0
transition.in_cross_dissolve_after 144/1 0/0 144/1 0/0
transition.in_dip_to_black_start 144/1 0/0 144/1 0/0
transition.in_dip_to_black_mid 1400/1 0/0 1400/1 0/0
transition.in_dip_to_black_last 3010/1 0/0 3010/1 0/0
transition.in_dip_to_white_start 144/1 0/0 144/1 0/0
transition.in_dip_to_white_mid 1400/1 0/0 1400/1 0/0
transition.in_dip_to_white_last 3010/1 0/0 3010/1 0/0
transition.in_wipe_start 144/1 0/0 144/1 0/0
transition.in_wipe_mid 1400/1 0/0 1400/1 0/0
transition.in_wipe_last 3010/1 0/0 3010/1 0/0
transition.in_slide_start 144/1 0/0 144/1 0/0
transition.in_slide_mid 1400/1 0/0 1400/1 0/0
transition.in_slide_last 3010/1 0/0 3010/1 0/0
transition.in_zoom_start 144/1 0/0 144/1 0/0
transition.in_zoom_mid 1400/1 0/0 1400/1 0/0
transition.in_zoom_last 3010/1 0/0 3010/1 0/0
transition.in_custom_start 144/1 0/0 144/1 0/0
transition.in_custom_mid 1400/1 0/0 1400/1 0/0
transition.in_custom_last 3010/1 0/0 3010/1 0/0
transition.out_cross_dissolve_start 144/1 0/0 144/1 0/0
transition.out_cross_dissolve_mid 1410/1 0/0 1410/1 0/0
transition.out_cross_dissolve_last 2760/1 0/0 2760/1 0/0
transition.out_cross_dissolve_before 144/1 0/0 144/1 0/0
transition.out_dip_to_black_start 144/1 0/0 144/1 0/0
transition.out_dip_to_black_mid 1410/1 0/0 1410/1 0/0
transition.out_dip_to_black_last 2760/1 0/0 2760/1 0/0
transition.out_dip_to_white_start 144/1 0/0 144/1 0/0
transition.out_dip_to_white_mid 1410/1 0/0 1410/1 0/0
transition.out_dip_to_white_last 2760/1 0/0 2760/1 0/0
transition.out_wipe_start 144/1 0/0 144/1 0/0
transition.out_wipe_mid 1410/1 0/0 1410/1 0/0
transition.out_wipe_last 2760/1 0/0 2760/1 0/0
transition.out_slide_start 144/1 0/0 144/1 0/0
transition.out_slide_mid 1410/1 0/0 1410/1 0/0
transition.out_slide_last 2760/1 0/0 2760/1 0/0
transition.out_zoom_start 144/1 0/0 144/1 0/0
transition.out_zoom_mid 1410/1 0/0 1410/1 0/0
transition.out_zoom_last 2760/1 0/0 2760/1 0/0
transition.out_custom_start 144/1 0/0 144/1 0/0
transition.out_custom_mid 1410/1 0/0 1410/1 0/0
transition.out_custom_last 2760/1 0/0 2760/1 0/0
transition.alone_cross_dissolve_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_cross_dissolve_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.alone_dip_to_black_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_dip_to_black_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.alone_dip_to_white_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_dip_to_white_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.alone_wipe_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_wipe_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.alone_slide_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_slide_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.alone_zoom_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_zoom_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.alone_custom_fade_out_mid 1625/1 0/0 1625/1 0/0
transition.alone_custom_fade_in_mid 1964/1 0/0 1964/1 0/0
transition.in_alone_mid 1964/1 0/0 1964/1 0/0
transition.in_alone_start 0/0 0/0 0/0 0/0
transition.in_alone_last 2557/1 0/0 2557/1 0/0
transition.in_as_long_as_the_clip 1155/1 0/0 1155/1 0/0
transition.out_alone_mid 1645/1 0/0 1645/1 0/0
transition.out_alone_last 2076/1 0/0 2076/1 0/0
transition.in_after_one_frame_gap 2819/1 0/0 2819/1 0/0
transition.in_prev_disabled 2819/1 0/0 2819/1 0/0
transition.out_next_disabled 3729/1 0/0 3729/1 0/0
transition.conflict_before_cut 144/1 0/0 144/1 0/0
transition.conflict_after_cut 1400/1 0/0 1400/1 0/0
transition.in_neighbour_keyframes_hold 3249/1 54/1 3287/1 8/1
transition.out_next_before_start 2693/1 144/1 2671/1 83/1
transition.in_over_backdrop_mid 3199/1 0/0 3199/1 0/0
transition.in_solid_to_picture_mid 7102/1 0/0 7102/1 0/0
transition.in_offline_neighbour_mid 2819/1 0/0 2819/1 0/0
transition.in_with_track_effects_mid 1883/1 144/1 1879/1 74/1
transition.in_reversed_neighbour_mid 1594/1 0/0 1594/1 0/0
transition.in_noise_mid 2189/1 0/0 2189/1 0/0
transition.in_noise_last 2487/1 0/0 2487/1 0/0
transition.out_noise_mid 2113/1 0/0 2113/1 0/0
generators.solid_rgb 0/0 0/0 0/0 0/0
generators.solid_rgba 4591/1 0/0 4591/1 0/0
generators.solid_without_metadata 0/0 0/0 0/0 0/0
generators.solid_clamped 0/0 0/0 0/0 0/0
generators.solid_two_components 0/0 0/0 0/0 0/0
generators.solid_not_an_array 0/0 0/0 0/0 0/0
generators.solid_non_number_shifts 0/0 0/0 0/0 0/0
generators.solid_set_by_command 0/0 0/0 0/0 0/0
generators.solid_transformed 144/1 0/0 144/1 0/0
generators.solid_masked 382/1 0/0 382/1 0/0
generators.solid_graded 0/0 0/0 0/0 0/0
generators.solid_track_effects 0/0 0/0 0/0 0/0
generators.color_bars 0/0 0/0 0/0 0/0
generators.color_bars_transformed 108/1 0/0 108/1 0/0
generators.text 144/1 0/0 144/1 0/0
generators.adjustment 144/1 0/0 144/1 0/0
unimplemented.blur 0/0 0/0 0/0 0/0
unimplemented.curves 0/0 0/0 0/0 0/0
unimplemented.lut 0/0 0/0 0/0 0/0
unimplemented.text_effect 0/0 0/0 0/0 0/0
unimplemented.shader 0/0 0/0 0/0 0/0
unimplemented.mixed_with_working_nodes 1632/1 140/1 1632/1 70/1
unimplemented.sequence_level_kinds 0/0 0/0 0/0 0/0
unimplemented.time_remap_frame_blend 0/0 0/0 0/0 0/0
stack.three_layers 2063/1 139/1 2084/1 59/1
stack.six_layers 1384/1 234/1 1404/1 107/1
stack.disabled_clip 144/1 0/0 144/1 0/0
stack.disabled_transform 2000/1 0/0 2000/1 0/0
stack.disabled_crop 2000/1 0/0 2000/1 0/0
stack.disabled_color 2000/1 0/0 2000/1 0/0
stack.disabled_mask 2000/1 0/0 2000/1 0/0
stack.disabled_composite 2000/1 0/0 2000/1 0/0
stack.disabled_track_node 787/1 181/1 936/1 16/1
stack.mute_top 144/1 0/0 144/1 0/0
stack.solo_bottom 144/1 0/0 144/1 0/0
stack.solo_both 161/1 66/1 223/1 2/1
stack.mute_soloed_track 144/1 0/0 144/1 0/0
stack.audio_track_solo_is_ignored 157/1 43/1 198/1 1/1
stack.offline_top 144/1 0/0 144/1 0/0
stack.offline_only 0/0 0/0 0/0 0/0
stack.cut_exact_boundary 144/1 0/0 144/1 0/0
stack.gap 0/0 0/0 0/0 0/0
stack.after_the_end 0/0 0/0 0/0 0/0
stack.empty_sequence 0/0 0/0 0/0 0/0
stack.track_transform 1075/1 176/1 1097/1 77/1
stack.track_color 2652/1 0/0 2652/1 0/0
stack.track_crop 0/0 0/0 0/0 0/0
stack.track_mask 487/1 0/0 487/1 0/0
stack.track_composite 1690/1 0/0 1690/1 0/0
stack.track_keyframes_clip_local 3982/1 0/0 3982/1 0/0
stack.track_values_out_of_range 0/0 0/0 0/0 0/0
stack.same_asset_twice 928/1 147/1 931/1 72/1
sources.bt709 144/1 0/0 144/1 0/0
sources.linear 1935/1 0/0 1935/1 0/0
sources.pq 144/1 0/0 144/1 0/0
sources.unknown_transfer 144/1 0/0 144/1 0/0
sources.half_decode 715/1 0/0 715/1 0/0
sources.other_aspect_decode 3001/1 391/1 2706/1 343/1
sources.tiny_decode 2593/1 208/1 2527/1 137/1
sources.anamorphic_hdv 71/1 0/0 71/1 0/0
sources.anamorphic_dv 1049/1 4/1 1053/1 0/0
sources.rotated_90_turned 144/1 0/0 144/1 0/0
sources.rotated_90_stored 2534/1 163/1 2451/1 123/1
sources.rotated_270_turned 144/1 0/0 144/1 0/0
sources.rotated_180 144/1 0/0 144/1 0/0
sources.anamorphic_and_rotated 116/1 0/0 116/1 0/0
sources.stream_without_size 30/1 0/0 30/1 0/0
sources.still_image 144/1 0/0 144/1 0/0
sources.larger_than_frame 144/1 0/0 144/1 0/0
sources.half_decode_noise 1094/1 0/0 1094/1 0/0
sources.anamorphic_noise 35/1 0/0 35/1 0/0
sources.linear_noise 1645/1 0/0 1645/1 0/0
sources.larger_scaled_down 72/1 0/0 72/1 0/0
sources.larger_scaled_down_noise 0/0 0/0 0/0 0/0
sources.three_to_one_noise 0/0 0/0 0/0 0/0
sources.source_in_offset 144/1 0/0 144/1 0/0
output.native 2569/1 184/1 2585/1 84/1
output.width_200 1964/1 195/2 1988/2 85/1
output.height_100 1044/1 251/1 977/1 159/1
output.stretched_300x100 1836/1 332/1 1801/2 183/1
output.odd_131x77 811/1 160/1 905/1 33/1
output.double_512x288 10940/1 521/1 10955/1 253/1
output.at_640x360 18938/1 2511/2 19057/2 1195/1
output.minimum_1x1 1/1 0/0 1/1 0/0
output.background_colour 2604/1 181/1 2617/1 84/1
output.background_translucent 2604/1 175/1 2615/1 82/1
output.background_out_of_range 2663/1 180/1 2667/1 88/1
output.background_only 0/0 0/0 0/0 0/0
output.translucent_layer_over_background 0/0 0/0 0/0 0/0
output.odd_sequence_native 451/1 83/1 466/1 34/1
output.noise_native 3378/1 868/2 3870/3 185/1
output.noise_width_100 2092/2 271/2 2184/2 89/1
output.noise_odd_131x77 4188/2 430/2 4333/2 142/1
keyframes.linear_position_at_first 13/1 90/1 97/1 3/1
keyframes.linear_position_quarter 44/1 38/1 74/1 4/1
keyframes.linear_position_half 15/1 85/1 96/1 2/1
keyframes.linear_position_at_last 14/1 32/1 46/1 0/0
keyframes.linear_position_after_last 13/1 40/1 47/1 3/1
keyframes.hold_opacity_before 5482/1 0/0 5482/1 0/0
keyframes.hold_opacity_at_next 3043/1 0/0 3043/1 0/0
keyframes.bezier_rotation_quarter 1091/1 152/1 1075/1 84/1
keyframes.bezier_rotation_half 656/1 166/1 684/1 69/1
keyframes.bezier_rotation_three_quarters 1010/1 167/1 1009/1 84/1
keyframes.bezier_overshoot_early 392/1 86/1 414/1 32/1
keyframes.bezier_overshoot_late 2910/1 210/1 2946/1 87/1
keyframes.mixed_hold_segment 18/1 0/0 18/1 0/0
keyframes.mixed_linear_segment 18/1 0/0 18/1 0/0
keyframes.mixed_bezier_segment 66/1 2/1 66/1 1/1
keyframes.mixed_hold_after_bezier 18/1 0/0 18/1 0/0
keyframes.fractional_times 1081/1 188/1 1075/1 97/1
keyframes.colour_and_crop 2158/1 90/1 2234/1 7/1
time.ntsc_native 144/1 0/0 144/1 0/0
time.ntsc_24fps_source 144/1 0/0 144/1 0/0
time.ntsc_25fps_source 144/1 0/0 144/1 0/0
time.ntsc_transition 4269/1 0/0 4269/1 0/0
time.ntsc_keyframes 8/1 0/0 8/1 0/0
time.speed_half 144/1 0/0 144/1 0/0
time.speed_double 144/1 0/0 144/1 0/0
time.speed_third 144/1 0/0 144/1 0/0
time.reverse 144/1 0/0 144/1 0/0
time.reverse_slow 144/1 0/0 144/1 0/0
time.remap_disabled 144/1 0/0 144/1 0/0
time.source_in_with_speed 144/1 0/0 144/1 0/0
time.speed_ramp_linear 144/1 0/0 144/1 0/0
time.speed_ramp_bezier 144/1 0/0 144/1 0/0
time.reverse_slow_noise 0/0 0/0 0/0 0/0
time.fifty_fps_source_slowed 0/0 0/0 0/0 0/0
hd.noise_plain hash hash hash hash
hd.noise_scaled_rotated hash hash hash hash
hd.noise_scaled_rotated_preview_640 91904/2 84366/3 139419/4 17342/2
hd.noise_color_mask hash hash hash hash
hd.noise_color_mask_preview_640 7738/1 5/1 7739/1 2/1
hd.noise_half_decode hash hash hash hash
hd.noise_anamorphic hash hash hash hash
hd.noise_dissolve_mid hash hash hash hash
hd.noise_sequence_color hash hash hash hash
hd.noise_rotated_output_720 hash hash hash hash
hd.smooth_plain 7560/1 0/0 7560/1 0/0
hd.smooth_transform_mask 22188/1 2549/2 22230/2 1250/1
wide_noise.crop_transform_color_mask 41370/6 1896/3 41163/6 1035/3
wide_noise.dissolve_mid 24603/1 3/1 24606/1 0/0
wide_noise.plain 0/0 0/0 0/0 0/0
wide_noise.magnified_rotated 72474/2 38050/2 90665/2 9906/1
wide_noise.hd_half_decode_scaled_down 12552/1 1/1 12553/1 0/0
edges.in_and_out_on_one_clip_incoming_wins 1093/1 0/0 1093/1 0/0
edges.in_and_out_on_one_clip_outgoing_after 1866/1 0/0 1866/1 0/0
edges.dissolve_into_text_generator_mid 0/0 0/0 0/0 0/0
edges.transition_under_a_layer_mid 2034/1 0/0 2034/1 0/0
edges.off_grid_time 1952/1 76/1 1946/1 41/1
edges.track_mirror 0/0 0/0 0/0 0/0
edges.track_negative_scale_rotated 1371/1 236/2 1385/2 111/1
edges.track_negative_crop 17/1 39/1 56/1 0/0
edges.track_negative_crop_noise 2863/1 148/1 2843/1 84/1
edges.track_wrong_types_and_unknown_parameters 1018/1 33/1 1035/1 8/1
edges.track_integer_values 586/1 145/4 618/4 56/1
checker.mask_feather_half_decode 320/1 1/1 321/1 0/0
checker.mask_feather_anamorphic 643/1 13/1 638/1 9/1
checker.mask_feather_rotated_stored 1108/1 57/1 1061/1 52/1
checker.mask_feather_half_decode_noise 861/1 0/0 861/1 0/0
checker.mask_feather_other_decode_noise 5162/1 177/1 5137/1 101/1
checker.track_gamma_below_the_clamp 372/1 0/0 372/1 0/0
checker.track_mask_negative_feather_opacity_above_one 1017/1 0/0 1017/1 0/0
checker.solid_whole_numbers 0/0 0/0 0/0 0/0
checker.solid_five_numbers 2568/1 0/0 2568/1 0/0

## known_gaps

- Mac, Linux and ARM have not been run; WSL has no distribution here. The promise for them rests on crates/cpu-render/README.md, on IEEE 754 being exact and on the pinned hashes. The wasm-check is the nearest substitute: all 351 old recipes came out the same as native, but it is the same physical processor, so it says nothing about how ARM rounds. The real check is cargo test -p cpu-render -p golden on such a machine.
- The anchor has only two adapter families behind it, NVIDIA and WARP. The out-of-sample card is weak evidence, as it is another NVIDIA card with identical bytes. No AMD, Intel or Apple graphics has been compared.
- No byte-level cause was found for why this renderer is the odd one out on 37,742 bytes. What is established: pow is not it, the order or width of the filter's sum is not it, nothing tried removes them, three renderers near a boundary will produce an odd one out, and this renderer is the odd one out least often. The accepted explanation for the 21 scenes is an exact counting statement about who is the odd one out, not a proven physical mechanism. No defect was found, which is not proof that none exists.
- Possible lead, not adopted: cutting the filter's result to 16 fractional bits comes slightly closer to both adapters (bytes from NVIDIA -0.4%, from WARP -0.9%, software alone -5.6%). It removes few bytes and proves nothing about what the adapters do. Adopting it would redefine the arithmetic, and would change the references and the pinned hashes in tests of crates/cpu-render that I did not touch.
- The 8 large scenes keep only a hash in Stage 0, so anchor.rs cannot compare them on any computer. They were compared once by hand on this PC, stored nowhere; the README has the numbers.
- The whole workspace test suite was not run, only golden, cpu-render and render-recipe. cargo test --release of these tests was not run in this session. Release results are the logged make_portable_references --write and --check runs, from before today's edits; since then only comments, one type alias in the example and test code have changed.
- The planted-bug check ran before the new odd-one-out test existed. I checked that test separately on one planted change (it fails). Eight planted bugs show the two tests notice broad defects; they say nothing about defects of a kind not planted.
- crates/golden/README.md does not mention this folder and README-STAGES.md does not record it. I do not own them, so the main session should update them when it commits.
- Older clippy findings remain in crates I do not own or in older golden files (golden/src/sound.rs, golden/tests/picture_references.rs, core-*). There are none in crates/cpu-render, crates/render-recipe, crates/golden/src/portable.rs or crates/golden/tests/portable_references.rs.
- The old code's build folder and the throwaway copy were deleted to keep temp small (Stage 0 of a repeat run would rebuild). Kept in C:\Users\const\AppData\Local\Temp\claude\D--chadfx\fa88a9d6-2afc-40b2-bfee-2f22a921f56a\scratchpad\old: src (the old code with the changes), drawn, runs, capture, logs, mutate.py, recheck. Re-running deliverable 6 means rebuilding the old code first.

## checks

Run from D:\chadfx under a hard time limit, after the last change unless noted.

| Command | Result |
|---|---|
| `cargo test --offline -j 16 -p golden -p cpu-render -p render-recipe --no-fail-fast` | exit 0: 314 passed, 0 failed, 1 ignored (old sound.rs doc example), 0 warnings; includes portable_references 14, old_recipes 12, anchor 9, dependencies 4; 25.5 s |
| `cargo run -p cpu-render --example make_portable_references -- --check --threads 8` | exit 0, "compared 351 of 351 scenes", "every byte is as stored", 20 s; run after the last edit to the program |
| same example with no argument | exit 2, "nothing is written without --write"; file names, sizes and times in the folder unchanged after this and after --check (checked on the earlier run, before the alias edit) |
| `cargo clippy --offline -p golden -p cpu-render -p render-recipe --all-targets` | no findings in cpu-render, render-recipe or golden's portable files; older findings elsewhere remain |
| `cargo run -p cpu-render --example anchor_table -- --totals --third <old/drawn/default-release>` | exit 0; software-third 2,531,465 of 133,000,804, NVIDIA-third 0 of 71,253,604; run before the last small edits, the program did not change |
| `cargo tree --offline -p golden -e normal --prefix none` | no editor crate; render-recipe is the only workspace crate |
| `git diff --stat HEAD` on crates/golden/data/render and data/audio, plus picture.rs and sound.rs | no change |

**Also checked this session, each once:**
- Separate Python recount: all 343 rows and all totals match (102 s).
- A throwaway scratch tool comparing each drawn folder's hashes with the Stage 0 sets: 3090 Vulkan 351 of 351 against the A4000, WARP dev 351 of 351.
- old-code-changes.diff applied to the tag's files with git's line-ending conversion off: 5 of 5 identical to the temp copy; all 1,632 of the tag's files compared with the temp copy.
- old/mutate.py in two batches: 8 of 8 caught, unchanged copy passes first.

**Files from the earlier attempt that I rely on and did not re-run, because the code they cover has not changed:** verify-warp-dev.log and verify-warp-rel.log, verify-3090-dev.log, draw-*.log, matrix.txt and matrix.tsv, explain*.tsv, frac.tsv, var-*.tsv, det-*.log, wasm.log, capture-*.log.

**Not run:** Mac, Linux, ARM; `cargo test --workspace`; `cargo test --release`.

**Files changed this session:**
- crates/golden/src/portable.rs (doc example only)
- crates/golden/tests/portable_references.rs (one list)
- crates/golden/data/render-portable/README.md
- crates/golden/data/render-portable/capture/old-code-changes.diff
- crates/cpu-render/tests/anchor.rs
- crates/cpu-render/examples/make_portable_references.rs (a type alias only)

No reference picture or recipe changed. Git state was not touched. Free space on C: was 344 GB after cleanup.
