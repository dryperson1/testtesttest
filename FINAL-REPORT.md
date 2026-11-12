# ChadFX plug-in blocks: the final report (Stages 0 to 6)

Checked on 2026-11-06 on the RTX A4000 computer (Windows 11, Intel i9-13900). This report covers the whole rewrite. Every number in it was measured on that computer. The detailed reports are in `docs/blocks-rewrite/stages/`, and the binding rules in `docs/blocks-rewrite/BLOCKS-SPEC.md`.

## In short

The rewrite is built, and it works on this computer.

- Every effect in ChadFX is now a "block". A block is a small package that anyone, including outsiders, can write and install into the finished app. Nobody has to change or rebuild the editor.
- Old projects still open, look the same and sound the same. This was proven against pictures and sounds recorded from the old code before any of this work began.
- Broken and hostile blocks are refused, or kept from doing harm. This was proven for every kind of picture and sound block, at every level from the block checker up to the running window.

Not proven: anything on a Mac, on Linux, on a mid-range laptop or on another graphics card.

## What the editor can do now with blocks

- **One format for everything.** Effects, transitions, blend modes and generators are all blocks. The editor ships 17 of its own, in exactly the format an outsider uses:
  - picture: move and scale, crop, opacity, colour, mask, blur, glow, blend modes, the normal blend, cross dissolve, wipe, solid colour;
  - sound: volume, pan, a three-band EQ, crossfade, a test tone.

  The editor's core does not know any of them by name.
- **What a block is.** A description file holds the block's name, a plain description the AI will read, and its settings with their types, ranges and units. The code comes with it:
  - a picture block's code is a shader, a small program in the graphics card's language (WGSL);
  - a sound block's code is a WebAssembly module, compiled code that runs sealed off from the computer. It can be written in Rust, for example.

  A picture block can also be a single `.wgsl` file, with its description at the top.
- **Installing a block** puts it into the running editor, from a file. Nothing is downloaded, and nothing in the file runs before all of it is checked. The ways in:
  - the block manager: a file, a folder, or a file dropped on the window;
  - after the Windows installer, double-clicking a `.chadfx-block` file, or "Open with" ChadFX;
  - the command line, `vep blocks install`.
- **Using a block.** The inspector builds its controls from the block's description:
  - a clip's effects;
  - new in Stage 6, a whole track's effects and the whole sequence's effects, with switch, reset, move, remove and undo;
  - transitions, with their settings.

  The preview, the export and the AI's description of the timeline pick up a new block at once.
- **When a block is missing, turned off, broken or misbehaving:**
  - the project still opens, plays and exports, without that block;
  - the block manager, the inspector, the timeline, the export dialog, the export summary and the AI's text each say so in plain words;
  - the project keeps the effect exactly as it was, so installing the block again brings it back.

  This includes a block left out because a layer holds more than one draw can take; the window's export summary and the preview's problem list now say so too.
- **The command line:** `vep blocks list`, `info`, `catalog`, `validate`, `pack`, `install`, `remove`, `disable`, `enable` and `enable-again`.
- **For outsiders:**
  - `docs/BLOCKS.md`, the guide: how to write, test, pack and install a block, every rule, and every refusal with what to do about it;
  - five working templates in `blocks/template`: a picture filter, the same filter as one file, a transition, a generator, and a sound filter written in Rust.
- **A Windows installer** for the current user only, 8.6 MB. It registers only the `.chadfx-block` file type.

## What was proven, and how

1. **Old pictures are unchanged.** Before the rewrite, the old code drew 351 test scenes, and those pictures were stored. It also crashed on one scene, and that crash was recorded.
   - Today's editor draws all 351 byte for byte the same, and the recorded crash still happens the same way.
   - This holds on the RTX A4000, on Microsoft's software renderer (WARP), and with no graphics card at all. The last one uses a software renderer written in Rust that reads the same shaders.
   - On this card, the new code and the frozen old shaders give identical bytes, both the way the preview draws and the way the export draws.
   - The card and the software renderer differ only by rounding: 1.9% of the bytes differ, almost all by 1 level out of 255, and the largest difference is 6 levels. That is within the stated bound.
