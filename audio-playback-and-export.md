# Sound effects and parameters today, and how preview and export depend on effect kinds

Scope: `crates/audio`, `crates/playback`, `crates/export`, `ui/src-tauri/src/player.rs`. I also read the model and registry code these call: `core-model` effect.rs, time_map.rs and timeline.rs, `effects-registry`, and `render` renderer.rs and graph.rs. Nothing in the project was changed.

How I checked:
- `cargo test -p audio` passes: 15 unit tests, 6 hostile, 1 memory, 1 processes, 3 retry, 3 reverse and 7 streaming tests. FFmpeg is present, so none were skipped.
- I wrote a small probe program against the real `audio::mix_range`. It lives outside the project, in my scratch folder: `docs/blocks-rewrite\mixprobe\src\main.rs`. Its results are marked **[measured]** below.

---

## 0. Summary of findings that matter most for the redesign

1. **Clip and track gain and pan are evaluated once per piece, at the piece's start, never per sample** (`crates/audio/src/mixer.rs:194,242-245`). So **keyframed gain or pan comes out as a staircase whose steps sit wherever the caller cut its mix windows**.
   - The player mixes 1/4-second windows starting from the playhead (`player.rs:648-659`).
   - Export mixes 1-second windows starting from the export range start (`export/src/audio_mix.rs:55-68`).
   - So **preview and export already differ** for keyframed gain and pan. **[measured]** A 0 to -20 dB ramp over 2 s:
     - in 1/4-second chunks it has 8 steps;
     - in 1-second chunks it has 2 steps;
     - mixed as one window it is a flat 1.0.
     - 144,000 of 192,000 samples differ between the two chunkings, by up to 0.578.
2. **There is no track bus and no sequence bus processing.**
   - Track nodes are folded into each clip's per-channel gain vector, evaluated at *clip-local* time (`mixer.rs:243`).
   - Pieces are added straight into the one output buffer (`mixer.rs:348-363`).
   - `Sequence.effects` (`core-model/src/timeline.rs:380`) is ignored by the mixer.
   - The only sequence-level step is a hard clamp to [-1, 1] (`mixer.rs:365-367`).
3. **Audio nodes that do nothing today:**
   - `AudioEq`, `AudioCompressor` and `AudioTimeStretch` (whose `preserve_pitch` setting is ignored; pitch follows speed, `audio/src/lib.rs:23-24`).
   - `TransitionKind` (every kind is a linear crossfade) and `Transition.params`.
   - `mask` and `blend` on audio nodes.
   - Speed ramps (the clip is silent).
4. **The mixer cannot report problems at all.** `mix_range` returns only `Vec<f32>`. Export reports sound trouble only as missing files (`export/src/lib.rs:117`).
5. **Preview and export call the same mixing function** (`audio::mix_range` with `StreamingAudioSource`, `player.rs:655`, `audio_mix.rs:68`). They differ in:
   - sample rate and channel count (sound device vs sequence);
   - where chunks start and how long they are;
   - NaN scrubbing, which happens in export only.

   Picture: both go through `render::layers_at` plus the same GPU path.
6. **Speed-changed clips depend on chunking** wherever a chunk's source sample count is not an exact integer ratio:
   - **[measured]** speed 0.7003: about 288,000 of 384,000 samples differ between chunkings;
   - speed 0.5: the last sample of every chunk is held instead of interpolated;
   - speeds 1.5 and 2.0 forwards, reversed clips, and dissolves are bit-identical across chunkings.

---

## 1. The mixer's signal path

### 1.1 Entry point and types

- `MixSpec { sample_rate: u32, channels: u16 }`, default 48000 and 2 (`mixer.rs:8-18`).
- `trait AudioSource { fn read(&mut self, clip, asset, source_time: RationalTime, frames: usize, spec) -> Option<Vec<f32>> }` (`mixer.rs:28-30`).
  - It returns interleaved f32.
  - A short answer or `None` becomes silence.
  - A negative `source_time` gives leading silence.
- `pub fn mix_range(state, seq, range: TimeRange, source: &mut dyn AudioSource, spec) -> Vec<f32>` (`mixer.rs:258-369`).
  - It is a pure function of its inputs plus the source's answers. **No state is kept between calls.**

### 1.2 Sequence level (`mix_range`)

```rust
// mixer.rs:259-268
let ch = spec.channels as usize;
if ch == 0 || spec.sample_rate == 0 { return Vec::new(); }
let total = range.duration().to_samples_round(spec.sample_rate).max(0) as usize;
... out.resize(total*ch, 0.0)   // try_reserve_exact, empty Vec if impossible
// mixer.rs:269-273  mute / solo
let soloed = seq.audio_tracks().any(|t| t.solo);
for track in seq.audio_tracks() {
    if track.muted || (soloed && !track.solo) { continue; }
```

- Only audio tracks are mixed (`seq.audio_tracks()`, `timeline.rs:446`).
- Each track's pieces are summed straight into `out` (`mixer.rs:348-363`).
- At the end:

```rust
// mixer.rs:365-367
for v in &mut out { *v = v.clamp(-1.0, 1.0); }
```

- There is no master gain or limiter, and `seq.effects` is never read.

### 1.3 Track level (inside `mix_range`)

