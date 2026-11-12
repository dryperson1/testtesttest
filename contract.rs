//! The editor's sound-block contract (ABI version 1) for a filter, as safe
//! types and one export macro. You do not need to change anything here.
//!
//! Copied, trimmed to what a filter needs, from the kit the editor's own
//! sound blocks are written with (`blocks/src/kit` in the editor's
//! repository, MIT OR Apache-2.0), so that this folder builds on its own.
//! The kit also has transitions (`vep_io2`, `vep_transition`), sources
//! (`vep_generate`) and state cells; the contract itself is written up in
//! the editor's guide to blocks.
//!
//! # What the editor calls
//!
//! | export | what for |
//! |---|---|
//! | `vep_abi() -> i32` | must return 1, the contract's version |
//! | `vep_prepare(sample_rate, channels, max_frames) -> i32` | once per instance, before anything else; 0 = ready |
//! | `vep_io() -> i32` | where the samples are: interleaved f32, `max_frames * channels` |
//! | `vep_params() -> i32` | where the parameter values are: the values at the slice's start, then at its end |
//! | `vep_reset()` | forget everything (after a jump in time) |
//! | `vep_process(frames) -> i32` | change the samples in place; 0 = done |
//!
//! Calls come in slices of at most [`SLICE_FRAMES`] frames, aligned to
//! absolute positions on the timeline. A return value other than 0, a trap
//! (a panic is one), running out of fuel (about 2,000 instructions per
//! sample) or an output sample that is not a finite number turns the block
//! off for the rest of the session, and the editor says so.
//!
//! # Why there is no unsafe code
//!
//! The buffers the editor reads and writes are arrays of atomic cells in
//! `static`s, which Rust allows to change without `unsafe`; on WebAssembly
//! without threads they compile to plain loads and stores. Reading outside a
//! buffer gives 0 and writing outside one does nothing, so the block never
//! panics on its own arithmetic. The one construct Rust counts as unsafe is
//! the attribute that exports a function under a fixed name
//! (`#[unsafe(no_mangle)]`); it appears only in [`filter!`].

// The contract offers more than one block uses (a block that needs no
// setup never calls `Slice::setup`, for example).
#![allow(dead_code)]

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// The version of the editor's sound-block contract this follows.
pub const ABI_VERSION: i32 = 1;

/// How many frames the editor gives a block in one call, at most. The first
/// and last slice of a clip may be shorter.
pub const SLICE_FRAMES: usize = 128;

/// The most channels the sound buffer holds at [`SLICE_FRAMES`] frames.
pub const MAX_CHANNELS: usize = 32;

/// How many samples the sound buffer holds. `vep_prepare` refuses a setup
/// whose `max_frames * channels` is larger.
pub const BUFFER_SAMPLES: usize = SLICE_FRAMES * MAX_CHANNELS;

/// How many parameter values (at one point in time) the parameter buffer
/// holds: 64 parameters of at most 4 values each.
pub const MAX_PARAM_VALUES: usize = 256;

/// Return code: success.
pub const OK: i32 = 0;
/// Return code of `vep_prepare`: the sample rate, channel count or slice
/// length is not supported.
pub const UNSUPPORTED: i32 = 1;
/// Return code: the editor called before `vep_prepare` succeeded.
pub const NOT_PREPARED: i32 = 2;
/// Return code: `frames` was negative or larger than `max_frames`.
pub const BAD_FRAMES: i32 = 3;

/// What the editor runs the block with, as given to `vep_prepare`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setup {
    /// Samples per second, per channel.
    pub sample_rate: u32,
    /// Channels, interleaved (2 = stereo, left first).
    pub channels: usize,
    /// The most frames one call will ever have.
    pub max_frames: usize,
}

/// The sound buffer the editor writes into and reads back: interleaved f32
/// samples (frame 0 left, frame 0 right, frame 1 left, ...).
pub struct Buffer {
    cells: [AtomicU32; BUFFER_SAMPLES],
}

impl Buffer {
    const fn new() -> Buffer {
        Buffer { cells: [const { AtomicU32::new(0) }; BUFFER_SAMPLES] }
    }

    /// Sample `i`; 0 outside the buffer.
    #[inline]
    pub fn get(&self, i: usize) -> f32 {
        match self.cells.get(i) {
            Some(cell) => f32::from_bits(cell.load(Relaxed)),
            None => 0.0,
        }
    }

    /// Set sample `i`; nothing happens outside the buffer.
    #[inline]
    pub fn set(&self, i: usize, value: f32) {
        if let Some(cell) = self.cells.get(i) {
            cell.store(value.to_bits(), Relaxed);
        }
    }
}

/// The parameter values of the current slice: [`Filter::PARAM_VALUES`]
/// values at the slice's start, then as many at its end.
struct ParamBuffer {
    cells: [AtomicU32; 2 * MAX_PARAM_VALUES],
}

impl ParamBuffer {
    const fn new() -> ParamBuffer {
        ParamBuffer { cells: [const { AtomicU32::new(0) }; 2 * MAX_PARAM_VALUES] }
    }

    fn get(&self, i: usize) -> f32 {
        match self.cells.get(i) {
            Some(cell) => f32::from_bits(cell.load(Relaxed)),
            None => 0.0,
        }
    }
}

/// One call's view: how many frames, the setup, and the parameter values at
/// the slice's start and end.
pub struct Slice {
    frames: usize,
    setup: Setup,
    param_values: usize,
}

impl Slice {
    /// Frames in this call (1 to `max_frames`).
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The setup given to `vep_prepare`.
    pub fn setup(&self) -> Setup {
        self.setup
    }

    /// Channels per frame.
    pub fn channels(&self) -> usize {
        self.setup.channels
    }