2. **Old sound is unchanged, within the agreed rounding rule.** The proof uses 118 old projects and 590 recorded sounds.
   - 344 sounds are compared sample by sample: 289 are bit-identical, and 28 are within 3 units of the last digit of a 32-bit number.
   - The remaining 27 come from 7 test projects: mixed or dissolving sounds, and the test tone. They are within the slightly wider rule for those, about one part in a few million.
   - Every project gives the same samples however playback is cut into pieces.
   - The proof printed exactly what it printed at Stage 4.
3. **Preview equals export,** for pictures (point 1) and for sound: the player plays the export's own samples.
4. **Hostile and broken blocks are contained.**
   - **Picture blocks of all four kinds** were attacked through the block checker, the drawing code, the app and the command line:
     - damaged and hostile packages: zip bombs, paths that climb out of the package, links, Windows device names;
     - absurd descriptions: 5,000 settings, defaults that are not numbers, hidden direction-changing characters;
     - code the checker refuses: 27 rules, each tried on each kind;
     - code the checker accepts but that is as costly, as deeply nested or as wild as the rules allow.

     Every frame came back, and the window kept answering. A block that is too slow, or that was in use when the editor crashed, is turned off, stays off after a restart, and is named everywhere.
   - **Sound modules broken in every way the checker knows** are refused in plain words, through the app and through the command line.
   - **Sound blocks of all three kinds that pass every check but misbehave once used** were run in the mixer, through the player while it plays, in exports, through the command line, and in the running window. They crash, run out of their work allowance, write outside their memory, fail in the middle of a crossfade, or lie about how long they need to warm up.
     - The sound plays on, and the export finishes and names each of them.
     - The file holds exactly what plays instead: the clip's own sound, silence, or a plain crossfade.
     - In the running window, while a crashing, an endless and the costliest allowed sound block played, the window answered within 1.5 seconds and the preview named the faults.
     - Installing, replacing, turning off and removing a misbehaving sound block while the project plays never held the picture or the playhead.
   - The table of which test proves what, for sound, is `stages/s6-sound-inventory.md`. For pictures, it is in `stages/s6-picture.md`.
5. **The owner's end-to-end test.** In the running app, the template picture filter was installed from its file, put on a clip, changed in the inspector and exported.
   - The PNG still is within 1 level of the shader's own arithmetic, and the ProRes video is within the encoder's stated tolerance.
   - The template sound filter, exported to WAV, gave all 288,000 samples exactly as its arithmetic says.
   - The one-file form was installed by dropping it on the window.
   - Once removed, each block is listed with its clip and track. The export leaves it out, byte for byte. The saved project keeps it unchanged.
6. **Checks done by hand, as a person would do them.**
   - The guide's walkthrough was followed word for word, with the release command-line tool copied outside the repository. The picture with the block matched the shader's arithmetic within 1 level, and the sound matched the template's arithmetic exactly. Turning off, removing, replacing and installing again did what the guide says.
   - The editor was installed with its installer into a folder with a space in its name, a block file was opened with the installed program, and the editor was uninstalled.
