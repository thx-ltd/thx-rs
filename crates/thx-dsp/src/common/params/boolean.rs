//! [`BoolParam`]: a discrete on/off parameter.

use core::sync::atomic::{AtomicBool, Ordering::Relaxed};

use super::Param;

/// A boolean parameter (bypass, enable, …).
///
/// Discrete parameters do not smooth — there is nothing meaningful between
/// `false` and `true` — so reads are an instantaneous atomic load. Like the
/// other params it is `Sync`, so it can live in a shared (`Arc`) parameter
/// struct: the control thread [`set`](Self::set)s it, the audio thread
/// [`get`](Self::get)s it, both wait-free.
pub struct BoolParam {
    value: AtomicBool,
    default: bool,
}

impl BoolParam {
    /// A boolean parameter resting at `default`.
    pub fn new(default: bool) -> Self {
        Self {
            value: AtomicBool::new(default),
            default,
        }
    }

    /// Set the value. Control thread (but cheap and also safe to read back).
    pub fn set(&self, value: bool) {
        self.value.store(value, Relaxed);
    }

    /// The current value. Wait-free: audio thread or control thread.
    pub fn get(&self) -> bool {
        self.value.load(Relaxed)
    }
}

impl Param for BoolParam {
    type Plain = bool;

    fn default_value(&self) -> bool {
        self.default
    }

    fn value(&self) -> bool {
        self.get()
    }

    fn reset(&self) {
        // Discrete: nothing to settle.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_roundtrip() {
        let p = BoolParam::new(false);
        assert!(!p.default_value());
        assert!(!p.get());
        p.set(true);
        assert!(p.get());
        assert!(p.value());
    }
}