For each enabled clip overlapping `range`, in clip order (`mixer.rs:275-347`):

- **Neighbours** (`mixer.rs:280-282`): `prev_adjacent` must end exactly at `clip.start()` and be enabled; `next_adjacent` must start exactly at `clip.end()` and be enabled.
- **Incoming transition** (`mixer.rs:286-312`):
  - The fade range is `[clip.start, clip.start + min(tr.duration, clip.duration))`.
  - Intersected with `range` it gives `fw`.
  - The clip's piece over `fw` is faded **up**. `prev_adjacent`'s piece over the same `fw`, which lies after prev's own end so prev is read past its out-point, is faded **down**.
  - `body` then starts at the fade end.
- **Outgoing transition** (`mixer.rs:314-341`), only when the next clip has no `transition_in`:
  - The fade range is `[clip.end - min(tr.duration, clip.duration), clip.end)`.
  - The clip is faded **down**. If `next_adjacent` exists, its piece over `fw` is faded **up**; that piece lies before next's start, so it reads source before next's in-point.
  - `body` is cut at the fade start.
- **Body** (`mixer.rs:342-346`): `clip_piece` over what remains.
- **Push order** within one clip is: incoming pieces, then outgoing pieces, then body (`mixer.rs:289-345`). **The outgoing fade piece is computed before the body piece even though it is later in time.** This matters for stateful blocks (§3.3).
- **Summing** (`mixer.rs:348-363`):

```rust
for piece in pieces {
    let frames = piece.samples.len() / ch;
    for i in 0..frames {
        let o = piece.start_frame + i;
        if o >= total { break; }
        let f = match piece.fade { Some(up) => ramp(i, frames, up), None => 1.0 };
        for c in 0..ch { out[o*ch+c] += piece.samples[i*ch+c] * piece.gains[c] * f; }
    }
}
```

- `piece.fade` is always `None` by the time pieces are summed. It is set to `Some(true)` and back to `None` at `mixer.rs:290,295`, and other pieces are built with `fade: None` (`mixer.rs:246`). So `ramp()` (`mixer.rs:170-173`) is **dead code** and `f == 1.0` always.
- The effective sum is `out += (s_faded * gains[c]) * 1.0`, left to right in f32.

### 1.4 Clip level (`clip_piece`, `mixer.rs:187-247`)

```rust
let start_frame = (window.start() - out_start).to_samples_round(spec.sample_rate).max(0) as usize;   // :189
let n = window.duration().to_samples_round(spec.sample_rate).max(0) as usize;                        // :190
if n == 0 { return None; }                                                                           // :191
let local = clip.to_local(window.start());   // window.start - clip.start                         // :194
```

**Source by kind:**
- **Generator `Tone`** (`mixer.rs:197-204`): `sin((t0 + i/sr) * 440 * TAU) as f32 * 0.25`, computed in f64. `t0` is the clip-local time in seconds, not the source in-point.
- **Any other generator, including `Silence`:** `None` (`mixer.rs:206`).
- **Asset** (`mixer.rs:208-240`):
  - `state.clip_asset(clip)?`: a missing asset entry gives silence, and nothing reports it.
  - `speed = time_map::constant_speed(clip)?`: a keyframed speed gives `None`, so the clip is **silent** (`mixer.rs:210-212`; `time_map.rs:101-107`).
  - `if !speed.is_positive()`: speed 0 is silent (`mixer.rs:213-216`).
  - `src_frames = round(n * speed).max(1)` (`mixer.rs:217`).
  - `reversed = time_map::is_reversed(clip)` (`mixer.rs:218`).
  - `src_t` (`mixer.rs:224-229`): when reversed, `source_time_at(clip, local + window.duration, None)`, the source time at the window's end; otherwise `source_time_at(clip, local, None)`. The formulas are in `time_map.rs:234-248`: forwards `in + t*s`, reversed `(in + d*s) - t*s`, exact rational.
  - `raw = source.read(clip, asset, src_t, src_frames, spec)`, resized to `src_frames*ch` with zeros (`mixer.rs:230-231`).
  - If reversed, `reverse_frames` (`mixer.rs:158-167`): frame order is flipped and channel order is kept.
  - If `speed != 1`, `resample(&raw, ch, n)` (`mixer.rs:235-239`).

**Gains** (`mixer.rs:242-245`):

```rust
let mut gains = stack_gain(&clip.effects, local, spec.channels);
for (g, t) in gains.iter_mut().zip(stack_gain(&track.effects, local, spec.channels)) { *g *= t; }
```

Both clip and track nodes are evaluated at the same clip-local `local`, the piece's start.

### 1.5 The maths, copied

