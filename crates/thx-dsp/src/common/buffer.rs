//! Planar (deinterleaved) audio buffer.
//!
//! A [`Buffer`] is the unit of work handed to [`Processor::process`](super::Processor::process). It exposes
//! a read-only **input** view (`N_in` channels) and a writable **output** view
//! (`N_out` channels).
//!
//! The buffer borrows its storage, it never allocates. Each channel is a
//! contiguous `&[S]` / `&mut [S]`.
//!
//! # Capacity vs. frames
//!
//! Two lengths describe a buffer, mirroring [`Vec::capacity`]/[`Vec::len`]:
//!
//! * **capacity**: The allocated size of each channel. Engines typically
//!   allocate at `max_frames` and reuse that storage every block.
//! * **frames**: The number of *active* frames in this buffer
//!   (`frames <= capacity`).
//!
//! The default accessors ([`input`], [`output`], [`output_mut`]) yield the
//! active `frames` region. The full `capacity` is reachable via the `*_full`
//! accessors.
//!
//! [`input`]: Buffer::input
//! [`output`]: Buffer::output
//! [`output_mut`]: Buffer::output_mut

use super::sample::Sample;

/// A buffer of planar audio with read-only input and a writable output, each a
/// slice of per-channel slices, sharing a common `capacity` and an active
/// `frames` count (`frames <= capacity`).
///
/// Construct one with [`Buffer::new`] (no inactive tail) or
/// [`Buffer::with_capacity`] (explicit capacity).
///
/// The lifetime `'a` ties the view to the borrowed channel storage.
pub struct Buffer<'a, S: Sample> {
    input: &'a [&'a [S]],
    output: &'a mut [&'a mut [S]],
    capacity: usize,
    frames: usize,
}

impl<'a, S: Sample> Buffer<'a, S> {
    /// Create a buffer whose every frame is active (`capacity == frames`).
    pub fn new(input: &'a [&'a [S]], output: &'a mut [&'a mut [S]], frames: usize) -> Self {
        Self::with_capacity(input, output, frames, frames)
    }

    /// Create a buffer with an explicit channel `capacity` of which the first
    /// `frames` are active.
    pub fn with_capacity(
        input: &'a [&'a [S]],
        output: &'a mut [&'a mut [S]],
        capacity: usize,
        frames: usize,
    ) -> Self {
        debug_assert!(
            frames <= capacity,
            "frames ({frames}) must not exceed capacity ({capacity})"
        );
        debug_assert!(
            input.iter().all(|ch| ch.len() >= capacity),
            "every input channel must hold at least `capacity` ({capacity}) samples",
        );
        debug_assert!(
            output.iter().all(|ch| ch.len() >= capacity),
            "every output channel must hold at least `capacity` ({capacity}) samples",
        );

        Self {
            input,
            output,
            capacity,
            frames,
        }
    }

    /// Number of *active* frames in this buffer (`<= capacity`).
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Allocated capacity of each channel, in frames.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of input channels.
    pub fn in_channels(&self) -> usize {
        self.input.len()
    }

    /// Number of output channels.
    pub fn out_channels(&self) -> usize {
        self.output.len()
    }

    /// Read-only samples for input channel `ch`.
    pub fn input(&self, ch: usize) -> &[S] {
        &self.input[ch][..self.frames]
    }

    /// Read-only view of output channel `ch` (e.g. to read back processed/"wet"
    /// samples during a crossfade).
    pub fn output(&self, ch: usize) -> &[S] {
        &self.output[ch][..self.frames]
    }

    /// Writable samples for output channel `ch` (active `frames` region).
    pub fn output_mut(&mut self, ch: usize) -> &mut [S] {
        &mut self.output[ch][..self.frames]
    }

    /// Read-only samples for input channel `ch` over the full `capacity`
    /// (active frames plus the inactive tail).
    pub fn input_full(&self, ch: usize) -> &[S] {
        &self.input[ch][..self.capacity]
    }

    /// Writable samples for output channel `ch` over the full `capacity`.
    ///
    /// Use when a block may write beyond the active `frames` — e.g. a sample-rate
    /// converter producing more output frames than it consumed, or to clear the
    /// inactive tail.
    pub fn output_full_mut(&mut self, ch: usize) -> &mut [S] {
        &mut self.output[ch][..self.capacity]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_count() {
        let frames = 8;
        let in_num_ch = 12;
        let out_num_ch = 2;

        let input = vec![vec![0.0_f32; frames]; in_num_ch];
        let mut output = vec![vec![0.0_f32; frames]; out_num_ch];
        let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
        let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();

        let buffer = Buffer::new(&in_refs, &mut out_refs, frames);
        assert_eq!(buffer.frames(), frames);
        assert_eq!(buffer.in_channels(), in_num_ch);
        assert_eq!(buffer.out_channels(), out_num_ch);
    }

    #[test]
    fn frames_all_active() {
        let input = vec![vec![1.0_f32; 4]; 1];
        let mut output = vec![vec![0.0_f32; 4]; 1];
        let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
        let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();

        // 4 frames allocated, all active.
        let buffer = Buffer::new(&in_refs, &mut out_refs, 4);
        assert_eq!(buffer.frames(), buffer.capacity());
    }

    #[test]
    fn frames_below_capacity() {
        let input = vec![vec![1.0_f32; 8]; 2];
        let mut output = vec![vec![0.0_f32; 8]; 2];
        let in_refs: Vec<&[f32]> = input.iter().map(Vec::as_slice).collect();
        let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();

        // 8 frames allocated, only 3 active.
        let buffer = Buffer::with_capacity(&in_refs, &mut out_refs, 8, 3);
        assert_eq!(buffer.frames(), 3);
        assert_eq!(buffer.capacity(), 8);
        assert_eq!(buffer.input(0).len(), 3);
        assert_eq!(buffer.input_full(0).len(), 8);
    }
}
