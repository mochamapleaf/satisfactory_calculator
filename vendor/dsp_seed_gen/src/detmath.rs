//! planner fork (not upstream): one maths library on every target.
//!
//! Upstream calls `f32::sin`, `f64::powf`, ... from `std`. Those resolve to the platform C library
//! natively (the Windows CRT) and to Rust's bundled libm on `wasm32-unknown-unknown`; the two round
//! differently in the last bit. The planet grid is built with `acos`/`sin`/`cos`, every terrain
//! height is sampled on it, and a vein node is kept or dropped by a height threshold, so a one-bit
//! difference moved veins: seed 12345678 / 32 stars / 0.1x gave 26.70 oil/s on the home planet
//! natively and 24.12 in the browser (review 8, H1).
//!
//! Every transcendental function of the generator goes through this module instead: the pure-Rust
//! `libm` crate in double precision. The `f32` versions compute in `f64` and round once, which is
//! what the game's `UnityEngine.Mathf` does (`(float)Math.Sin((double)f)`). `sqrt`, `floor`,
//! `round_ties_even` and the arithmetic operators are exact IEEE operations and stay on `std`.
pub trait DetMath: Sized {
    fn sin_d(self) -> Self;
    fn cos_d(self) -> Self;
    fn asin_d(self) -> Self;
    fn acos_d(self) -> Self;
    fn atan2_d(self, x: Self) -> Self;
    fn ln_d(self) -> Self;
    fn log10_d(self) -> Self;
    fn log_d(self, base: Self) -> Self;
    fn powf_d(self, e: Self) -> Self;
}

impl DetMath for f64 {
    #[inline]
    fn sin_d(self) -> f64 {
        libm::sin(self)
    }
    #[inline]
    fn cos_d(self) -> f64 {
        libm::cos(self)
    }
    #[inline]
    fn asin_d(self) -> f64 {
        libm::asin(self)
    }
    #[inline]
    fn acos_d(self) -> f64 {
        libm::acos(self)
    }
    #[inline]
    fn atan2_d(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
    #[inline]
    fn ln_d(self) -> f64 {
        libm::log(self)
    }
    #[inline]
    fn log10_d(self) -> f64 {
        libm::log10(self)
    }
    #[inline]
    fn log_d(self, base: f64) -> f64 {
        libm::log(self) / libm::log(base)
    }
    #[inline]
    fn powf_d(self, e: f64) -> f64 {
        libm::pow(self, e)
    }
}

impl DetMath for f32 {
    #[inline]
    fn sin_d(self) -> f32 {
        libm::sin(self as f64) as f32
    }
    #[inline]
    fn cos_d(self) -> f32 {
        libm::cos(self as f64) as f32
    }
    #[inline]
    fn asin_d(self) -> f32 {
        libm::asin(self as f64) as f32
    }
    #[inline]
    fn acos_d(self) -> f32 {
        libm::acos(self as f64) as f32
    }
    #[inline]
    fn atan2_d(self, x: f32) -> f32 {
        libm::atan2(self as f64, x as f64) as f32
    }
    #[inline]
    fn ln_d(self) -> f32 {
        libm::log(self as f64) as f32
    }
    #[inline]
    fn log10_d(self) -> f32 {
        libm::log10(self as f64) as f32
    }
    #[inline]
    fn log_d(self, base: f32) -> f32 {
        (libm::log(self as f64) / libm::log(base as f64)) as f32
    }
    #[inline]
    fn powf_d(self, e: f32) -> f32 {
        libm::pow(self as f64, e as f64) as f32
    }
}
