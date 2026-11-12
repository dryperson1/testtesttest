# ChadFX plug-in blocks rewrite: what is done and what to do next

This file is for a future Claude Code session, on this computer or another, with no memory of the sessions that built this. Read it fully before touching anything. [README-PROJECT.md](README-PROJECT.md) explains how to interpret the project as a whole.

**Status on 2026-11-06.** **The plug-in blocks rewrite is finished**: Stages 0 to 6 are done and verified (tag `stage-6-done`). The plain-English final report is [docs/blocks-rewrite/FINAL-REPORT.md](docs/blocks-rewrite/FINAL-REPORT.md), and its "Decisions only you can make" list is waiting for the owner.
- The renderer draws every picture block from its manifest and shader, and the mixer plays every sound block from its WebAssembly module, with no block named in either.
- Old pictures are byte for byte the same, and old sound the same within the rounding statement (spec 23).
- The app has:
  - one registry with live refresh;
  - a block manager;
  - an inspector built from manifests, for a clip's, a track's and the sequence's effects;
  - a sound thread of its own;
  - the `.chadfx-block` file type with double-click install, and a per-user Windows installer;
  - blocks in one `.wgsl` file (spec 24 to 26).
- Broken and hostile blocks of every kind are contained at every layer, up to the running window.
- Outsiders have the guide [docs/BLOCKS.md](docs/BLOCKS.md) and five working templates in `blocks/template`.
- Everything was proven on Windows, on the RTX A4000 computer, only. The no-GPU picture proof has never run on a Mac, Linux or an ARM processor (spec 20).

This guide was first written quickly by the main session from its own knowledge of the work, and a planned fact-check by a separate team was stopped. **Treat its file and function names as strong pointers, not proof: confirm them in the code.** `docs/blocks-rewrite/BLOCKS-SPEC.md` is the binding detail.

## 1. Do this next

