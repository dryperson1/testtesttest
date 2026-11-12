# ChadFX product brief

What the product owner wants, gathered in an interview on 2026-10-24. Plans for later phases should be checked against this. Where [ARCHITECTURE.md](ARCHITECTURE.md) and this brief disagree about *what* to build, this brief wins; ARCHITECTURE.md still decides *how*.

## The product

- **Name:** ChadFX. The code calls the app "Vep" for now.
- **A public product**, eventually released for many people. How it makes money is undecided.
- **For beginners and pros alike**, through two modes of the same app (see below).
- **Every kind of video:** sports and hype edits, short-form social, YouTube and long-form, cinematic. Sports and hype edits come first.
- **Windows, Mac and Linux.** No phone or tablet version.
- **English first.** Other languages later.

## Hard rules

- **The user's material never leaves the computer.** Footage, sound, projects, chat, and everything the AI works out stay on the user's computer. No cloud AI and no online editing service, not even optionally. Everything is free to run. The app itself may go online only for:
  - downloading AI models once, at setup or when a feature is first turned on (never during editing);
  - updating itself automatically;
  - a one-time online activation, if the product is ever sold;
  - crash reports, and only when the user agrees each time.
- **Scales to the machine.** The weakest supported computer is a mid-range laptop: 16 GB of memory with a basic graphics card, or an Apple M1. Stronger machines give better and faster AI results.
- **The AI never:**
  - deletes or changes the original footage;
  - overwrites edits the user made by hand, unless asked;
  - makes a change that is not shown in the history and cannot be undone;
  - uses the internet.

## How a session works

1. **Import.** The app starts studying the footage in the background as soon as it is imported, so the AI is ready when asked.
2. **Ask.** The user types a request in a chat panel beside the preview, for example "a 60 second hype edit of the best plays". Typing in chat is the main way to talk to the AI.
3. **Plan.** Before building anything, the AI shows a story outline split into sections (intro, build-up, climax and so on). The user can reorder, delete or rewrite sections directly, or reply in the chat ("use the second song", "more crowd shots"), or simply approve.
4. **First cut.** The AI builds the whole edit by itself. For about an hour of footage, the first cut should appear in **under 15 minutes**, thanks to the background study done at import.
5. **Refine.** Most of the time is spent on:
   - **chat changes** ("make the intro shorter", "swap the third shot for something funnier");
   - **point and ask**: select a clip or a moment, then ask the AI about just that part.
   - The **timeline** stays available for quick fixes by hand: trim, move, delete, split.
6. **Explain.** Every AI change comes with a short one-line summary of what it did and why (for example "Replaced shot 3 with the dunk at 12:40 because it has more motion"), and can be undone.
7. **Export** to a file on disk. No uploading and no platform presets needed at first.

Expected footage per edit: about **an hour** (a full game, an event, a day of filming).

## What the AI is responsible for

In order of importance for the first cut:

1. **Picking the best shots**, judged by what the user asked for (action, funny moments, emotion) and by technical quality (sharp, steady, well exposed).
2. **Cutting to the music**, which comes from the user's own files: beats, drops and energy.
3. **Telling a story**: a sensible beginning, middle and end.
4. **Effects and polish**, which matter but come after the three above.

The AI also handles, so that users do not have to do them by hand:

- captions from speech;
- reframing wide footage to vertical by following the subject;
- removing boring parts (silences, dead time, bad takes);
- colour and look;
- text on screen, sound levels (for example music under speech) and animation.

Not wanted: learning the user's taste over time, saved or built-in style presets, copying a reference video, voice control, AI-generated music, several alternative versions of an edit.

## The two modes

- **Simple mode** has the same layout but tucks away settings, effect parameters and technical details.
- **Pro mode** shows the AI's work (its plan, how it scored shots, why it chose them) and lets the user change it directly. It also keeps full manual controls (animation, text, sound, colour) for those who want them, even though the AI normally does these.
- The user can **switch modes at any time** without losing anything.

