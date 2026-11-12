//! `chadfx.tone`: a steady sine tone, the same on every channel.
//!
//! Today's generator (crates/audio mixer, `GeneratorKind::Tone`): frame `i`
//! of a call starting `t0` seconds after the clip's start is
//! `sin((t0 + i / sample_rate) * frequency * 2 pi)` in f64, converted to f32
//! and multiplied by the amplitude in f32. Today's constants 440 Hz and 0.25
//! are the parameters' defaults, so the result is unchanged.
//!
//! The formula has no memory, so the frequency is not animated (its
//! manifest says `keyframable: false`): changing it over time would make
//! the phase jump. The amplitude may be animated and then moves in a
//! straight line inside a slice.
//!
//! Long clips: once the sine's argument passes [`CHEAP_PHASE`] radians
//! (after about 10 minutes at 440 Hz, 13 seconds at 20 kHz), libm's exact
//! argument reduction costs more than the fuel a frame gets, so the block
//! takes the sine of the fractional part of the number of turns instead.
//! That differs from today's formula only by the rounding of the phase
//! (crates/blocks/tests/wasm_exact.rs measures it); before that point the
//! result is today's, bit for bit.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use block_kit::{Block, Buffer, Slice, Source, OK};
use core::f64::consts::TAU;

/// Returned when the editor gives a start time that is not a finite number,
/// or one beyond [`MAX_TIME`] (about 317,000 years), where the phase would
/// lose all precision or overflow.
const BAD_TIME: i32 = 10;

/// The largest start time accepted, in seconds.
const MAX_TIME: f64 = 1e13;

/// Arguments of the sine below this many radians take libm's short argument
/// reduction (musl's limit is 2^20 * pi/2, about 1,647,099); larger ones
/// take a reduction that costs over 4,700 fuel per frame.
const CHEAP_PHASE: f64 = 1.6e6;

/// The sine of `turns * 2 pi`: today's arithmetic while it is cheap, the
/// fractional part of the turns beyond.
fn sine_of_turns(turns: f64) -> f64 {
    let x = turns * TAU;
    if x.abs() < CHEAP_PHASE {
        block_kit::libm::sin(x)
    } else {
        block_kit::libm::sin((turns - block_kit::libm::floor(turns)) * TAU)
    }
}

struct Tone;

impl Block for Tone {
    /// `frequency`, `amplitude`.
    const PARAM_VALUES: usize = 2;
}

/// A finite value of `x` within `min..=max` (`fallback` for NaN).
fn within(x: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if x.is_nan() { fallback } else { x.clamp(min, max) }
}

impl Source for Tone {
    fn generate(slice: &Slice, out: &Buffer, t0: f64) -> i32 {
        if !t0.is_finite() || t0.abs() > MAX_TIME {
            return BAD_TIME;
        }
        let rate = slice.setup().sample_rate as f64;
        let frequency = within(slice.start(0), 20.0, 20_000.0, 440.0) as f64;
        let channels = slice.channels();
        for frame in 0..slice.frames() {
            let amplitude = within(slice.at(1, frame), 0.0, 1.0, 0.0);
            let v = sine_of_turns((t0 + frame as f64 / rate) * frequency) as f32 * amplitude;
            for c in 0..channels {
                out.set(frame * channels + c, v);
            }
        }
        OK
    }
}

block_kit::source!(Tone);
