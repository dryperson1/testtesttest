//! `chadfx.crossfade`: the outgoing sound fades out while the incoming one
//! fades in, in straight lines.
//!
//! Today's per-sample fade (crates/audio mixer `apply_partial_fade`): for
//! frame `i` of the call, `u = clamp((offset + i) / max(full, 1), 0, 1)` in
//! f32; the outgoing sample is multiplied by `1 - u`, the incoming one by
//! `u`, and the two are added. With no clip on one side the editor passes
//! silence for it, which gives today's fade in and fade out.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use block_kit::{Block, Buffer, Slice, Transition, OK};

struct Crossfade;

impl Block for Crossfade {
    /// No parameters.
    const PARAM_VALUES: usize = 0;
}

impl Transition for Crossfade {
    fn transition(slice: &Slice, outgoing: &Buffer, incoming: &Buffer, offset: i32, full: i32) -> i32 {
        let channels = slice.channels();
        let full = full.max(1) as f32;
        for frame in 0..slice.frames() {
            let u = ((offset as i64 + frame as i64) as f32 / full).clamp(0.0, 1.0);
            let down = 1.0 - u;
            for c in 0..channels {
                let i = frame * channels + c;
                outgoing.set(i, outgoing.get(i) * down + incoming.get(i) * u);
            }
        }
        OK
    }
}

block_kit::transition!(Crossfade);