1. Read, in this order: this file; `docs/blocks-rewrite/BRIEF.md` (the owner's words, binding); `docs/blocks-rewrite/DESIGN-APPROVED.md`; `docs/blocks-rewrite/BLOCKS-SPEC.md`, all 26 sections. Sections 17 to 26 are later changes that override earlier text. Then `docs/blocks-rewrite/FINAL-REPORT.md`.
2. If this is a new computer, do section 2 first.
3. Health check. Run `cargo build --workspace --all-targets` (expect zero warnings), then `cargo test --workspace --no-fail-fast`. Expect the totals in section 2, in about 25 minutes; see section 2 for which results differ on other hardware. In `ui`: `npm ci`, `npm run build`, `npm run lint`, `npm test`.
4. The rewrite is done (section 6). Ask the owner about the decisions in `docs/blocks-rewrite/FINAL-REPORT.md` ("Decisions only you can make") that are still open, and do only what they answer. Changing the fuel price, the EQ's defaults or anything else in a block's contract needs the same proofs as a stage: the picture and sound proofs unchanged, unless the owner chose a change that moves them, and then only that.
5. Then the work after the rewrite (section 8): text and captions (D3) next. Write it as a stage with its own script, reports and acceptance check, the way sections 5 and 6 describe, and push only if the owner asks.

**If the owner just says "continue", this list is what they mean.**

## 2. Setting up on another computer

The repository is the private `constantinemunoz/chadfx` on GitHub. Since 2026-11-06 GitHub reports that it has moved to `constantinemunoz/cfxcorewindows`; the old address still forwards, and the owner has not yet said whether to switch the remote. Clone it signed in as that account. If the computer also has another GitHub sign-in, name the account in the address, as the owner's computer does: `https://constantinemunoz@github.com/constantinemunoz/chadfx.git`. The `gh` tool was not installed on the owner's computer; git's own credential manager did the sign-in. In the clone, set a commit identity for this repository only: name `Constantine Munoz`, email `214721139+constantinemunoz@users.noreply.github.com`.

**Line endings matter.** `.gitattributes` holds `* -text`, so git never converts anything. Do not remove it. Without it, Git for Windows converts text on checkout, and the byte-exact tests correctly fail. That really happened: see "Clean clone" below.

Needed, all free:
- Rust stable, 1.88 or newer (`rust-version` in `Cargo.toml`; the app, `vep-app`, needs 1.89 for its file lock); the owner's computer has 1.99.0. Windows also needs the Microsoft C++ Build Tools.
- Only to build the Windows installer: Tauri downloads the NSIS tools into its own cache on the first `npx tauri build` (the owner allowed this on 2026-11-05; `ui/src-tauri/tauri.windows.conf.json` limits the build to NSIS, so WiX is never fetched).
- Node.js and npm. On the owner's computer Node is a **portable copy** at `%LOCALAPPDATA%\Programs\node\current` that must be put on PATH first.
- FFmpeg. On the owner's computer it is the LGPL shared build from winget (`BtbN.FFmpeg.LGPL.Shared`), found by `media::Tools::locate` in the WinGet packages folder rather than on PATH.
- WebView2, for the app on Windows.
- Only to rebuild the sound blocks: the `wasm32v1-none` target and a Rust toolchain reporting 1.99.0. `blocks/builtin/*/block.wasm` are committed, and the staleness test skips the rebuild half and only checks the recipe hash when the toolchain is absent.

**What differs on other hardware:**
- Golden pictures (`crates/render/tests/golden_pictures.rs`, `golden_pictures_software.rs`): references exist for an NVIDIA RTX A4000 on Vulkan and for Microsoft WARP on DX12, each from debug and release builds. On a computer whose card has no stored set the test **prints why and compares nothing**. It passes, and that pass proves nothing, so read the output. On a computer whose card does have a set, comparing nothing is a failure (`golden_must_compare.rs`). Picture rendering work is proven on any machine by `no_gpu_pictures.rs`, with no graphics card at all, and on the machine's own card by `same_card.rs` (spec 20). **Never regenerate Stage 0 data to make a test pass.**
- Golden sounds: the references record their platform, and today's pan values depend on Windows maths rounding. On another operating system they have not been tried, so a sound mismatch there is not automatically a defect.
- End-to-end tests (`ui/e2e`) drive the real app through WebView2's debugging port, so as written they need Windows. They open app windows and take about 15 minutes.
- Mac and Linux are product targets but have not been exercised here. On one, first run `cargo test -p cpu-render -p golden`, then `no_gpu_pictures` in `render` (`no_gpu_recipes` was retired in Stage 3): they are the measurement the picture proof's promise is still waiting for.
- Hardware video encoding: on the old machine the NVIDIA driver was too old for its FFmpeg's NVENC, so Intel Quick Sync and software H.264 were used. On this PC (RTX 3090, full FFmpeg 8.1.2 build, on PATH) NVENC works, and it refuses frames below 146x50 for H.264 and 130x34 for HEVC, so the product now refuses those sizes in words or moves to the next encoder.

**The owner has two computers and syncs them through GitHub. Check which one you are on;** anything here or in a workflow script that says "this PC" may mean the other one.

*The RTX 3090 computer.* Windows 11, Threadripper 3970X, 128 GB, RTX 3090, Rust 1.99.0 (not on PATH: `%USERPROFILE%\.cargo\bin`), Gyan FFmpeg 8.1.2 full build on PATH. The repository is at `D:\chadfx`, moved out of OneDrive at the owner's choice. WARP reproduces all 351 Stage 0 scenes and the known panic byte for byte; the RTX 3090 has no stored reference set, so `golden_pictures` compares nothing on it. WSL has no Linux installed.

*The RTX A4000 computer.* Windows 11, i9-13900, 128 GB, RTX A4000, cargo on PATH, Node as the portable copy, LGPL FFmpeg from winget. The repository is at `C:\Users\Mauro\Documents\CLAUDESTART`. It has a second, school GitHub sign-in as its default, which is why its remote names the account. This is the only computer where `golden_pictures` compares for real. Measured here on 2026-11-04, at tag `stage-4-done`: build with zero warnings; `cargo test --workspace` 2,699 passed, 0 failed, 25 ignored in about 19 minutes; in `ui`, build, lint and test (576 of 576) passed and `npm run e2e` passed 72 of 72 in about 17 minutes; `golden_pictures` found all 351 scenes and the known panic byte-identical to `nvidia-rtx-a4000-vulkan-debug` (and, at `stage-3-done`, to `-release`), and WARP to its own sets; `no_gpu_pictures`, `same_card` (both ways) and `card_vs_cpu` matched; `golden_sounds` read 118 cases and 590 references, and under `Rule::GenericBlocks` 289 of 344 were bit-identical and the rest within the statement of spec 23, with every case the same however a range is cut. At `stage-5-done` (2026-11-06): build with zero warnings; `cargo test --workspace` 2,843 passed, 0 failed, 28 ignored in 244 test programs, in about 22 minutes; in `ui`, build, lint and test (726 of 726) passed and `npm run e2e` passed 85 tests, with 2 that run only on request, in about 18 minutes; `golden_sounds` printed exactly what it printed at `stage-4-done`, and the picture proofs were unchanged. At `stage-6-done` (2026-11-06):
- build with zero warnings;
- `cargo test --workspace`: 2,952 passed, 0 failed, 35 ignored in 264 test programs, in about 29 minutes;
- in `ui`: build and lint passed, and test 763 of 763;
- `npm run e2e`: 97 tests passed, with 2 that run only on request, in about 20 minutes;
- `golden_sounds` printed exactly what it printed at `stage-4-done`;
- every picture proof was unchanged: 351 scenes and the known panic byte for byte on the A4000 and WARP and with no card, and `card_vs_cpu` 1.901% of bytes differing, the largest by 6.

The 35 ignored tests run only on purpose. Each says why:
- measurements and timings;
- debugging aids;
- helpers that other tests start in a process of their own, or that write files when asked;
- two that show a rule a graphics card does not follow, which other tests pin;
- one open design question: a ripple trim of a split edit can part another pair that lies between the two edges.

On both: always run tests with a hard time limit, and check that a log is still growing before calling a run "still running". The owner stops work abruptly and uses the computer meanwhile, so leave nothing running when told to stop, and never plant a deliberate bug in the real source tree: use a temp copy with its own build folder.

**Clean clone, 2026-10-30.** A from-scratch clone of commit `bab9706` built with zero warnings in about 3 minutes; `cargo test --workspace` took about 10 minutes: 1895 passed, 4 failed, 16 ignored. `npm ci`, build, lint and test all passed. The four failures were `every_golden_sound_is_mixed_bit_for_bit`, two in `crates/golden/tests/sound_references.rs`, and `every_event_and_job_kind_is_described_for_the_window`. That commit predates `.gitattributes`, so git rewrote line endings in the clone. Re-run on the next commit (`57a9376`, the first with `.gitattributes`), the three test programs holding those failures passed: 19 of 19. The rest of the workspace was not re-run on that commit, but no code changed between the two. The end-to-end tests were not re-run in the clone; the app job ran them twice in the main folder, with 69 of 69 passing.

## 3. The stages

| Stage | What it delivers | Status | Tag | Reports |
|---|---|---|---|---|
| 0 | Golden pictures and sounds from the OLD code; schema-2 test projects | Done, verified | `stage-1-done` | `stages/s0-*.md` |
| 1 | `crates/blocks`, all 17 built-in blocks, security review | Done, verified | `stage-1-done` | `stages/s1-*.md` |
| 2 | Schema 3, migration, old journals, edges; renderer and mixer match new ids | Done, verified | `stage-2-done` | `stages/s2-*.md` |
| 2a | Picture proof with no graphics card; the frozen and failing video tests on this PC | Done on Windows; Mac, Linux and ARM not run | `no-gpu-proof-done` | `stages/ng-*.md` |
| 3 | Renderer runs picture blocks generically; blur, glow, wipe, blend | Done, verified | `stage-3-done` | `stages/s3-*.md` |
| 4 | Mixer runs sound blocks generically; smooth ramps; eq3 | Done, verified | `stage-4-done` | `stages/s4-*.md` |
| 5 | Registry service, block manager, inspector from manifests, strings, CSP, sound thread, `.chadfx-block` file type and installer, blocks in one `.wgsl` file | Done, verified (main script, then a fix round, then the main session's fixes) | `stage-5-done` | `stages/s5-*.md`, `stages/s5f-*.md` |
| 6 | Safety tests for every kind at every layer, the uniform cap against the card, install-use-export e2e, track and sequence effects, `docs/BLOCKS.md`, the template folder, the final report | Done, verified (main script, then the main session's fix round) | `stage-6-done` | `stages/s6-*.md`, `stages/s6f-*.md`, `FINAL-REPORT.md` |

Report paths are under `docs/blocks-rewrite/`. Tag `baseline-before-blocks` is the code before the rewrite; use it to read old behaviour.

## 4. Binding rules

From the brief:
1. A project stores block id + version + parameters. A missing block still opens, shows as missing in the interface and the AI view, renders without it, warns on export, and keeps the node byte for byte.
2. Untrusted blocks cannot harm the computer or files, cannot reach the network, and cannot crash or freeze the app. A failing block is disabled with a clear message.
3. Preview and export produce identical pictures and sound.
4. Undo, journal, crash recovery and exact rational time are unchanged.
5. Schema-2 projects open through a tested migration and render pixel for pixel the same, proven by golden images made BEFORE the change.
6. Decide, document and justify what stays core (time and placement) and what becomes a block.
7. Free and local; Windows, Mac and Linux; a mid-range laptop. Install from a file only; nothing downloaded.

Owner decisions:
- D1: the migration adds a neutral colour node to picture clips and sequences, for byte-exactness.
- D2: keyframed volume and pan become smooth ramps, identical in preview and export.
- D3: text and captions are the next task after this rewrite.
- D4: the defect-finding reviewers run after the plug-in work.

Corrections that override older text (spec 19):
- **Sound precision.** For static cases every sample is bit-exact or within 3 units in the last place of a 32-bit float, under 3 parts in ten million. The earlier "about one part in ten million" holds only for one gain block. Blocks with memory (eq3) agree with uninterrupted playback to about 1e-12 after warm-up, not bit for bit.
- **Sound engine.** wasmi keeps `portable-dispatch`. `auto-dispatch` is about 3 times faster but can overflow the stack in unoptimised builds, which breaks rule 2.
- **Panics.** Every guard relies on panics unwinding: no build profile may ever set `panic = "abort"`.
- **New limits** (WGSL operators, locals, copy costs, version length): see spec 19.

## 5. How to run a stage

This pattern has worked, and it is the reason the work has been sound:
1. Before a stage, take stock with `git status` and a health check; keep test media small.
2. Write one workflow script: build jobs, then reviewers, then an acceptance checker. Copy `docs/blocks-rewrite/workflows/blocks-stage-2.js`: its `RULES`, per-job "You own", `TAIL`, `REVIEW_RULES` and `ACCEPT` are the template. Also copy the `speed(step)` text from `workflows/no-gpu-proof.js`: while working, agents run only the test at hand; whole packages once at the end of a step; repeat rounds only in the acceptance checker; planted bugs once, on one reused temp copy, never in the real source; big jobs split into smaller agents with running notes, because a report is saved only when an agent finishes. The owner chose on 2026-11-01 to keep all the reviewers.
3. **Build jobs** each own explicit paths. Run jobs that break each other's compile one after another; give each the previous report.
4. **Reviewers** are adversarial with authority to fix: "prove each defect with a failing test, fix it, keep the test; no proof, no finding; never weaken a test." Give them disjoint paths and tell them where to look hardest.
5. **The acceptance checker** runs every check and ticks off each deliverable with file and test evidence.
6. **Then verify yourself.** Agent reports are claims. Stage 1's security review found 13 real problems that the builders' tests missed, including a crash of the whole process.
7. Save reports to `docs/blocks-rewrite/stages/`, add any rule change to the spec as a new dated section, commit, tag `stage-N-done`.

A stage ends only with: zero warnings in `cargo build --workspace --all-targets`; `cargo test --workspace` green; and in `ui`: `npm run build`, `npm run lint`, `npm test`, `npm run e2e` green.

Git: agents never change git state, because they share one working tree; read-only git is fine. The main session commits and tags. Push only when the owner asks; earlier approval does not carry over.

Team size: decide by what is best for the product, neither shrinking to save usage nor inflating to spend it.

Usage limits have cut runs off repeatedly, and the owner just says "continue". In the same session: `Workflow({scriptPath, resumeFromRunId})`; finished agents replay from cache. From a new session the run cannot be resumed: check the build and tests, then run a new script containing only the unfinished jobs, as `stage2-finish.js` does, and tell agents to read and continue partial work on disk. If the owner asks for a clean stop before a limit: stop the workflow and background tasks by task id, write down what stopped and how to resume, then say so plainly.

Disk: stop if free space on C: falls below 50 GB; it is a hard rule from the owner. Watch it with a background loop that alerts below the floor, and when it alerts, stop the run first, then look. `target/debug` grows fast (84 GB was seen; 58 GB during Stage 3, with feature-variant builds of `render`; 67 GB, 19 GB of it the incremental cache, during the Stage 5 fix round, which fell to 49 GB free and was paused and resumed). The owner installs large things meanwhile: on 2026-11-03 free space fell from 117 to 37 GB in three hours, mostly from a game update and two Adobe betas. Never delete the owner's files. Without a full rebuild you can delete `target/debug/incremental` when nothing is building (29 GB freed once), and `*.exe` and `*.pdb` older than two hours in `target/debug/deps` (40 GB freed once). `cargo clean` frees everything (58 GB) at the cost of one full rebuild; run it only when no agent and no test is running. Agents must delete their temp copies and those copies' build folders as soon as they are done with them.

Processes: **never end a process by image name**; the tool itself runs as `node.exe`. End only what you started, by id. Agents' log watchers (`tail -f`, started by the Monitor tool) never end by themselves: a workflow agent that leaves one counts as "waiting on its own background work", and the workflow waits for it. In Stage 4 the run only moved on once the main session ended the leftover watchers by process id. Tell agents to stop their monitors when done, and look for leftover `tail.exe` processes when a run seems stuck.

Shared scratch space: every agent shares one scratch folder, so each planted-bug copy and build folder needs a name with the agent's own prefix (for example `s4rs-plant`). In Stage 4 two reviewers used the same `plant` folder at once, and one reviewer's results had to be redone.

Messages to running agents: do not send a message to an agent of a workflow that is still running. In Stage 4 that started a second copy of the agent, working beside the first in the same files; both handed back reports. Wait for the agent's result instead, or tell the owner.

## 6. The remaining stages

### Stage 3: the renderer runs picture blocks generically (spec 7): done
Done and verified on 2026-11-03 (tag `stage-3-done`; reports `stages/s3-*.md`; rules in spec 22). `crates/render` has no block ids left: it plans each layer from the catalogue, composes the stack with `crates/blocks/src/wgsl`, draws passes, blends and mix transitions through intermediates, and runs blocks of the user under the runtime protections of 7.9. Old pictures are byte for byte the same on the card, on WARP and with no graphics card. Blur, glow, wipe and blend work and the window offers them. The block checker refuses what the software renderer refuses. What it left for later stages is in spec 22's "Left open" and in section 7 below.

### Stage 4: the mixer runs sound blocks generically (spec 8): done
Done and verified on 2026-11-04 (tag `stage-4-done`; reports `stages/s4-*.md`; rules in spec 23). `crates/audio` has no block ids left: an `audio::Mixer` session plays every clip, track and sequence node, transition and sound source from the catalogue, in 128-frame slices on absolute sample positions, with one instance per (node, clip), reset and warm-up after a jump and before a chain starts afresh, and parameter values at each slice's start and end, so keyframed volume and pan are smooth ramps (D2). Old sound is the same within the rounding statement; preview equals export. eq3 works and the window offers it. Hostile sound blocks are held by fuel, a 512 MiB session memory budget and a fault that turns the block off for the session. After the acceptance check the main session had the warm-up at a clip's start added (a clip with an EQ cut in two clicked at the cut). What it left is in spec 23's "Left open" and in section 7 below.

### Stage 5: app and interface (spec 11, 12, 13, 15, 17): done
Done and verified on 2026-11-06 (tag `stage-5-done`; reports `stages/s5-*.md` for the main script and `stages/s5f-*.md` for its fix round; rules in spec 24 and 25). The app keeps one registry and one catalogue snapshot (`blocks::CatalogSlot`) that the renderer, the mixer and the export read, and every change reaches the window at once. The window has a block manager (install from a file, a folder or a dropped file, turn off and on, enable again, remove, with every package text shown as plain, isolated text and icons decoded in the backend), an inspector and transition settings built from manifests, all its texts in `ui/src/lib/strings.ts`, a simple-or-pro flag and a strict content security policy. Older-format nodes are shown converted and converted on edit; a block in the wrong place is unusable everywhere, with a reason. The player mixes sound on a thread of its own, follows a lost or changed sound device, and plays the export's samples at any device rate. A packaged app ships `blocks/builtin`. The owner's two requests are built: the `.chadfx-block` file type (double-click or "Open with" asks "Install <name> by <maker>?", also when ChadFX is already running), with a real per-user NSIS installer, and a picture block in one `.wgsl` file whose description sits in a header comment (spec 24). After the fix round's acceptance, the main session added installer hooks (the open command's program path is quoted; the uninstaller leaves no empty `.chadfx-block` key), limited the Windows build to NSIS, named the clip and track in the export's missing list, and worded refusals of a `.wgsl` file as a file. What it left is in spec 25's "Left open" and in section 7 below.

**The owner should check one thing by hand:** Claude's app container redirects registry writes, so the file association was only ever seen inside it. Running `target/release/bundle/nsis/ChadFX_0.1.0_x64-setup.exe` outside Claude shows whether Explorer gives `.chadfx-block` files the name "ChadFX Block" and ChadFX's icon.

### Stage 6: proof and documents (spec 16): done
Done and verified on 2026-11-06 (tag `stage-6-done`). Reports: `stages/s6-*.md` for the main script, and `stages/s6f-*.md` for the main session's fix round and its own check (`s6f-acceptance.md`). Rules in spec 26. The final report is `FINAL-REPORT.md`.

The main script (`workflows/blocks-stage-6.js`) ran four build jobs at once, then the install-use-export end-to-end test, then the documents; three reviewers (an attacker, a person using the window, a newcomer following the guide); and an acceptance check. That check ran every test three times and judged the stage "done with gaps", because the hostile-sound tests were only partly written: a safety filter stopped the sound engineer from writing hostile WebAssembly by hand.

The main session then ran a fix round of two engineers, one after the other:
- `s6f-sound` wrote the sound tests with the existing builders (`crates/blocks/tests/sound_cases`): the mixer's `contained_sound.rs`, the player's `contained_sound_player.rs`, hostile sound blocks in the running window (`blocks-contained.e2e.ts`) and the table `stages/s6-sound-inventory.md`.
- `s6f-draw`:
  - named a node left out of a draw in the window's export summary and preview (`left_out_of_draw`);
  - made `vep blocks install|validate|pack` refuse Windows device paths with the window's own rule;
  - made descriptions keep their line breaks.

What Stage 6 built:
- Containment tests for every picture and sound kind at every layer.
- The composer's uniform cap is the card's own limit.
- A mistyped shape value is named in every list.
- Track and sequence effects are edited in the inspector.
- Notices never cover the panels above the timeline, or a dialog's content.
- `blocks/template`.
- `docs/BLOCKS.md`, pinned to the code by `crates/blocks/tests/docs.rs`.
- The install-use-export end-to-end test.
- The cost of sound blocks was measured.
- The reviewers' fixes, including a renderer panic on transition lengths off every grid.

What it left is in spec 26's "Left open" and in section 7 below.

**The owner should check two things by hand** (they cannot be done inside Claude's app):
- Run `target/release/bundle/nsis/ChadFX_0.1.0_x64-setup.exe` outside Claude, and see whether Explorer gives `.chadfx-block` files the name "ChadFX Block" and ChadFX's icon.
- Unplug or switch a sound device while the editor plays.

## 7. Carry-forward list

| Item | Owner |
|---|---|
| After the first commit or undo on an upgraded old file, a reopen has no undo history before that point (a checkpoint with an empty journal) | By design; a person may notice it, raise it if so |
| FFmpeg's gap filling (`aresample=async=1`) gives slightly different sound from a damaged file from run to run, so preview and export can differ on such a file; 64 runs at 24 at once gave 9 results, and 1 without that filter (spec 21) | Accepted for now; only damaged files |
| A hand-edited file with a derived-id collision opens but refuses adding a track or sequence until fixed | Accepted; no real file can reach it |
| The neutral colour node prints as "Color" on every upgraded clip in the AI's text | The AI work decides |
| `crates/golden/src/sound.rs` names the retired generator without `.txt`, left so that frozen file stays byte-identical | Accepted |
| None of the Stage 2, 3 or 4 changes has run on the RTX 3090 computer | Run the health check there when next on it |
| The picture proof has never run on a Mac, Linux or an ARM processor, or on an AMD, Intel or Apple card; Stage 3 was proven only on the RTX A4000 (Vulkan, Direct3D 12, OpenGL) and WARP | Owner to choose how; `cargo test -p cpu-render -p golden`, then `no_gpu_pictures` |
| `crates/cli` tests wait on the `vep` program with no limit of their own; three test stand-ins wait on real FFmpeg, stopped only by the outer limit | Accepted for now; a freeze there would need a look |
| A stuck decoder can hold the preview for up to 5 minutes | Accepted for now |
| A hostile shader in `cpu-render` ends in an error, but at worst after about 45 CPU-minutes | Accepted; test-only code |
| `D:\chadfx-base`, `D:\chadfx-health`, `D:\chadfx-health-target` (22 GB) are leftovers of the health check | Owner may delete them |
| Infinities and NaN in new-block frames: the card and the software renderer may disagree (spec 22) | Open; old pictures unaffected |
| Where the two sides of a wipe have different blends the picture can jump at its ends; a cross dissolve on a multiply track is darker in its middle | Accepted for now (spec 22) |
| Smaller Stage 3 items: many different hostile blocks together under the tripwire; one thread per user stack; a quarantine racing a planned frame; a compile outliving its limit at a normal close | Accepted for now (spec 22) |
| Default values of eq3 are a first draft (0 dB on all bands; corners 200 Hz and 4 kHz) | The owner: listen and decide (FINAL-REPORT) |
| Sound blocks cost about 18% of one core for eight tracks with gain, pan and eq3 (measured in Stage 6); fuel underprices denormal arithmetic (9 times) and 1,024-local calls (5 times), so eight hostile tracks run sound 10 to 15 times slower than real time | The owner: the fuel price and the locals limit (FINAL-REPORT) |
| The `.chadfx-block` file association was seen only inside Claude's app container, which redirects registry writes | The owner: run the installer outside Claude |
| A real sound device was never unplugged or switched; the recovery rests on test doubles and cpal's own source | The owner, by hand |
| The Mac path of "Open with" (`RunEvent::Opened`, the Mac bundle) was never built | When a Mac is available |
| The installer, Start menu and "Open with" say ChadFX; the window's title still says Vep | The owner's choice |
| The copy rule stays exact: a copied node with different keyframes counts as new and is refused if its block is unusable (decided in Stage 5, `edges.rs`) | Decided |
| Smaller Stage 5 items: the focus rule's time window; device-like shares other than `pipe` (a mailslot) are not refused | Accepted for now (spec 25) |
| Smaller Stage 4 items: a faulted hostile block off from the next call; near the 512 MiB budget the call pattern matters; wasmi's interpreter stack outside the budget; negative-duration transitions untested | Accepted for now (spec 23) |
| rustdoc link in `crates/cli/src/cancel.rs`; older clippy warnings in core-ids, core-model, core-time | Any time |
| One `in-flight.json` per user folder; two processes are last-writer-wins, including `state.json` and simultaneous installs | Accepted for now; revisit if `vep export` runs untrusted blocks |
| Hard links inside folder packages cannot be detected safely on Windows | Accepted |
| Fused blocks in one layer share one loop budget, so a hostile block can spoil only its own layer | Accepted |
| Windows device names such as `con.glow` on Windows 10 and earlier | Unproven; re-check |
| Timing bounds in `crates/blocks/tests/security.rs` could flake on an overloaded machine | Accepted |
| The owner's decisions from Stage 6: whether `vep` ships with the installer; whether the uninstaller also deletes `HKEY_CURRENT_USER\Software\vep\ChadFX` (the remembered install folder); notices in the timeline panel's corner or a status-bar list; forcing colours to finite values; a faulting sound block off for good instead of for the session; generator clips and keyframes from the window | The owner (FINAL-REPORT) |
| The uniform cap against a real card smaller than 64 KiB was only simulated (a 16 KiB device on the A4000) | When such a card is available |
| Smaller Stage 6 items: Delete with only a track selected does nothing and says nothing; tooltips that hold information are mouse-only; a removed block is named by its id in the export dialog; a runtime problem names no clip or track, so two left-out nodes of one block read alike; no end-to-end test installs, replaces or removes a sound block while it plays (the player's tests do); the registry re-checks every package after each install | Accepted for now (spec 26) |

The full lists are the `known_gaps`, `remaining_risks` and `deviations_from_spec` sections of every report in `docs/blocks-rewrite/stages/`. This table is a summary and was not exhaustively cross-checked: go through those sections at the start of each stage.

## 8. After the rewrite

In order: **text and captions** (D3), then the **defect review** (D4). Finding B1 in `docs/REVIEW-2026-10-29.md` (decoder memory is limited by count rather than bytes; 4K on 16 GB fails; about a week to fix) is for the owner to schedule; raise it.

**Do NOT start the AI planner, the skills system or anything else from later phases unless the owner asks.**

## 9. Talking to the owner

Explain everything in normal English. Do not show unexplained terminal commands, shell syntax, raw Git output or compiler errors. Before major changes say what, why and which files; afterwards say what changed and whether it worked. Name agent jobs in plain words: the owner has objected to unexplained jargon. Report failures plainly.

**End-of-stage checklist:** every check run and read by you; reports saved; spec updated; committed and tagged; owner told in plain English what works, what does not and what is next.