## First target: sports and hype edits

- **Any sport**, including extreme sports: the AI should work out what the action is.
- **Any footage**: phone videos, full game recordings, broadcast clips, action cameras.
- What makes a hype edit feel great:
  - the big moments land exactly on the beat or the drop;
  - slow motion at the peaks, then snapping back to speed;
  - a build-up from calmer to a climax.
- Focusing on one player ("a highlight reel of number 23") is nice to have, but comes after everything else is finished.

## What success looks like

The AI's first cut is good enough that, after a few chat tweaks, the user would actually post it.

## Look and feel

- **Minimal:** mostly the video and the chat, very little clutter.
- Inspirations: the seriousness and control of Premiere and Resolve, and the feel of a ChatGPT-style chat.
- The chat lives in a **side panel**, always open beside the preview.

## Decisions that are expensive to change

Gathered in a second, deeper interview on 2026-10-24. Most of these shape data formats or core behaviour, so code written today must not block them.

**Projects and files**
- **One project makes one video in one shape.** No several edits per project; no linked wide and vertical versions. Vertical means a separate project.
- **A project is one file** on disk, to be named `.chadfx` (today `.vep`). Footage stays outside it.
- **Footage is referenced, never copied.** Files stay where they are; if they move, the project asks where they went.
- **The AI's study of the footage lives inside the project.** Deleting the project deletes it; it is not shared between projects.
- **No moving or sharing of projects** between computers, and no collaboration.
- **Very long projects** (edits of an hour or more) must be possible.

**The AI's work in the project**
- **The whole chat is saved with the project:** conversation, plans and explanations, available when the project is reopened.
- **Undo goes step by step** through each small change the AI made, not one undo per request.
- **The plan's sections are used to build the first cut, then become history.** They do not stay linked to the timeline.
- **Before replacing anything the user edited by hand, the AI asks** ("you edited 2 clips here, replace them?").
- **When unsure, the AI decides and says so** in its summary, naming the alternative.
- **Results vary.** Asking the same thing twice may give a somewhat different edit.
- **The chat only edits the video.** It does not run the app (export, import, save) and is not a help system.
- **Point and ask can target:** a clip, a stretch of time across all tracks, or a section of the plan. Pointing at things inside the picture is not needed.

**Settled in a final round (2026-10-25)**
- **The study stays per project, even when footage repeats.** Making a second video from the same footage means studying it again; that cost is accepted, so that deleting a project really deletes everything about it.
- **Simple mode shows a simplified timeline:** a single strip of clips the user can trim and point at, without tracks and tools.
- **No song, no build.** If a hype edit is asked for without music, the AI asks for a song before building.
- **The chat only moves forward.** Earlier messages cannot be edited to branch or redo; Undo is the way back.
- **Corrections are made by saying them in the chat** ("no, I meant the dunks, not the layups"). No thumbs-down or pin buttons are needed.
- **Model size is the user's choice:** the app offers best quality (slower), balanced, or fast and lighter models.
- **What to build next:** after the conflict review, first fix whatever the review finds that blocks this brief, then add features.

**Footage, export and captions**
- **Footage up to 4K and HDR**, handled correctly (through proxies for smooth editing).
- **Export up to 4K, in HDR, at 60 fps and above**, and in vertical sizes such as 1080x1920.
- **The background study starts right away** on import.
- **Captions:** burned into the picture with word-by-word pop-in, a separate subtitle file when the user wants one, and words corrected by typing.
- **Sound in hype edits:** music only for now. Game sound under music, sound effects and natural-pitch slow motion come later.

