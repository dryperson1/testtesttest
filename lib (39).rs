//! The ChadFX sound-block kit: write a sound block in Rust that follows the
//! editor's WebAssembly contract (ABI version 1) without any unsafe code.
//!
//! # What a sound block is
//!
//! A WebAssembly module that imports nothing at all (every function it uses,
//! maths included, is compiled into it; [`libm`] is re-exported for that),
//! has no start function, and exports its memory and a few functions. The
//! editor calls them in fixed slices of at most [`SLICE_FRAMES`] frames:
//!
//! | export | what the editor uses it for |
//! |---|---|
//! | `vep_abi() -> i32` | must return [`ABI_VERSION`] (1) |
//! | `vep_prepare(sample_rate, channels, max_frames) -> i32` | once per instance, before anything else; 0 = ready |
//! | `vep_io() -> i32` | where the samples are: interleaved f32, `max_frames * channels` |
//! | `vep_io2() -> i32` | transitions only: the incoming clip's samples |
//! | `vep_params() -> i32` | where the parameter values are: `P` values at the slice start, then `P` at its end |
//! | `vep_reset()` | forget everything (after a jump in time) |
//! | `vep_process(frames) -> i32` | filters: change the samples in place |
//! | `vep_transition(frames, offset, full) -> i32` | transitions: mix the incoming sound into the outgoing one |
//! | `vep_generate(frames, t0) -> i32` | sources: write new samples; `t0` = seconds since the clip's start |
//!
//! A return value other than 0 turns the block off for the rest of the
//! session, and the editor shows the number in its message. So do a trap
//! (for example a panic, which this kit turns into a trap), running out of
//! fuel (each call may run about 2000 instructions per sample), and an
//! output sample that is not a finite number.
//!
//! # Writing one
//!
//! Implement [`Block`] and one of [`Filter`], [`Transition`] or [`Source`]
//! on a type, then name it in [`filter!`], [`transition!`] or [`source!`],
//! which write every export. State that must last between calls lives in
//! `static` cells of this kit ([`StateF32`], [`StateF64`], [`StatesF64`]);
//! the editor only ever runs one call at a time.
//!
//! ```ignore
//! struct Half;
//! impl block_kit::Block for Half {
//!     const PARAM_VALUES: usize = 0;
//! }
//! impl block_kit::Filter for Half {
//!     fn process(slice: &block_kit::Slice, io: &block_kit::Buffer) -> i32 {
//!         for i in 0..slice.samples() {
//!             io.set(i, io.get(i) * 0.5);
//!         }
//!         block_kit::OK
//!     }
//! }
//! block_kit::filter!(Half);
//! ```
//!
//! # Memory and fuel
//!
//! The kit allocates nothing: its sound buffers and the parameter buffer
//! are `static`s, sized for [`SLICE_FRAMES`] frames of [`MAX_CHANNELS`]
//! channels, and a block's own state should be too. A module may start
//! with at most 64 pages (4 MiB) and grow to 16 MiB, but growing costs fuel
//! (1 per 64 bytes), so a block that grows does it in `vep_prepare`, which
//! gets the fuel of a full slice. Every other call gets 100,000 fuel plus
//! 2,000 per frame per channel; a sine from [`libm`] costs about 1,000.
//!
//! # Why there is no unsafe code
//!
//! The buffers the editor reads and writes are arrays of atomic cells in
//! `static`s, which Rust allows to change without `unsafe`; on WebAssembly
//! without threads they compile to plain loads and stores. Indexing outside
//! a buffer reads 0 and writes nothing, so a block never panics on its own
//! arithmetic. The one construct Rust counts as unsafe is the attribute that
//! exports a function under a fixed name (`#[unsafe(no_mangle)]`); it
//! appears only inside this kit's three export macros.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

/// The maths library blocks use (sine, powers, logarithms...), compiled into
/// the module so the results are the same on every machine.
pub use libm;

/// The version of the editor's sound-block contract this kit follows.
pub const ABI_VERSION: i32 = 1;

/// How many frames the editor gives a block in one call, at most. Slices are
/// aligned to absolute sample positions on the timeline, so the first and
/// last slice of a clip may be shorter.
pub const SLICE_FRAMES: usize = 128;

/// The most channels this kit's buffers hold at [`SLICE_FRAMES`] frames.
pub const MAX_CHANNELS: usize = 32;

/// How many samples each of the kit's sound buffers holds. `vep_prepare`
/// refuses a setup whose `max_frames * channels` is larger.
pub const BUFFER_SAMPLES: usize = SLICE_FRAMES * MAX_CHANNELS;

/// How many parameter values (one point in time) the kit's parameter buffer
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
    /// Samples per second, per channel (8000 to 192000 in practice).
    pub sample_rate: u32,
    /// Channels, interleaved (1 or more; 2 = stereo, left first).
    pub channels: usize,
    /// The most frames one call will ever have.
    pub max_frames: usize,
}

