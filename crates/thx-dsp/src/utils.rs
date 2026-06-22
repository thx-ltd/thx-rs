//! DSP math helpers shared across blocks.

use crate::sample::Sample;

/// Convert decibels to a linear amplitude factor.
pub fn db_to_linear<S: Sample>(db: S) -> S {
    S::from_f64(10.0_f64.powf(db.to_f64() / 20.0))
}