**Openness, updates and the long term**
- **Swappable AI models:** advanced users can choose which local model to use, or bring their own.
- **Effect building blocks are plug-ins (decided 2026-10-28, replacing the earlier "no third-party plugins").** Team members AND outsiders must be able to add new effect building blocks without changing the core and without rebuilding the app: install a block package, and the renderer, the inspector and the AI can all use it. Every existing built-in effect is moved onto this same plug-in system; nothing stays hard-wired. A project that uses a block the app does not have still opens, shows the block as missing and keeps it intact. Outsiders' blocks are untrusted and must not be able to harm the computer or the user's files.
  - *Status, 2026-11-06:* built. Every effect, transition, blend mode and generator is a block in one format, the editor's own 17 included; a block is installed from a file, a folder or a dropped or double-clicked file into a running app, with no rebuild, and the renderer, the mixer, export, the inspector and the AI's catalogue use it at once. How to write, test, pack and install one, what is checked, what a block cannot do and what is still open: [BLOCKS.md](BLOCKS.md), with a template folder in `blocks/template`. The technical contract and the work's history: `docs/blocks-rewrite/`. What stays in the core (time and placement) and why: BLOCKS.md, section 10.
- **The interface must be translatable later:** keep all interface text in one place from now on.
- **Keyboard-first:** everything reachable without a mouse.
- **If ever sold:** a one-time online activation is acceptable.

## Where this brief pulls against the current build

Recorded at the end of Phase 2 so later planning addresses them:

- **The window needs a redesign.** The Phase 2 window is a dense, dark, pro-style editor, while the brief asks for a minimal look with the chat as the main tool. The engine underneath already fits: every change goes through the same commands an AI can call.
- **Laptops, offline and speed pull against each other.** An ordinary laptop, fully offline, analysing an hour of footage, with a first cut in under 15 minutes, is the hardest combination in the brief. On a laptop with no graphics card the background study may take longer than that, so expect a visible difference in speed and quality between weak and strong machines.
- **"Any footage" will be reached gradually.** Odd formats, very long recordings, and broadcast footage with replays and scoreboards are a long tail.
- **Text and captions are not drawn yet.** The renderer does not draw text, and captions are central to the brief, so text rendering moves up the roadmap.
- **The pipeline is 8-bit standard range only,** while the brief requires 4K HDR footage in and HDR out. Decode, rendering, proxies and export all assume SDR today.
- **AI study results are planned for a shared, per-computer cache** (ARCHITECTURE section 8.2: reusable across projects, keyed by file content). The brief wants them inside the project file and deleted with it.
- **Several sequences per project.** The data model and the interface support many sequences in one project; the brief wants one video per project.
- **The internet rule now has four allowed uses** (model download, auto-update, activation, opt-in crash reports). ARCHITECTURE.md and LOCAL-FREE-DECISIONS.md still say "nothing online".
- **The project file extension** is `.vep`, not `.chadfx`.
- **Interface text** is written directly inside the React components, not kept in one place for translation.

## Checklist for a conflict review

An agent checking the code against this brief should look at least at:

1. Anything that makes the internet rule above impossible or harder (for example hard-coded "never online" assumptions where model download or updates will need to plug in).
2. The project file format: can it hold the full chat, plans, explanations and the AI's per-shot study results, and can it grow to hour-long edits?
3. Undo and history: can a single chat request be recorded as many separately undoable steps, each with its explanation, and can a step be marked as "made by the AI" or "made by hand"?
4. Hand-edit protection: is there any record of which clips the user changed by hand, so the AI can ask before replacing them?
5. 4K and HDR from import to export: colour depth, colour handling, proxies, renderer formats.
6. Text rendering and captions: word-level timing, animation, a subtitle file export.
7. Model swapping: is the AI side designed so local models can be chosen or replaced?
8. One video per project, one shape per project: what in the current model assumes otherwise, and whether that is harmless or should be removed.
9. Interface text kept in one place for translation, and keyboard access to everything.
10. Behaviour on a mid-range laptop (16 GB, basic graphics card or Apple M1): anything that assumes a strong graphics card.
11. Effect building blocks as plug-ins: anything that assumes a fixed, built-in list of effect types (the closed `EffectKind` list, the hand-written catalogue, renderer code that knows specific effects by name, interface code with effect-specific knowledge).
