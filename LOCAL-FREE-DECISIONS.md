# Open questions, resolved the free and local way

Every item below was flagged in `ARCHITECTURE.md` as unresolved or risky. The constraint applied to all of them: **nothing paid, nothing in the cloud, everything on the user's machine**. Where two options were both free, the simpler one won.

| Question | Decision | Why it is the easiest free option |
|---|---|---|
| Preview surface in the Tauri window (native child window vs. pixels pushed to the webview) | **Start with readback-to-canvas.** The renderer draws into a wgpu texture, copies it to CPU memory, and the UI paints it on a `<canvas>` via a Tauri custom protocol or shared buffer. The native child surface stays as a later optimization behind a feature flag. | Zero platform-specific window code on Windows, macOS, or Linux. At preview resolution (1080p or below) the copy is a few hundred MB/s, well within budget. Switching later changes only the preview widget. |
| Hardware video decode | **Software decode through FFmpeg plus proxies by default.** FFmpeg's built-in hardware accelerators (D3D11VA/DXVA2 on Windows, VideoToolbox on macOS, VAAPI on Linux) are free and get enabled later with CPU readback; zero-copy GPU interop is deferred. | Proxies make 4K editing smooth on any machine with no GPU-specific code. Everything is already inside FFmpeg; no extra SDKs or licenses. |
| FFmpeg Rust bindings versus wrapping ourselves | **`ffmpeg-sys-next` raw bindings with our own thin, safe wrapper for decode and seek; the `ffmpeg` and `ffprobe` command-line tools for probing, proxies, and bulk transcodes.** | The CLI tools are the most battle-tested path and cost nothing to integrate. In-process bindings are used only where frame-accurate control is mandatory (playback decode). |
| FFmpeg build and licensing | **Prebuilt LGPL shared libraries from the BtbN FFmpeg-Builds GitHub releases (free), dynamically linked.** Encoding uses the free hardware encoders (NVENC, AMF, QSV, VideoToolbox) with OpenH264 (BSD, Cisco-covered) as the software H.264 fallback, plus libvpx/libaom/SVT-AV1 for VP9 and AV1. x264/x265 (GPL) are excluded unless the whole app is released under the GPL. | No build toolchain for FFmpeg, no license fees, and the app can stay closed or open source as the user chooses. |
| Python distribution to end users | **`uv` with `python-build-standalone`.** The app ships a lock file; on first run it calls `uv sync`, which downloads a self-contained Python and the pinned packages into the app's data folder. No system Python is required. | `uv` is free, cross-platform, and reproducible. It avoids freezing tools like PyInstaller that break with PyTorch. |
| Model weights | **Downloaded on demand from Hugging Face (free) into the local cache; Ollama manages LLM and VLM weights the same way.** A model manager shows size and progress and never downloads without the user starting a feature that needs it. | All candidate models (SigLIP-2, V-JEPA-2, Qwen3-VL, faster-whisper, SAM 2.1, SEA-RAFT, RIFE, Demucs, DeepFilterNet) have free weights. |
| Cloud model backends | **None.** The `ai-llm` backend trait keeps an OpenAI-compatible client so a local llama.cpp or vLLM server works, but no hosted API is wired up and no key storage exists. | Removes the cost, privacy, and consent work entirely. Adding a hosted backend later is additive. |
| Fast visual judge compute | **Batched local inference in the Python model service on the user's GPU, CPU fallback with ONNX Runtime.** | Free, and far faster than routing images through an HTTP LLM server. |
| Database and cache | **SQLite project file, plain files in the user's cache directory, Arrow IPC for per-frame tables.** | All embedded and free; no services to run. |
| Rust toolchain on Windows | **rustup plus Microsoft C++ Build Tools (both free, both installed on this machine on 2026-10-01).** | The standard MSVC path keeps Tauri, wgpu, and FFmpeg bindings on their best-supported target. |

## What was actually done in Phase 1 (2026-10-02)

- FFmpeg: the free LGPL shared build installed through the Windows package manager (`BtbN.FFmpeg.LGPL.Shared`). Decoding runs entirely through the `ffmpeg` and `ffprobe` executables as child processes; no bindings were compiled at all, which turned out to be the easiest path and is fast enough for real-time preview thanks to sequential decoding.
- Node.js: the official portable zip (no administrator rights needed) under `%LOCALAPPDATA%\Programs\node\current`. The installer version was declined at its administrator prompt, so the portable copy is what the project uses.
- GPU: wgpu picked Vulkan on the NVIDIA RTX A4000 automatically; the software fallback adapter is used when no GPU is present.
- Audio: the default output device through cpal, with no extra drivers.

## What was actually done in Phase 2 (2026-10-09)

- **Export encoders.** Still FFmpeg as a child process, with no bindings. An encoder is used only after a real test encode succeeds on the machine. Hardware encoders (NVIDIA, Intel Quick Sync, AMD, Apple) are tried first, then the free software ones: libopenh264 for H.264, SVT-AV1 and libaom for AV1, libvpx for VP9, and FFmpeg's own ProRes and DNxHR encoders. x264 and x265 are used only if the user's own FFmpeg happens to include them.
- **On the development machine** the NVIDIA encoder is rejected because the installed driver (596) is older than this FFmpeg build needs (610 or newer). Intel Quick Sync does the hardware work and libopenh264 is the software fallback. Updating the NVIDIA driver enables it with no code change.
- **Cache of derived files** (thumbnails, waveforms, proxies): plain files plus a SQLite index in the user's cache folder (`%LOCALAPPDATA%\vep\cache` on Windows), overridable with `VEP_CACHE_DIR`. Nothing in it is irreplaceable.
- **Proxies** are made locally with the same encoder detection and play back through the same child-process decoder. Software decode plus proxies remains the answer for heavy footage; hardware decode is still deferred.
- **Interface libraries**, all free and bundled with the app so nothing is fetched at runtime: React, zustand, lucide-react for icons. No web fonts, no CDN.
- **Testing tools**, all free and local: vitest for unit tests, and playwright-core driving the real app through the debugging port of the WebView2 that Windows already ships. No browser download is needed.
- **Settings and recent projects** are one JSON file in the user's config folder, overridable with `VEP_CONFIG_DIR`.
- The row above about FFmpeg bindings was superseded in Phase 1 and stays superseded: nothing links FFmpeg, in any phase so far.

## The Windows installer (2026-11-05)

- **NSIS, fetched once by Tauri's build.** The owner allowed it on 2026-11-05: the first `npx tauri build` downloads the free NSIS tools into Tauri's own cache, and later builds use the cache. Nothing is downloaded when ChadFX is installed or run. `ui/src-tauri/tauri.windows.conf.json` builds only the NSIS installer, so WiX (for an MSI) is never fetched. The installer installs for the current user only, needs no administrator rights, and registers the `.chadfx-block` file type (never `.wgsl`).

## What this means for the next phases

- Phase 3 (skills framework) needs nothing new installed: skills expand into the same commands and gestures the window already sends.
- Phase 4 (analysis service) needs only `uv` and a GPU with a few gigabytes of memory; everything runs offline after the first model download.
- There is no account, API key, subscription, or server anywhere in the plan.