```rust
// mixer.rs:98-100
fn db_to_gain(db: f32) -> f32 { 10f32.powf(db / 20.0) }

// mixer.rs:103-109  equal-power pan, stereo only
fn pan_gains(pan: f32, channels: u16) -> Vec<f32> {
    if channels != 2 { return vec![1.0; channels as usize]; }
    let theta = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    vec![theta.cos() * std::f32::consts::SQRT_2, theta.sin() * std::f32::consts::SQRT_2]
}

// mixer.rs:112-133
fn stack_gain(effects, t, channels) -> Vec<f32> {
    let mut gains = vec![1.0f32; channels as usize];
    for e in effects.iter().filter(|e| e.enabled) {
        match e.kind {
            EffectKind::AudioGain => {
                let db = e.param_value_at("gain_db", t).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let g = db_to_gain(db);
                for x in &mut gains { *x *= g; }
            }
            EffectKind::AudioPan => {
                let pan = e.param_value_at("pan", t).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                for (x, p) in gains.iter_mut().zip(pan_gains(pan, channels)) { *x *= p; }
            }
            _ => {}
        }
    }
    gains
}

// mixer.rs:136-154  linear resampling, per piece, clamped at the piece's last input frame
for i in 0..out_frames {
    let pos = i as f64 * (in_frames as f64 / out_frames as f64);
    let i0 = (pos.floor() as usize).min(in_frames - 1);
    let i1 = (i0 + 1).min(in_frames - 1);
    let f = (pos - i0 as f64) as f32;
    for c in 0..channels { out.push(a + (b - a) * f); }
}

// mixer.rs:373-383  transition fade, applied to the piece's samples in place, before the gains
fn apply_partial_fade(samples, ch, n, offset, full, up) {
    for i in 0..n {
        let u = ((offset + i) as f32 / full.max(1) as f32).clamp(0.0, 1.0);
        let g = if up { u } else { 1.0 - u };
        for c in 0..ch { if let Some(s) = samples.get_mut(i*ch+c) { *s *= g; } }
    }
}
```

- The fade arguments (`mixer.rs:291-293` and repeated at 300-302, 320-322, 328-330) are `n = round(fw.duration*sr)`, `full = round(fade_range.duration*sr).max(1)`, `offset = round((fw.start - fade_range.start)*sr)`. Because the fade is measured from the fade start, it is independent of chunking.
- **[measured]** A fade-out's last sample is `1/full`, not 0 (4.17e-5 for a 0.5 s fade). A fade-in starts at 0 and ends at `(full-1)/full`. A dissolve of two equal signals sums to exactly 1.0.
- **Pan:** pan 0 gives `[0.99999994, 0.99999994]` (`0x3f7fffff`), **not 1.0**. So **an `AudioPan` node at 0 changes every sample by 1 ulp compared with no node [measured]**. Pan +1 leaves `-6.2e-8` on the left. Pan ±1 gives `1.41421354` on the panned side.
- **Gain:** `gain_db` is not clamped to the registry's -96 to +24 range (`effects-registry/src/lib.rs:216-218`); only the final [-1, 1] clamp applies. **[measured]** +12 dB on 0.5 becomes 1.0.

### 1.6 How keyframes are interpolated

- `EffectNode::param_value_at` (`effect.rs:546-548`) calls `Param::value_at(t)` (`effect.rs:385-416`):
  - before the first key or after the last, the value holds;
  - otherwise a binary search finds the segment, then `u = seconds_between(a, t) / span` in f64.
  - The segment's interpolation is `Hold` (keep a), `Linear` (`lerp_value`, `effect.rs:431-446`) or `Bezier` (`bezier_ease`: Newton then bisection, `effect.rs:450-484`).
- The value is converted f64 to f32 in `stack_gain`.
- Keyframe time is "clip-local (0 = clip start)" (`effect.rs:323`). Track nodes are evaluated at the same clip-local time, which matches the video renderer (`render/src/graph.rs:228`).
- **[measured]** A track gain keyframed 0 to -20 dB over 2 s, across two 1-second clips, gives 0 dB everywhere: each clip evaluates it at its own local times 0 to 1.

### 1.7 Each sound-related feature: what happens and how

| Feature | Behaviour | When evaluated | Where |
|---|---|---|---|
| `AudioGain` `gain_db` (default 0) | ×10^(dB/20), all channels | once per piece, at the piece's start in clip-local time | `mixer.rs:116-122` |
| `AudioPan` `pan` (default 0) | equal-power ×√2; stereo only, other layouts get 1.0 | once per piece | `mixer.rs:123-128,103-109` |
| `AudioEq` | **nothing** | – | `mixer.rs:129` (`_ => {}`) |
| `AudioCompressor` | **nothing** | – | `mixer.rs:129` |
| `AudioTimeStretch` (`preserve_pitch`) | **nothing**; pitch follows speed | – | `mixer.rs:129`; `audio/src/lib.rs:23-24` |
| `TimeRemap` constant speed | read `round(n*speed)` source frames, then linear resample to `n` | per piece; source position exact (rational) | `mixer.rs:212-239` |
| `TimeRemap` reverse | decode the stretch forwards, flip the frames | per piece | `mixer.rs:218-234` |
| `TimeRemap` keyframed speed (ramp) | **silent** | – | `mixer.rs:210-212` |
| speed ≤ 0 | silent | – | `mixer.rs:213-216` |
| Dissolve (`transition_in` / `transition_out`, any `TransitionKind`) | linear ramp on samples, per sample; kind and params ignored | per sample, offset from the fade start | `mixer.rs:286-341,373-383` |
| Mute / solo | track skipped; solo of audio tracks only | per call | `mixer.rs:269-273` |
| Disabled clip or node | skipped | – | `mixer.rs:276-278,114` |
| Node `mask` / `blend` | ignored | – | `stack_gain` never reads them |
| Track effects | folded into the clip's gains (`g_clip * g_track`) | at clip-local time | `mixer.rs:242-245` |
| Sequence effects | **ignored** | – | (no reference) |
| Final clamp | hard clip to ±1 | per sample | `mixer.rs:365-367` |

