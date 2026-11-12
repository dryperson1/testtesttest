//! `chadfx.eq3`: a three-band equalizer with today's `AudioEq` parameter
//! names (the old editor listed this effect but never played it).
//!
//! * a low shelf at `low_freq` with `low_gain_db`,
//! * a peak at the geometric middle of the two corners,
//!   `sqrt(low_freq * high_freq)`, with Q = 1/sqrt(2) and `mid_gain_db`,
//! * a high shelf at `high_freq` with `high_gain_db`.
//!
//! The filters are the "Audio EQ Cookbook" biquads (shelf slope 1), run in
//! f64 in transposed direct form II, one set of memories per channel. Gains
//! are limited to -24..24 dB and corners to 20 Hz .. min(20 kHz, 0.49 x the
//! sample rate); a value that is not a number counts as the parameter's
//! default. Sample rates below 1 kHz are refused in `vep_prepare` (the
//! editor mixes at 8 kHz or more). With all three gains at exactly 0 dB the sound comes out
//! unchanged, bit for bit (the filters keep running, so turning a gain up
//! later starts without a click).
//!
//! The filters have memory, so the manifest declares a warm-up: after a
//! jump in time the editor resets the block and runs it over the preceding
//! 200 ms first. While a parameter changes inside a slice, the filters are
//! redesigned every 16 frames from the values on the straight line between
//! the slice's start and end values.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use block_kit::libm;
use block_kit::{Block, Buffer, Filter, Setup, Slice, StatesF64, MAX_CHANNELS, OK};
use core::f64::consts::{FRAC_1_SQRT_2, PI, SQRT_2};

/// Low shelf, peak, high shelf.
const BANDS: usize = 3;
/// b0, b1, b2, a1, a2, all divided by a0.
const COEFS: usize = 5;
/// low_gain_db, mid_gain_db, high_gain_db, low_freq, high_freq.
const PARAMS: usize = 5;
/// Frames between redesigns while a parameter changes.
const STEP: usize = 16;
/// Filter memories smaller than this are set to zero, so silence after
/// sound never runs on slow denormal numbers.
const TINY: f64 = 1e-30;

/// Two memories per band per channel.
static MEMORY: StatesF64<{ MAX_CHANNELS * BANDS * 2 }> = StatesF64::new();
/// The coefficients designed last, for steady parameters.
static DESIGNED: StatesF64<{ BANDS * COEFS }> = StatesF64::new();
/// The parameter values `DESIGNED` was made from (NaN: none yet).
static DESIGNED_FROM: StatesF64<PARAMS> = StatesF64::new();

type Design = [[f64; COEFS]; BANDS];

/// The lowest sample rate the filters are designed for; `vep_prepare`
/// refuses lower ones (the editor mixes at 8 kHz or more).
const MIN_RATE: u32 = 1000;

/// `x` limited to `lo..=hi` (`fallback` for NaN), without the panics of
/// `clamp`.
fn limit(x: f64, lo: f64, hi: f64, fallback: f64) -> f64 {
    if x.is_nan() { fallback } else { x.max(lo).min(hi) }
}

