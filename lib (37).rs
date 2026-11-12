//! `chadfx.gain`: changes the volume by `gain_db` decibels.
//!
//! The multiplier is today's `10^(dB / 20)` in f32 (crates/audio mixer
//! `db_to_gain`). It is computed in f64 and rounded once to f32: measured
//! against the editor's native maths this agrees far more often than libm's
//! f32 power function (crates/blocks/tests/wasm_exact.rs).
//!
//! A steady value multiplies every sample by the same number, as today. When
//! the value changes inside a slice (animated gain), the multiplier moves in
//! a straight line from the value at the slice's start to the value at its
//! end, so the change is smooth (owner decision D2).
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use block_kit::{Block, Buffer, Filter, Slice, OK};

/// The most decibels applied. The manifest's range is -96 to 24 dB and the
/// editor cannot set more, but a hand-edited project can hold any number, and
/// the mixer has always applied it as it is (the golden sound case
/// `gain_m120_beyond_range`): so does this block, up to here, where the
/// multiplier is about 1e19 and any sample short of 1e19 still comes out a
/// finite number.
const MAX_DB: f32 = 380.0;

/// The multiplier for `db` decibels. Not a number counts as 0 dB.
fn db_to_gain(db: f32) -> f32 {
    let db = if db.is_nan() { 0.0 } else { db.min(MAX_DB) };
    block_kit::libm::pow(10.0, (db / 20.0) as f64) as f32
}

struct Gain;

impl Block for Gain {
    /// `gain_db`.
    const PARAM_VALUES: usize = 1;
}

impl Filter for Gain {
    fn process(slice: &Slice, io: &Buffer) -> i32 {
        let start = db_to_gain(slice.start(0));
        if slice.is_steady(0) {
            for i in 0..slice.samples() {
                io.set(i, io.get(i) * start);
            }
            return OK;
        }
        let end = db_to_gain(slice.end(0));
        let (channels, frames) = (slice.channels(), slice.frames());
        for frame in 0..frames {
            let g = start + (end - start) * (frame as f32 / frames as f32);
            for c in 0..channels {
                let i = frame * channels + c;
                io.set(i, io.get(i) * g);
            }
        }
        OK
    }
}

block_kit::filter!(Gain);