/// A sound buffer the editor writes into and reads back: interleaved f32
/// samples (frame 0 channel 0, frame 0 channel 1, ...).
pub struct Buffer {
    cells: [AtomicU32; BUFFER_SAMPLES],
}

impl Buffer {
    /// An empty (silent) buffer.
    pub const fn new() -> Buffer {
        Buffer { cells: [const { AtomicU32::new(0) }; BUFFER_SAMPLES] }
    }

    /// The buffer's byte offset in the module's memory (what `vep_io`
    /// returns).
    pub fn offset(&self) -> i32 {
        self.cells.as_ptr() as usize as i32
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

    /// How many samples the buffer holds ([`BUFFER_SAMPLES`]).
    pub const fn capacity(&self) -> usize {
        BUFFER_SAMPLES
    }
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer::new()
    }
}

/// The parameter values of the current slice: [`Block::PARAM_VALUES`]
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

/// One f32 that lasts between calls (a block's state).
pub struct StateF32(AtomicU32);

impl StateF32 {
    /// A state cell holding `value`.
    pub const fn new(value: f32) -> StateF32 {
        StateF32(AtomicU32::new(value.to_bits()))
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Relaxed))
    }

    /// Change the value.
    #[inline]
    pub fn set(&self, value: f32) {
        self.0.store(value.to_bits(), Relaxed);
    }
}

/// One f64 that lasts between calls (a block's state).
pub struct StateF64(AtomicU64);

impl StateF64 {
    /// A state cell holding `value`.
    pub const fn new(value: f64) -> StateF64 {
        StateF64(AtomicU64::new(value.to_bits()))
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(Relaxed))
    }

    /// Change the value.
    #[inline]
    pub fn set(&self, value: f64) {
        self.0.store(value.to_bits(), Relaxed);
    }
}

/// `N` f64 values that last between calls (filter memories, delay lines).
pub struct StatesF64<const N: usize>([AtomicU64; N]);

impl<const N: usize> StatesF64<N> {
    /// `N` zeros.
    pub const fn new() -> StatesF64<N> {
        StatesF64([const { AtomicU64::new(0) }; N])
    }

    /// Value `i`; 0 outside.
    #[inline]
    pub fn get(&self, i: usize) -> f64 {
        match self.0.get(i) {
            Some(cell) => f64::from_bits(cell.load(Relaxed)),
            None => 0.0,
        }
    }

    /// Change value `i`; nothing happens outside.
    #[inline]
    pub fn set(&self, i: usize, value: f64) {
        if let Some(cell) = self.0.get(i) {
            cell.store(value.to_bits(), Relaxed);
        }
    }

    /// Set every value to `value`.
    pub fn fill(&self, value: f64) {
        for cell in &self.0 {
            cell.store(value.to_bits(), Relaxed);
        }
    }

    /// How many values there are (`N`).
    pub const fn len(&self) -> usize {
        N
    }

    /// Whether there are none.
    pub const fn is_empty(&self) -> bool {
        N == 0
    }
}