/// A shelf: low (`high == false`) or high.
fn shelf(high: bool, db: f64, freq: f64, rate: f64) -> [f64; COEFS] {
    let a = libm::pow(10.0, db / 40.0);
    let w0 = 2.0 * PI * freq / rate;
    let (cos, sin) = (libm::cos(w0), libm::sin(w0));
    let k = 2.0 * libm::sqrt(a) * (sin / 2.0 * SQRT_2);
    let (p, m) = (a + 1.0, a - 1.0);
    let (b0, b1, b2, a0, a1, a2) = if high {
        (a * (p + m * cos + k), -2.0 * a * (m + p * cos), a * (p + m * cos - k), p - m * cos + k, 2.0 * (m - p * cos), p - m * cos - k)
    } else {
        (a * (p - m * cos + k), 2.0 * a * (m - p * cos), a * (p - m * cos - k), p + m * cos + k, -2.0 * (m + p * cos), p + m * cos - k)
    };
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// A peak with Q = 1/sqrt(2).
fn peak(db: f64, freq: f64, rate: f64) -> [f64; COEFS] {
    let a = libm::pow(10.0, db / 40.0);
    let w0 = 2.0 * PI * freq / rate;
    let (cos, sin) = (libm::cos(w0), libm::sin(w0));
    let alpha = sin / (2.0 * FRAC_1_SQRT_2);
    let a0 = 1.0 + alpha / a;
    [(1.0 + alpha * a) / a0, -2.0 * cos / a0, (1.0 - alpha * a) / a0, -2.0 * cos / a0, (1.0 - alpha / a) / a0]
}

/// The three filters for parameter values `p` at `rate`.
fn design(p: [f32; PARAMS], rate: f64) -> Design {
    let top = (0.49 * rate).clamp(20.0, 20_000.0);
    let gain = |x: f32| limit(x as f64, -24.0, 24.0, 0.0);
    let low = limit(p[3] as f64, 20.0, top, 200.0);
    let high = limit(p[4] as f64, 20.0, top, 4000.0f64.min(top));
    let mid = libm::sqrt(low * high);
    [shelf(false, gain(p[0]), low, rate), peak(gain(p[1]), mid, rate), shelf(true, gain(p[2]), high, rate)]
}

/// The design for steady values `p`, reusing the last one when nothing
/// changed.
fn steady_design(p: [f32; PARAMS], rate: f64) -> Design {
    let same = (0..PARAMS).all(|i| DESIGNED_FROM.get(i).to_bits() == (p[i] as f64).to_bits());
    let mut d = [[0.0; COEFS]; BANDS];
    if same {
        for (band, coefs) in d.iter_mut().enumerate() {
            for (k, c) in coefs.iter_mut().enumerate() {
                *c = DESIGNED.get(band * COEFS + k);
            }
        }
        return d;
    }
    d = design(p, rate);
    for (band, coefs) in d.iter().enumerate() {
        for (k, c) in coefs.iter().enumerate() {
            DESIGNED.set(band * COEFS + k, *c);
        }
    }
    for (i, v) in p.iter().enumerate() {
        DESIGNED_FROM.set(i, *v as f64);
    }
    d
}

/// Run frames `from..to` of every channel through the filters. With
/// `unchanged`, the memories follow the sound but the samples stay as they
/// are.
fn run(io: &Buffer, channels: usize, from: usize, to: usize, d: &Design, unchanged: bool) {
    for c in 0..channels {
        let base = c * BANDS * 2;
        let mut z = [0.0f64; BANDS * 2];
        for (k, m) in z.iter_mut().enumerate() {
            *m = MEMORY.get(base + k);
        }
        for frame in from..to {
            let i = frame * channels + c;
            let mut s = io.get(i) as f64;
            for (band, [b0, b1, b2, a1, a2]) in d.iter().enumerate() {
                let y = b0 * s + z[2 * band];
                z[2 * band] = b1 * s - a1 * y + z[2 * band + 1];
                z[2 * band + 1] = b2 * s - a2 * y;
                s = y;
            }
            if !unchanged {
                io.set(i, s as f32);
            }
        }
        for (k, m) in z.iter().enumerate() {
            let keep = m.is_finite() && m.abs() >= TINY;
            MEMORY.set(base + k, if keep { *m } else { 0.0 });
        }
    }
}

fn forget() {
    MEMORY.fill(0.0);
    DESIGNED_FROM.fill(f64::NAN);
}

struct Eq3;

impl Block for Eq3 {
    /// low_gain_db, mid_gain_db, high_gain_db, low_freq, high_freq.
    const PARAM_VALUES: usize = PARAMS;

    fn prepare(setup: Setup) -> i32 {
        if setup.sample_rate < MIN_RATE {
            return block_kit::UNSUPPORTED;
        }
        forget();
        OK
    }

    fn reset() {
        forget();
    }
}

impl Filter for Eq3 {
    fn process(slice: &Slice, io: &Buffer) -> i32 {
        let rate = slice.setup().sample_rate as f64;
        let (channels, frames) = (slice.channels(), slice.frames());
        if (0..PARAMS).all(|i| slice.is_steady(i)) {
            let p = [slice.start(0), slice.start(1), slice.start(2), slice.start(3), slice.start(4)];
            let neutral = p[0] == 0.0 && p[1] == 0.0 && p[2] == 0.0;
            run(io, channels, 0, frames, &steady_design(p, rate), neutral);
            return OK;
        }
        let mut from = 0;
        while from < frames {
            let to = (from + STEP).min(frames);
            let p = [slice.at(0, from), slice.at(1, from), slice.at(2, from), slice.at(3, from), slice.at(4, from)];
            run(io, channels, from, to, &design(p, rate), false);
            from = to;
        }
        OK
    }
}

block_kit::filter!(Eq3);
