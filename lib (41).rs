//! `chadfx.pan`: places stereo sound between left and right.
//!
//! Today's equal-power pan (crates/audio mixer `pan_gains`): with
//! `theta = (clamp(pan, -1, 1) + 1) * pi/4`, left is multiplied by
//! `cos(theta) * sqrt(2)` and right by `sin(theta) * sqrt(2)`, in f32. The
//! cosine and sine are computed in f64 and rounded once to f32. Sound that
//! is not stereo is left exactly as it is (today: 1.0 per channel).
//!
//! When the value changes inside a slice (animated pan), both multipliers
//! move in a straight line from their values at the slice's start to their
//! values at its end, so the movement is smooth (owner decision D2).
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use block_kit::{Block, Buffer, Filter, Slice, OK};
use core::f32::consts::{FRAC_PI_4, SQRT_2};

/// The left and right multipliers for `pan`.
fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = if pan.is_nan() { 0.0 } else { pan };
    let theta = ((pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4) as f64;
    (block_kit::libm::cos(theta) as f32 * SQRT_2, block_kit::libm::sin(theta) as f32 * SQRT_2)
}

struct Pan;

impl Block for Pan {
    /// `pan`.
    const PARAM_VALUES: usize = 1;
}

impl Filter for Pan {
    fn process(slice: &Slice, io: &Buffer) -> i32 {
        if slice.channels() != 2 {
            return OK;
        }
        let frames = slice.frames();
        let (left, right) = pan_gains(slice.start(0));
        if slice.is_steady(0) {
            for frame in 0..frames {
                io.set(2 * frame, io.get(2 * frame) * left);
                io.set(2 * frame + 1, io.get(2 * frame + 1) * right);
            }
            return OK;
        }
        let (left_end, right_end) = pan_gains(slice.end(0));
        for frame in 0..frames {
            let k = frame as f32 / frames as f32;
            let l = left + (left_end - left) * k;
            let r = right + (right_end - right) * k;
            io.set(2 * frame, io.get(2 * frame) * l);
            io.set(2 * frame + 1, io.get(2 * frame + 1) * r);
        }
        OK
    }
}

block_kit::filter!(Pan);