7. **Test totals, after the last change** (the main session's own check, 2026-11-06, 17:59 to 18:51):
   - the Rust tests: 2,952 passed, 0 failed, 35 skipped by design, in 264 test programs, in about 29 minutes;
   - the window's build and its code check passed, and its 763 unit tests passed;
   - the end-to-end tests, which drive the real app: 97 passed, plus 2 that run only on request.

   Before the fix round, the acceptance check had run every Rust test three times in a row, with no change between rounds, and had run both on-request tests by hand.

## Defects found and fixed in Stage 6

The reviewers and the fix round found these, and each fix has a test that fails without it.
- The renderer crashed on an outgoing transition whose length lies on no frame grid, an exact fraction the editor accepts. This dated from before the rewrite. It is fixed without moving any picture.
- Some refusals quoted other libraries' words raw, including names a package chose, with invisible direction-changing characters and no length limit. Now they are shown as one plain line.
- `vep blocks enable-again` could turn on a block the person had turned off.
- A blocks folder with more than 1,000 packages lost blocks silently, and installing into it said "Installed" of a block that was then not there. Now the folder says so, and a new install into a full folder is refused.
- Four refusals gave a false or confusing reason:
  - a shader stage written as an entry point;
  - a one-file `.wgsl` block declared as sound;
  - a sound module whose memory is too large;
  - a refused `.wgsl` file called "the package" in the block manager.
- `vep effect set` refused a colour in the form `vep blocks info` prints.
- Notices covered the inspector, the preview's controls and dialogs' buttons, where the keyboard could not reach them.
- A menu closed with the keyboard dropped the keyboard focus.
- An add menu that offered nothing gave no reason, or a false one.
- The sequence's effect order was explained with a clip's rule.
- The end-to-end tests built the app without the "offline" setting.
- The window's export summary and the preview's problem list said nothing of a block left out because a layer holds more than one draw can take.
- `vep blocks install` read Windows device paths that the window refuses, and a mistyped one could make it wait forever.
- Line breaks in a block's description showed as `\n`.
- One older test had been checking a stack of 65 blocks while it meant 64, and passed only because it never looked at what was left out. It now checks exactly 64.

## What does not work, or was never tried

**Never tried:**
- a Mac, Linux, an ARM processor;
- an AMD, Intel or Apple graphics card;
- a real graphics card whose limit for one draw's settings is below 64 KB (simulated with a 16 KB limit on the A4000);
- a mid-range laptop;
- the RTX 3090 computer, since Stage 2.

So the promise "Windows, Mac and Linux, on a mid-range laptop" is still a design, not a measurement. The Mac way of opening a block file with ChadFX has never been built.

**Seen only inside Claude's app:**
- The file association. Claude's app redirects registry writes, so Explorer itself has never been seen to show "ChadFX Block" files.
- A real sound device being unplugged or switched has never been tried.

**Known gaps that remain (small):**
- No end-to-end test installs, replaces or removes a sound block while the project plays. The player's own tests do, with its test doubles.
- A sound block that fails by returning an error code is tested in the block checker and the mixer only, not through the app, the command line or the window. It goes through the same fault path as a crash, which is tested at every level.
- When two copies of one block are both left out of a draw, the export summary gives two identical sentences, because a runtime problem does not carry its clip and track. The inspector places each note on its node.
- What a block's "not a number" colours become on screen is up to the graphics card.
- Uninstalling leaves one registry entry, `HKEY_CURRENT_USER\Software\vep\ChadFX`, which remembers the install folder.
- The window cannot yet put a generator clip on a track or create keyframes. The command line can, with hand-written commands, and the guide shows how.
- Notices sit over the right-hand corner of the timeline panel, on purpose. They can be scrolled and closed.
- Pressing Delete with only a whole track selected does nothing and says nothing.
- Tooltips that hold information can be read only with the mouse.
- Once a block is removed, the export dialog names it by its id, because the editor no longer knows its name.
- The window's title still says "Vep".

## What was measured

- **Sound blocks' cost** (release build, this computer), per second of sound, with volume, pan and EQ on each track:
  - 1 track: about 22 ms of one processor core;
  - 4 tracks: about 90 ms;
  - 8 tracks: about 180 ms, or 18% of one core; about 205 ms when clips are cut every 2 seconds.
- **While playing,** the player's sound thread used about 4 to 5%, 15 to 17% and 29% of one core for 1, 4 and 8 tracks, with a tone generator on each track. No frame was dropped, and sound started within 10, 60 and 90 ms.
- **A mid-range laptop is estimated, not measured,** at 35 to 50% of one core for 8 EQ tracks.
- **The work allowance.** Each sound block gets an allowance of work per 128-sample slice (called "fuel"): 612,000 units. The built-ins use:
  - volume and pan: about 1%;
  - crossfade: 2.4%;
  - EQ: 10.7%;
  - test tone: 19.7%.

  The worst a hostile sound block can take for one slice:
  - 0.4 ms with ordinary code;
  - 2.0 ms with functions holding 1,024 local variables;
  - 3.5 ms with arithmetic on "denormal" numbers, tiny numbers near zero that processors handle very slowly.

  Eight such hostile tracks run the sound 10 to 15 times slower than real time. The picture and the window carry on.
- **Settings per draw.** The editor's limit is now the smaller of 64 KB and the card's own limit. The A4000 and WARP both allow 64 KB. On a simulated card allowing 16 KB, the stack that would otherwise have failed the whole frame kept its picture: the blocks that do not fit are left out and named, and the frame was drawn in 37.5 ms.
- **Speeds** of the release command-line tool:
  - check, install or remove a block: 0.11 to 0.15 s;
  - draw one frame: about 0.9 s;
  - a 5-second H.264 export: 2.4 s;
  - a 3-second WAV export: 0.3 s.
- **The installer:** 8.6 MB, built in 4 minutes, installed in 1.1 seconds.
- **Hostile picture code:** 14 to 21 ms a frame at a small size; the costliest at 4K, 37 to 49 ms.

## Decisions only you can make

1. **Should a sound block's work allowance charge more for arithmetic on decimal numbers, or allow fewer local variables per function (256 instead of 1,024)?**
   - If yes: a hostile sound block's worst slice drops from about 3.5 ms toward 0.4 ms. You need both changes for a tight bound, and the built-ins' figures must be measured again. They still fit at up to 4 times the price; the test tone would then use 79% of its allowance.
   - If no: eight hostile tracks can keep the sound 10 to 15 times slower than real time, while the picture and the window carry on.
2. **Are the EQ's default settings right?** Today they are 0 dB on all three bands, with corners at 200 Hz and 4 kHz. Please listen and decide. Changing a default later changes existing projects that rely on it, unless a new settings format and a conversion are added.
3. **Should `vep` ship with the installer?** Today block authors must build it from the editor's source code with Rust, just to check and pack their blocks.
4. **Should uninstalling also remove the remembered install folder** (the registry entry above)?
   - If yes: nothing at all remains after an uninstall, and a reinstall offers the default folder.
   - If no: it stays as it is today.
5. **Should notices stay in the timeline panel's corner, or move to a list in the status bar?** In the status bar they would never cover anything.
6. **Should the editor force every colour to a finite value?** Today a block's "not a number" colours are drawn however the graphics card decides.
   - The cost is a little time per pixel.
   - It could change new blocks' pictures, never old projects' pictures.
7. **Should a sound block that crashes be turned off permanently, as picture blocks are?** Today it is only turned off until the project is opened again. Doing it permanently costs about 1.7 ms of disk writing on the sound thread each time a block crashes.
8. **Should the window's title say ChadFX instead of "Vep"?** The installer, the Start menu and "Open with" already say ChadFX.
9. **Should the window be able to add generator clips and edit keyframes?** Today only the command line can.
10. **GitHub says the repository has moved to `constantinemunoz/cfxcorewindows`.** The old address still works. Should the project switch to the new one?
11. **Can you arrange a Mac, a Linux computer, another graphics card and a mid-range laptop?** The portable picture proof and the laptop estimate are waiting for them.
12. **Two checks only you can do by hand:**
    - run the installer outside Claude's app, and see whether Explorer shows block files as "ChadFX Block" with ChadFX's icon;
    - unplug or switch a sound device while the editor plays.

## What happens next

When you have answered what you want to answer above, the next work is text and captions (your decision D3), and after that the defect review (D4). Finding B1 of the October review is still yours to schedule: the decoder's memory is limited by a count rather than by bytes, so 4K on a 16 GB computer fails, and it needs about a week.