    /// Samples in this call: frames times channels.
    pub fn samples(&self) -> usize {
        self.frames * self.setup.channels
    }

    /// Parameter value `i` at the slice's first frame (0 if `i` is not a
    /// parameter value of the block).
    pub fn start(&self, i: usize) -> f32 {
        if i < self.param_values { PARAMS.get(i) } else { 0.0 }
    }

    /// Parameter value `i` just after the slice's last frame, that is at the
    /// first frame of the next slice (0 if `i` is not a parameter value).
    pub fn end(&self, i: usize) -> f32 {
        if i < self.param_values { PARAMS.get(self.param_values + i) } else { 0.0 }
    }

    /// Whether parameter value `i` is the same at both ends of the slice.
    pub fn is_steady(&self, i: usize) -> bool {
        self.start(i) == self.end(i)
    }

    /// Parameter value `i` at `frame`, on the straight line from the start
    /// value to the end value (exactly the start value when they are equal).
    pub fn at(&self, i: usize, frame: usize) -> f32 {
        let (a, b) = (self.start(i), self.end(i));
        if a == b {
            return a;
        }
        a + (b - a) * (frame as f32 / self.frames as f32)
    }
}

/// A block that changes sound (`audio.filter`).
pub trait Filter {
    /// How many f32 values the block's parameters flatten to, in the order
    /// of block.json's "params": 1 for a float, int, bool (0 or 1) or enum
    /// (the option's index), 2 for a vec2, 3 for a vec3, 4 for a colour;
    /// text, curve, shape, blob and artifact parameters are not passed. At
    /// most [`MAX_PARAM_VALUES`].
    const PARAM_VALUES: usize;

    /// Get ready for `setup`. There is no allocator: size the block's
    /// `static`s for [`MAX_CHANNELS`]. Return [`OK`], or a non-zero code to
    /// refuse the setup (the buffers' limits are checked already).
    fn prepare(setup: Setup) -> i32 {
        let _ = setup;
        OK
    }

    /// Forget all state: the next call follows a jump in time. A block with
    /// memory clears it here and declares a warm-up in block.json.
    fn reset() {}

    /// Change `slice.samples()` samples of `io` in place. Return [`OK`] or a
    /// non-zero code.
    fn process(slice: &Slice, io: &Buffer) -> i32;
}

static IO: Buffer = Buffer::new();
static PARAMS: ParamBuffer = ParamBuffer::new();
/// The prepared setup: sample rate, channels, max frames; 0 channels = not
/// prepared.
static SETUP: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];

fn prepared() -> Option<Setup> {
    let channels = SETUP[1].load(Relaxed) as usize;
    if channels == 0 {
        return None;
    }
    Some(Setup { sample_rate: SETUP[0].load(Relaxed), channels, max_frames: SETUP[2].load(Relaxed) as usize })
}

/// What [`filter!`] calls. Not for the block to use directly.
#[doc(hidden)]
pub mod glue {
    use super::*;

    pub fn prepare<B: Filter>(sample_rate: i32, channels: i32, max_frames: i32) -> i32 {
        SETUP[1].store(0, Relaxed);
        if B::PARAM_VALUES > MAX_PARAM_VALUES || sample_rate <= 0 || channels <= 0 || max_frames <= 0 {
            return UNSUPPORTED;
        }
        let (channels, max_frames) = (channels as usize, max_frames as usize);
        match channels.checked_mul(max_frames) {
            Some(samples) if samples <= BUFFER_SAMPLES => {}
            _ => return UNSUPPORTED,
        }
        let setup = Setup { sample_rate: sample_rate as u32, channels, max_frames };
        let code = B::prepare(setup);
        if code == OK {
            SETUP[0].store(setup.sample_rate, Relaxed);
            SETUP[2].store(max_frames as u32, Relaxed);
            SETUP[1].store(channels as u32, Relaxed);
        }
        code
    }

    pub fn reset<B: Filter>() {
        B::reset();
    }

    pub fn io() -> i32 {
        IO.cells.as_ptr() as usize as i32
    }

    pub fn params() -> i32 {
        PARAMS.cells.as_ptr() as usize as i32
    }

    pub fn process<B: Filter>(frames: i32) -> i32 {
        let Some(setup) = prepared() else {
            return NOT_PREPARED;
        };
        if frames < 0 || frames as usize > setup.max_frames {
            return BAD_FRAMES;
        }
        if frames == 0 {
            return OK;
        }
        B::process(&Slice { frames: frames as usize, setup, param_values: B::PARAM_VALUES }, &IO)
    }
}

/// Export a [`Filter`]: writes `vep_abi`, `vep_prepare`, `vep_io`,
/// `vep_params`, `vep_reset` and `vep_process`. Name the block's type in it
/// once, at the end of lib.rs.
macro_rules! filter {
    ($block:ty) => {
        /// The contract version (ABI 1).
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_abi() -> i32 {
            $crate::contract::ABI_VERSION
        }

        /// Get ready for a sample rate, channel count and slice length.
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_prepare(sample_rate: i32, channels: i32, max_frames: i32) -> i32 {
            $crate::contract::glue::prepare::<$block>(sample_rate, channels, max_frames)
        }

        /// Where the samples are in memory.
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_io() -> i32 {
            $crate::contract::glue::io()
        }

        /// Where the parameter values are in memory.
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_params() -> i32 {
            $crate::contract::glue::params()
        }

        /// Forget all state.
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_reset() {
            $crate::contract::glue::reset::<$block>()
        }

        /// Change one slice of sound in place.
        #[allow(unsafe_code)]
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_process(frames: i32) -> i32 {
            $crate::contract::glue::process::<$block>(frames)
        }
    };
}
pub(crate) use filter;

/// A panic inside the block becomes a WebAssembly trap, which the editor
/// reports and which turns the block off for the session.
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn on_panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