Registry entries for the audio kinds: `effects-registry/src/lib.rs:216-238`; `TimeRemap` at 186-190.

---

## 2. Formats, chunk sizes, sample rates, and where sample identity is guaranteed or tested

### 2.1 Format

**Sample format and layout:**
- Interleaved `f32` everywhere: what sources return (`mixer.rs:28-30`), the mix (`mixer.rs:264`), and the export temporary file written as f32le (`audio_mix.rs:73-76`).
- FFmpeg delivers `aresample,aformat=sample_fmts=flt:sample_rates={rate}:channel_layouts={channels}c` as `-f f32le -acodec pcm_f32le` (`media/src/audio.rs:333,426`).
- Up-mixing and down-mixing is FFmpeg's; stereo to mono sums × 0.707 (`media/src/audio.rs:120-122`).

**Where samples land in time:**
- `start_frame = round(...)` and `n = round(...)` (`mixer.rs:189-190`). Rounding is `floor(x*sr + 1/2)` (`core-time/src/rational.rs:321-325`).
- The source's first sample is `ceil(source_time*sr)` (`sample_index`, `mixer.rs:35-40`; also `streaming.rs:592`).

**Export:**
- Mix rate is `seq.audio_sample_rate` when it is within 8 kHz to 192 kHz, otherwise 48 kHz (`export/src/plan.rs:190`).
- Channels are always `CHANNELS = 2` (`plan.rs:184`; `lib.rs:121`).
- Chunks are `sample_rate` frames, i.e. 1 s, counted from `timing.start` (`audio_mix.rs:55,63-65`).
- Each chunk is padded or truncated to its exact length (`audio_mix.rs:71`). NaN becomes 0 and values are clamped (`audio_mix.rs:74`).
- The total sample count is `round(duration*rate)` (`plan.rs:45-47`).
- Encoding: FFmpeg reads `-f f32le -ar mix -ac 2` (`plan.rs:210-212`) and writes AAC, `pcm_s16le` or `pcm_s24le`. AAC falls back to 48 kHz for rates it cannot carry (`plan.rs:14,191-193`).

**Preview** (`ui/src-tauri/src/player.rs`):
- The device is opened with `AudioOutput::open(48000, 2)` (`player.rs:853`). The device's default configuration is used if no f32 configuration fits (`audio/src/output.rs:32-37`); a non-f32 device means no sound at all (`output.rs:38-40`).
- The mix spec is the device's actual rate and channel count (`player.rs:854`). **The sequence's `audio_sample_rate` is ignored in preview.**
- Chunks are `1/4 s` starting at `audio_pos` (`player.rs:648-659`). `audio_pos` is reset to `transport.position` by `anchor_here` (`player.rs:380-387`) on Play, Pause, Seek, Step, rate change, Load and Update (`player.rs:391-496,590`).
  - A seek snaps to a frame (`playback/src/transport.rs:43-46`); at 29.97 fps a frame boundary is off the 48 kHz sample grid.
  - After pause and resume the position is the anchor plus whole microseconds (`player.rs:814-821`), so it is generally off the sample grid.
- The mix runs on the player thread, not in the sound-card callback (`player.rs:606-660`). The callback only drains the queue (`output.rs:52-74`).
- The queue holds 2 s (`output.rs:43`) and is refilled while free space exceeds two chunks (`player.rs:652`).
- `push`'s return value is ignored (`player.rs:656`). There is no NaN scrubbing.
- Sound plays only at rate 1.0 (`player.rs:613,644`).
- A scratch (uncommitted) state is mixed from the next chunk on, with no re-anchor (`player.rs:410-415,650`).

**Sources at other rates:** FFmpeg converts the rate continuously per decoder (`streaming.rs:235-245`).

**Speed:** the mixer's own linear resampler (`mixer.rs:136-154`); there is no `rubato`.

### 2.2 Buffer sizes the mixer works in

- Pieces of `n = round(window.duration*sr)` frames, from 1 up to a whole chunk. The window is the mix window intersected with the clip range (plus transition handles) and cut at fade boundaries.
- Player: 12,000 frames per 1/4-second chunk at 48 kHz.
- Export: 48,000 frames per chunk; the last chunk is shorter.
- Tests call arbitrary whole ranges.

### 2.3 Where sample identity is guaranteed or tested

Every comparison of chunked against unchunked output in the tests uses clips **without keyframed gain or pan**.