impl<const N: usize> Default for StatesF64<N> {
    fn default() -> StatesF64<N> {
        StatesF64::new()
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

/// What every block declares and may do on `vep_prepare` and `vep_reset`.
pub trait Block {
    /// How many f32 values the block's parameters flatten to, in manifest
    /// order: 1 for float, int, bool (0 or 1) and enum (the option's index),
    /// 2 for vec2, 3 for vec3, 4 for color; text, curve, shape, blob and
    /// artifact parameters are not passed. At most [`MAX_PARAM_VALUES`].
    const PARAM_VALUES: usize;

    /// Get ready for `setup` (allocate nothing: there is no allocator; size
    /// the block's statics for [`MAX_CHANNELS`]). Return [`OK`], or a
    /// non-zero code to refuse the setup. The kit has already refused
    /// setups its buffers cannot hold.
    fn prepare(setup: Setup) -> i32 {
        let _ = setup;
        OK
    }

    /// Forget all state: the next call follows a jump in time.
    fn reset() {}
}

/// A block that changes sound (`audio.filter`).
pub trait Filter: Block {
    /// Change `slice.samples()` samples of `io` in place. Return [`OK`] or a
    /// non-zero code.
    fn process(slice: &Slice, io: &Buffer) -> i32;
}

/// A block that mixes two clips at a cut (`audio.transition`).
pub trait Transition: Block {
    /// Mix `incoming` into `outgoing` (both `slice.samples()` samples; the
    /// result goes into `outgoing`). `offset` is the number of frames since
    /// the fade started and `full` the fade's length in frames; with no
    /// clip on one side, the editor gives silence for it. Return [`OK`] or a
    /// non-zero code.
    fn transition(slice: &Slice, outgoing: &Buffer, incoming: &Buffer, offset: i32, full: i32) -> i32;
}

/// A block that makes sound from nothing (`audio.source`).
pub trait Source: Block {
    /// Write `slice.samples()` samples into `out`; `t0` is the time of the
    /// first frame in seconds since the clip's start. Return [`OK`] or a
    /// non-zero code.
    fn generate(slice: &Slice, out: &Buffer, t0: f64) -> i32;
}

static IO: Buffer = Buffer::new();
static IO2: Buffer = Buffer::new();
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

/// What the export macros call. Not for blocks to use directly.
#[doc(hidden)]
pub mod glue {
    use super::*;

    pub fn prepare<B: Block>(sample_rate: i32, channels: i32, max_frames: i32) -> i32 {
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

    pub fn reset<B: Block>() {
        B::reset();
    }

    pub fn io() -> i32 {
        IO.offset()
    }

    pub fn io2() -> i32 {
        IO2.offset()
    }

    pub fn params() -> i32 {
        PARAMS.cells.as_ptr() as usize as i32
    }

    /// The slice for a call with `frames` frames: `Err(code)` to return at
    /// once, `Ok(None)` for nothing to do.
    fn slice<B: Block>(frames: i32) -> Result<Option<Slice>, i32> {
        let setup = prepared().ok_or(NOT_PREPARED)?;
        if frames < 0 || frames as usize > setup.max_frames {
            return Err(BAD_FRAMES);
        }
        if frames == 0 {
            return Ok(None);
        }
        Ok(Some(Slice { frames: frames as usize, setup, param_values: B::PARAM_VALUES }))
    }

    pub fn process<B: Filter>(frames: i32) -> i32 {
        match slice::<B>(frames) {
            Err(code) => code,
            Ok(None) => OK,
            Ok(Some(s)) => B::process(&s, &IO),
        }
    }

    pub fn transition<B: Transition>(frames: i32, offset: i32, full: i32) -> i32 {
        match slice::<B>(frames) {
            Err(code) => code,
            Ok(None) => OK,
            Ok(Some(s)) => B::transition(&s, &IO, &IO2, offset, full),
        }
    }

    pub fn generate<B: Source>(frames: i32, t0: f64) -> i32 {
        match slice::<B>(frames) {
            Err(code) => code,
            Ok(None) => OK,
            Ok(Some(s)) => B::generate(&s, &IO, t0),
        }
    }
}

/// The exports every kind has. Used by the three public macros.
#[doc(hidden)]
#[macro_export]
macro_rules! __vep_common_exports {
    ($block:ty) => {
        /// The contract version (ABI 1).
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_abi() -> i32 {
            $crate::ABI_VERSION
        }

        /// Get ready for a sample rate, channel count and slice length.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_prepare(sample_rate: i32, channels: i32, max_frames: i32) -> i32 {
            $crate::glue::prepare::<$block>(sample_rate, channels, max_frames)
        }

        /// Where the samples are in memory.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_io() -> i32 {
            $crate::glue::io()
        }

        /// Where the parameter values are in memory.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_params() -> i32 {
            $crate::glue::params()
        }

        /// Forget all state.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_reset() {
            $crate::glue::reset::<$block>()
        }
    };
}

/// Export a [`Filter`] (`audio.filter`): writes `vep_abi`, `vep_prepare`,
/// `vep_io`, `vep_params`, `vep_reset` and `vep_process`.
#[macro_export]
macro_rules! filter {
    ($block:ty) => {
        $crate::__vep_common_exports!($block);

        /// Change one slice of sound in place.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_process(frames: i32) -> i32 {
            $crate::glue::process::<$block>(frames)
        }
    };
}

/// Export a [`Transition`] (`audio.transition`): the common exports plus
/// `vep_io2` and `vep_transition`.
#[macro_export]
macro_rules! transition {
    ($block:ty) => {
        $crate::__vep_common_exports!($block);

        /// Where the incoming clip's samples are in memory.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_io2() -> i32 {
            $crate::glue::io2()
        }

        /// Mix one slice of the incoming clip into the outgoing one.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_transition(frames: i32, offset: i32, full: i32) -> i32 {
            $crate::glue::transition::<$block>(frames, offset, full)
        }
    };
}

/// Export a [`Source`] (`audio.source`): the common exports plus
/// `vep_generate`.
#[macro_export]
macro_rules! source {
    ($block:ty) => {
        $crate::__vep_common_exports!($block);

        /// Write one slice of new sound.
        #[unsafe(no_mangle)]
        pub extern "C" fn vep_generate(frames: i32, t0: f64) -> i32 {
            $crate::glue::generate::<$block>(frames, t0)
        }
    };
}

/// A panic inside a block becomes a WebAssembly trap, which the editor
/// reports and which turns the block off for the session.
#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn on_panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
