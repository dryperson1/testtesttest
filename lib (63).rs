//! Stereo width (`example.width`): a ChadFX sound filter that makes stereo
//! sound narrower or wider.
//!
//! Each frame's left and right samples are split into what they share (the
//! middle, `(left + right) * 0.5`) and what tells them apart (the side,
//! `(left - right) * 0.5`). The side is multiplied by `width` and the two
//! are put back together: left = middle + side, right = middle - side. So
//! width 0 gives mono, 1 leaves the sound exactly as it is (to the bit: the
//! arithmetic is skipped), and 2 doubles the difference between the
//! channels. With `swap` on, left and right then change places. Sound that
//! is not stereo is left exactly as it is.
//!
//! When the width is animated it moves in a straight line inside each slice,
//! from its value at the slice's start to its value at the slice's end, so
//! the change is smooth.
//!
//! This is the file to change for a block of your own. `contract.rs` is the
//! editor's contract and stays as it is.
#![no_std]
// The only unsafe code allowed is the attribute that exports a function
// under a fixed name, inside `contract::filter!`; nothing else may use it.
#![deny(unsafe_code)]

mod contract;

use contract::{Buffer, Filter, OK, Slice};

/// Where each parameter is among the values the editor passes, in the order
/// of block.json's "params". A number, a switch (0 or 1) or a choice (the
/// option's index) takes one value; a vec2 two, a vec3 three, a colour four.
const WIDTH: usize = 0;
const SWAP: usize = 1;

/// The widest this block makes sound, as block.json's range says. A project
/// edited by hand can hold any number; the block keeps to its range.
const MAX_WIDTH: f32 = 2.0;

/// The block.
struct Width;

impl Filter for Width {
    /// `width` and `swap`.
    const PARAM_VALUES: usize = 2;

    fn process(slice: &Slice, io: &Buffer) -> i32 {
        if slice.channels() != 2 {
            return OK;
        }
        let swap = slice.start(SWAP) != 0.0;
        for frame in 0..slice.frames() {
            let (left, right) = (io.get(2 * frame), io.get(2 * frame + 1));
            let width = slice.at(WIDTH, frame).clamp(0.0, MAX_WIDTH);
            let (mut l, mut r) = if width == 1.0 {
                (left, right)
            } else {
                let middle = (left + right) * 0.5;
                let side = (left - right) * 0.5 * width;
                (middle + side, middle - side)
            };
            if swap {
                core::mem::swap(&mut l, &mut r);
            }
            io.set(2 * frame, l);
            io.set(2 * frame + 1, r);
        }
        OK
    }
}

contract::filter!(Width);