| Test | What is compared | Exact? |
|---|---|---|
| `mixer.rs:585-597` `chunks_of_a_reversed_clip_join_without_a_seam` | whole vs 1/4-second chunks, reversed clip, synthetic source | bit-exact |
| `export/src/audio_mix.rs:112-136` `chunked_mix_equals_one_big_mix` | `mix_to_file` vs one `mix_range`, `SineSource`, off-grid start | **tolerance < 0.04** |
| `audio_mix.rs:160-207` `real_files_are_mixed_in_chunks_without_seams_by_one_decoder_each` | 1-second chunks with `StreamingAudioSource` vs whole range with `MediaAudioSource`; WAV, MP4, 44.1 kHz AAC; three starts | **0 differing samples** (no effects) |
| `audio/tests/streaming.rs:52-60,62-110` | each streaming request vs reference; chunks vs one `decode_audio_all` | bit-exact |
| `streaming.rs:318-366` `speed_changed_clips_mix_exactly...` | streaming vs reference **with the same 1/4-second chunking**, speeds 0.3 to 2.9999 | bit-exact (does not compare chunkings) |
| `streaming.rs:368-395` reversed clip | equals the file's stretch reversed | bit-exact |
| `audio/tests/hostile.rs:288-347` | reversed and dissolving clips **plus static gain -6 dB and pan 0.5** (`:319-320`), streaming vs reference, same windows | bit-exact; **the only real-file test with gain or pan in a mix** |
| `hostile.rs:227-281` | 44.1 kHz and Matroska: chunks equal one unbroken decode; reference only within 0.04 | exact / tolerance |
| `audio/tests/reverse.rs:64-83` | per-chunk equality of the two sources | bit-exact |
| `mixer.rs:446-470` `gain_pan_mute_and_solo` | RMS within 0.01 | tolerance |
| `export/tests/export.rs:1024-1062` `pcm_sound_is_sample_exact_and_seamless` | onset ±1 sample, seam bend, peak ±0.005 | not bitwise |
| `export/tests/export.rs:948-964` WAV range between frames | onset ±1 sample, seam check | not bitwise |
| `export/tests/review.rs:378-403,1323-1382` | onset ±1 sample; ≤ 0.02 against FFmpeg's own decode | tolerance |

**Not tested anywhere:**
- preview chunking against export chunking;
- keyframed gain or pan;
- track gain or pan;
- a pan node at 0;
- sample rates other than 48 kHz in the mixer itself;
- channel counts other than 2 (except mono reads in `hostile.rs:361-377`).

**[measured] Identity across chunkings:**
- Speed 1 with an off-grid player start equals the export exactly once aligned to the sample at or after the start (the ceil rule); it is one sample later than rounding would give.
- Speeds 1.5 and 2.0 reversed, 1.0 reversed, and dissolves are exact across whole, export and player chunkings.
- Speed 0.5 differs at chunk ends: 6 samples between whole and export, 16 between export and player.
- Speed 0.7003 differs almost everywhere (287,994 of 384,000 samples, up to 0.0175).

---

## 3. Where a sandboxed audio block would plug in

### 3.1 Insertion points in today's code

- **Per-clip chain:** inside `clip_piece`, after time mapping, i.e. after reverse and resample (`mixer.rs:241`) and before the gains are built (`mixer.rs:242`).
  - Today the transition fade is applied after `clip_piece` returns, to the samples (`mixer.rs:294,303,323,331`), and the gains are applied at the sum (`mixer.rs:360`).
  - The actual order is: source → reverse → resample → ×fade → ×(g_clip·g_track) → sum.
- **Per-track chain:** there is no track buffer. A real track chain (a compressor, say) needs a new track buffer: sum the pieces into it at `mixer.rs:348-363`, run the chain, then add it to `out`.
  - For gain and pan this changes float rounding:
    - `(s·g_c)·g_t` instead of `s·(g_c·g_t)`;
    - `(a+b)·g_t` instead of `a·g_t + b·g_t` during dissolves.
  - It also changes keyframe time from clip-local to whatever the track uses.
  - **For bit-identity, today's track `AudioGain`/`AudioPan` must keep being folded per clip at clip-local time**, either as a legacy path or by migrating them onto the clips.
- **Per-sequence chain:** after the track loop, before the clamp (`mixer.rs:364-365`). Export's second clamp and NaN scrub (`audio_mix.rs:74`) runs after that; preview has none.

### 3.2 What a block would see

- **Buffers:** interleaved f32 of any length from 1 frame (a piece of length 0 is skipped, `mixer.rs:191`) up to the chunk: 12,000 frames in the player at 48 kHz, `sample_rate` frames in export, or the whole range when called directly.
- **Sample rate:** export uses 8 kHz to 192 kHz. Preview uses whatever the device gives.
- **Channels:** export always 2; preview uses the device's count, which can be 1 or 6, and pan does nothing then.
- **Thread:** blocks run on the player thread (`player.rs:606-660`) or the export thread. The real-time budget is the ~1.5 to 2 s queue, not the sound-card callback.

### 3.3 State: filter memory across chunks and seeks

- Today the mixer has none: every call is from scratch, which is why linear static effects do not depend on chunking.
- The only state is the source's decoder cache (`streaming.rs:285-303`), cleared by `close_all` on Unload and ResetMedia (`player.rs:400-402,549-551`).
- A stateful block would need state keyed at least by (sequence, track, clip, node id), and for track and sequence chains by track or sequence.

**Problems in today's structure:**
1. **Pieces are not in time order.** A clip's outgoing-fade piece is processed before its body in the same call (`mixer.rs:314-345`). The previous clip's tail under an incoming dissolve is a separate piece produced in the *next* clip's iteration (`mixer.rs:298-306`). The fix: per clip, build one contiguous buffer for the clip's whole span in the window, including transition handles; run the chain over it in time order; then apply fades as envelopes.
2. **Discontinuities.**
   - Player: every `anchor_here` (Play, Pause, Seek, Step, rate change, Load and Update, `player.rs:380-387,590`) restarts mixing at an arbitrary position. Scratch edits keep the queued audio (`player.rs:410-415`).
   - Export: starts cold at the range start (`audio_mix.rs:59-65`).
   - Making preview, export and any chunking agree for IIR or envelope blocks needs one of:
     - (a) every block declares a warm-up length, and the host always pre-rolls `min(warmup, t - clip_start)` from a fresh state at every discontinuity *and* at every chunk start, so the result is deterministic and chunk-independent but costs more work;
     - (b) chunk independence is defined only within a tolerance.

     Continuous state with cold restarts is bit-identical only for finite-memory blocks.
3. **Tails and latency.** Pieces stop at the clip range plus transition handles, so a reverb tail would be cut off. There is no latency compensation anywhere. Blocks would need to declare their tail and latency, and the mixer would extend pieces by the tail.

### 3.4 Handing keyframed parameters to a block so gain and pan come out as today

What today's semantics require:

1. **Value:** `Param::value_at(t_local)` evaluated in the host (`effect.rs:385-416`, f64 including `bezier_ease`), where `t_local = piece.start - clip.start` for **both** clip and track nodes. A missing or non-numeric value means 0.0. Then converted to f32.
2. **One constant per piece**, over the same pieces as today. Those depend on the caller's window boundaries and on fade boundaries.
   - **[measured]** One window from 0 to 3 s with a keyframed gain and a dissolve: clip A's body uses its local-0 value (1.0), and A's fade-out piece uses its local-2 s value (0.1). The jump falls inside one window.
3. **Coefficients:** `10f32.powf(db/20.0)`; pan `θ=(clamp(p)+1)·FRAC_PI_4` in f32 with `cos θ·√2` and `sin θ·√2`; channel counts other than 2 get 1.0.
4. **Accumulation order:** start with `[1.0; ch]`, multiply in each enabled node in stack order, separately for clip and track, then `g_clip[c]·g_track[c]`. The sample is `(s·fade)·g[c]·1.0`, summed into `out` in this order: tracks in order, then clips in order, then pieces in push order. Finally clamp.
5. **Determinism inside WASM:**
   - f32 multiply and add are IEEE-identical in WASM.
   - `powf`, `sin` and `cos` are not WASM instructions; a block brings its own maths library. Today's native code uses the platform's (MSVC C runtime) functions, so **a gain or pan block computing its own coefficients in WASM is not guaranteed bit-identical**; this has to be measured. The safe options are host-computed coefficients, or keeping gain and pan native.
   - Wasmtime offers `relaxed_simd_deterministic` (or turning relaxed SIMD off) and `cranelift_nan_canonicalization` for reproducible float results. Its own documentation notes that NaN canonicalization covers only a list of opcodes.
6. **The keyframed design choice:**
   - A bug-compatible "one value per host-defined buffer" API reproduces today *only for the same chunking*. Preview and export already disagree (§0.1).
   - Per-sample automation (an array, or start value plus slope per buffer) would be chunk-independent and give preview equal to export, but **changes output** for every project with keyframed gain or pan.
   - Static gain and pan can be reproduced exactly either way, given (4) and (5).

---

## 4. How export reports missing media, and whether preview and export share code

### 4.1 Today's reporting

**Static scan** — `find_missing_media(state, seq, range, picture, sound)` (`export/src/export.rs:293-311`):
- Only tracks that take part count: their kind is being exported, they are not muted, and they are not silenced by a solo on another track of the same kind.
- Only enabled clips whose `timeline_range` overlaps the range count. Transition handles beyond a clip's range are not considered.
- Paths of assets missing on disk go into a `BTreeSet`.
- The result lands in `ExportReport.missing_media: Vec<String>` (`export.rs:135-137`, set at 444/526).
- Test: `export/tests/review.rs:1126-1174`.

**Runtime counting** (picture only):
- `playback::MediaFrameSource::note` aggregates per (asset, stream, kind): `count`, `first_time`, `last_time`, and the first message (`playback/src/frames.rs:783-791`).
- The types are `FrameErrorKind` and `FrameError` (`frames.rs:113-157`), drained by `take_errors` (`frames.rs:759-761`).
- `ExportFrameSource` counts `blank_layers` whenever a frame request returns `None` (`export/src/frames.rs:145-153`).
- `problems_from` maps errors to `MediaProblem` and `MediaProblemKind`, including `delivered_a_picture()` (`export/src/frames.rs:13-96`).
- They land in `VideoReport.blank_layers` and `.media_problems` (`export.rs:85-99`, filled at 628, 511-512), and in `StillReport.media_problems` (`still.rs:45-50,86`). A still has no `missing_media`.

**Where it surfaces:**
- UI wire types: `ui/src-tauri/src/wire.rs:690-718,743-762,807-813,856-862`.
- CLI warnings: `crates/cli/src/export_cmd.rs:345-360`.
- Player: `take_media_errors` and `PlayerNotice::MediaErrors`, capped at 256 (`player.rs:282-285,735-744`).

**Sound:** no runtime reporting. `mix_range` has no error channel, `AudioSource::read` returns only `Option`, and `StreamingAudioSource` has no `take_errors`. `export/src/lib.rs:117` says "Sound that could not be decoded is not reported beyond missing files."

**A "missing block" pattern following these:**
1. A static `missing_blocks` list in `ExportReport` (and `StillReport`), produced like `find_missing_media`. It would walk the clip, track and sequence nodes of tracks that take part and clips that overlap, include sequence-level nodes, and probably transition handles.
2. A runtime aggregate like `FrameError`, keyed by (node id or block id, target), with `count`, `first_time`, `last_time` and `message`. This needs the mixer to gain a problem sink or become a stateful object, because `mix_range` cannot report anything today.
3. The audio path already *renders without* unknown kinds (`mixer.rs:129` `_ => {}`).
4. Note that `EffectKind` is a closed serde enum (`core-model/src/effect.rs:11-30`), so an unknown kind cannot be stored or read today. Keeping a missing node byte for byte is a model change outside these crates.

### 4.2 Do preview and export share code?

**Mixing — yes:**
- Both call `audio::mix_range` with `StreamingAudioSource`: player at `player.rs:655,861`, export at `audio_mix.rs:68` and `export.rs:448`, with max decoders `max(12, 2·tracks+2)` (`audio_mix.rs:27-34`).
- They differ in:
  - spec (device vs sequence rate; device channels vs 2);
  - chunk grid (1/4 s from the playhead vs 1 s from the range start);
  - export's NaN scrub and second clamp (`audio_mix.rs:74`);
  - preview mixing only at rate 1.0;
  - preview mixing the scratch state.

**Rendering — yes:**
- All three use `render::layers_at` (`render/src/graph.rs:284`), then `check` and `start`, then `finish`:
  - preview: `Renderer::render_into` (`renderer.rs:326-335`; `player.rs:692`);
  - export: `layers_at` plus `submit_plan` and `collect_into` (`export.rs:596-609`; `renderer.rs:395-428`), whose comment says it is the same as `render_into`;
  - still: `render` (`still.rs:85`; `renderer.rs:313-317`).
- They differ in:
  - frame source: preview decodes at a reduced width and uses proxies (`player.rs:668-679,860`); export uses full size and never proxies (`export/src/frames.rs:127-129`);
  - output width;
  - frame cache size.

---

## 5. Every place in the four target areas that knows an effect kind or parameter

**`crates/audio/src/mixer.rs`**
- `3`: imports `time_map` and `EffectKind`.
- `98-109`: dB and pan maths.
- `112-133` `stack_gain`:
  - `.filter(|e| e.enabled)` (114);
  - `EffectKind::AudioGain` / `"gain_db"`, default 0.0 (116-122);
  - `EffectKind::AudioPan` / `"pan"`, default 0.0 (123-128);
  - every other kind ignored (129).
- `196-207`: `GeneratorKind::Tone` synthesised at 440 Hz × 0.25; other generators silent.
- `210-212`: `time_map::constant_speed` reads `TimeRemap` `"speed"`; animated means silent.
- `213-216`: speed ≤ 0 is silent.
- `217`: source frames from speed.
- `218`: `time_map::is_reversed` reads `TimeRemap` `"reverse"`.
- `224-229`: `time_map::source_time_at`.
- `235-239`: skips resampling when speed is 1.
- `242-245`: clip nodes times track nodes.
- `269-273`: `muted` / `solo`.
- `276-278,281-282`: `enabled` on clips and neighbours.
- `286-312,314-341`: `transition_in` / `transition_out` and `duration`; `TransitionKind` and `params` not read.
- Tests: `450-451` (gain and pan), `484,549,652` (`constant_speed_node`), `498,511` (CrossDissolve), `639-642` (`TimeRemap` keyframed speed, `SPEED_PARAM`, `REVERSE_PARAM`).

**`crates/audio/src/lib.rs`**
- `3-6`: documentation listing speed, reverse, gain, pan, mute/solo and dissolves.
- `23-24`: ramps silent, pitch follows speed.

**`crates/audio/src/streaming.rs`**
- No effect kinds.
- Its backward-reading and kept-frames logic exists *because* of reverse and speed (`KEPT_FRAMES` 32-38, block decoding 46-58, `note_request` 467-481, `read_block` 501-565). It infers direction from the pattern of requests, not from effect nodes.

**`crates/audio/src/output.rs`**
- None.

**`crates/audio/tests`**
- `hostile.rs:292,302,312,315,319-320`: speed, reverse, CrossDissolve, `AudioGain -6`, `AudioPan 0.5`.
- `processes.rs:15,115,120`.
- `reverse.rs:12,57`.
- `streaming.rs:10,345,356,358,376,391`.
- `memory.rs` and `retry.rs`: none.

**`crates/playback/src`** (`lib.rs`, `frames.rs`, `cache.rs`, `transport.rs`)
- **None.**
- `FrameSource::frame` takes a source time and knows nothing of effects (`frames.rs:1534-1619`).
- Reverse is handled by a request-pattern heuristic (`lib.rs:13-16`).

**`crates/export/src`**
- **No effect kinds.**
- `export.rs:293-311` knows `muted`, `solo` and `clip.enabled`.
- Everything else delegates to `render::layers_at`, which does know video kinds (out of scope; `graph.rs:228,331`), and to `audio::mix_range`.
- `lib.rs:109-117,121`: documentation.
- `settings.rs`, `plan.rs`, `output.rs`, `error.rs`, `feed.rs`, `still.rs`: none.

**`crates/export/tests`**
- `export.rs:140` and `review.rs:599`: `EffectKind::Transform` keyframes (video).
- No audio effect nodes in the export tests.

**`ui/src-tauri/src/player.rs`**
- **No effect kinds.**
- `22-23,125-126,613,644`: transport rate (sound only at 1.0). This is about the transport, not effects.

---

## 6. Making sample-exact golden references of today's output before the rewrite

**Method**
- Capture at the `audio::mix_range` level, not through an encode. Exports are only AAC, 16-bit or 24-bit integer PCM (`plan.rs:198-206`), so they are lossy relative to f32.
- Store raw f32 little-endian files (1 to 4 s of stereo at 48 kHz is 384 KB to 1.5 MB), or a blake3 hash of the `to_bits()` stream plus a few spot samples. blake3 is already a workspace dependency (`Cargo.toml`).
- Store each fixture as a schema-v2 project file as well, so the migration test can do: load v2 → migrate → mix → compare bits.

**Three chunkings per case**, since output depends on chunking:
- (a) one whole-range call;
- (b) the **export pattern**: copy `mix_to_file`'s loop (`audio_mix.rs:59-80`, 1-second windows from the range start, the `resize` at 71, the clamp and NaN scrub at 74) — `mix_to_file` itself is `pub(crate)`;
- (c) the **player pattern**: 1/4-second windows from a start on the frame grid and from an off-grid start such as 1.234567 s (`player.rs:648-659`).
- Record the platform. Gains use the native f32 `powf`, `sin` and `cos`; the Tone generator uses f64 `sin` (`mixer.rs:201`).

**Sources**
- A deterministic synthetic source that mimics a decoder: sample `k = ceil(t·sr) + i` maps to `f(k)`, with silence for `k < 0`. My probe's `GridSource` does this. It makes the goldens independent of FFmpeg.
- One set through `StreamingAudioSource` on an FFmpeg-generated "glide" WAV (as in `audio/tests/streaming.rs:29-37`) to lock down decoding too. Use a constant source to read gain values off directly.

**Cases**
- **Gain:** static gain -6.0206, +6, 0, -96 and +24 dB (the last reaches the clamp).
- **Pan:** -1, -0.5, 0, 0.5 and 1, plus a pan node at 0 against no node (the 1-ulp difference).
- **Stacking:** gain then pan and the reverse order; duplicate nodes; disabled nodes; clip and track nodes together.
- **Keyframed gain:** Linear, Hold and Bezier, on a clip and on a track. These are chunk-dependent, so store all three chunkings and mark which one the new engine must match.
- **Keyframed pan.**
- **Dissolves:**
  - `transition_in` with and without a previous clip;
  - `transition_out` with and without a next clip, and with a next clip that has its own incoming transition;
  - a transition longer than the clip;
  - a dissolve with keyframed gain (the jump inside one window);
  - a non-CrossDissolve kind (same output).
- **Speed:** 0.5, 1.5, 2, 0.7003 and 1.0001, each forwards and reversed. A ramp (silent) and speed 0 (silent).
- **Mute and solo;** the Tone generator; clips that start before their file (negative source time); two tracks summing over 1 (clamp).
- **Nodes that do nothing:** EQ, compressor and time stretch present; a sequence-level `AudioGain`.
- **Specs:** 44.1 kHz, mono, and 6 channels (pan ignored).

**Probe results to use as cross-checks** (my scratch probe; values printed exactly)
- Pan gains:
  - pan(0) = 0.99999994 (`0x3f7fffff`);
  - pan(-0.5) = [1.306562901, 0.541196108];
  - pan(1) = [-6.2e-8, 1.414213538].
- A 0 to -20 dB linear gain ramp over 2 s:
  - in 1/4-second chunks: 1.0, 0.7498942, 0.56234133, 0.4216965, 0.31622776, 0.23713738, 0.17782794, 0.13335215;
  - in 1-second chunks: 1.0, 0.31622776;
  - as one window: 1.0.
- A fade-out's last sample is `1/full`.
- An equal-signal dissolve sums to exactly 1.0.

Sources:
- [Wasmtime: Deterministic Wasm Execution](https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html)
- [Wasmtime Config docs (mirror)](https://docs.subsocial.network/rust-docs/latest/wasmtime/struct.Config.html)
- [Wasmtime issue #8145, NaN canonicalization coverage](https://bytecodealliance.github.io/zulip-archive/stream/225357-git-wasmtime/topic/wasmtime.20.2F.20issue.20.238145.20Cranelift.3A.20inconsistent.20NaN.20canoni.2E.2E.2E.html)
- [Zulip: Determinism of Wasm SIMD in Wasmtime](https://bytecodealliance.github.io/zulip-archive/stream/206238-general/topic/Determinism.20of.20Wasm.20SIMD.20in.20Wasmtime.html)