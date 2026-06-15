//! Planar (deinterleaved) audio buffers.
//!
//! A [`Buffer`] holds **one signal**: `channels` planar channels stored in one
//! contiguous, channel-major slab. It owns its storage, which is allocated once
//! (on the control thread, at construction) and only reused afterwards — none
//! of the accessors allocate, so a pre-built buffer is safe to touch from the
//! audio thread.
//!
//! A block reads one buffer and writes another (carried by
//! [`BlockSignal`](crate::BlockSignal)). Keeping input and output *separate* is
//! what lets it change layout (upmix/downmix) or frame count (resampling)
//! without copies.
//!
//! # Capacity vs. frames
//!
//! Two lengths describe a buffer, mirroring [`Vec::capacity`]/[`Vec::len`]:
//!
//! * **capacity** — the allocated size of each channel, fixed at construction
//!   (engines allocate at `max_frames` and reuse the storage every block);
//! * **frames** — the number of *active* frames (`frames <= capacity`).
//!
//! [`channel`](Buffer::channel)/[`channel_mut`](Buffer::channel_mut) yield the
//! active region. A producer that emits a different frame count than it
//! consumed (e.g. a resampler) writes via
//! [`channel_full_mut`](Buffer::channel_full_mut) and then publishes the new
//! length with [`set_frames`](Buffer::set_frames).

use crate::sample::Sample;

/// One planar audio signal: `channels` channels of `capacity` frames in a
/// single contiguous allocation, of which the first `frames` are active.
///
/// See the [module docs](self) for the capacity/frames distinction.
pub struct Buffer<S: Sample> {
    /// Channel-major storage, `channels * capacity` long.
    data: Box<[S]>,
    channels: usize,
    capacity: usize,
    frames: usize,
}

impl<S: Sample> Buffer<S> {
    /// Allocate a silent buffer of `channels` channels and `capacity` frames,
    /// all of them active. Control thread only: this allocates.
    pub fn new(channels: usize, capacity: usize) -> Self {
        Self {
            data: vec![S::ZERO; channels * capacity].into_boxed_slice(),
            channels,
            capacity,
            frames: capacity,
        }
    }

    /// Number of channels.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Allocated frames per channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of active frames (`<= capacity`).
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Set the number of active frames.
    ///
    /// # Panics
    ///
    /// Panics if `frames > capacity`.
    pub fn set_frames(&mut self, frames: usize) {
        assert!(
            frames <= self.capacity,
            "frames ({frames}) must not exceed capacity ({})",
            self.capacity
        );
        self.frames = frames;
    }

    /// The active region of channel `ch`, read-only.
    pub fn channel(&self, ch: usize) -> &[S] {
        let start = ch * self.capacity;
        &self.data[start..start + self.frames]
    }

    /// The active region of channel `ch`, writable.
    pub fn channel_mut(&mut self, ch: usize) -> &mut [S] {
        let start = ch * self.capacity;
        &mut self.data[start..start + self.frames]
    }

    /// The full `capacity` region of channel `ch`, read-only.
    pub fn channel_full(&self, ch: usize) -> &[S] {
        let start = ch * self.capacity;
        &self.data[start..start + self.capacity]
    }

    /// The full `capacity` region of channel `ch`, writable. Use this to write
    /// past the current active region (e.g. a resampler producing more frames
    /// than it consumed) before publishing the new length with
    /// [`set_frames`](Self::set_frames).
    pub fn channel_full_mut(&mut self, ch: usize) -> &mut [S] {
        let start = ch * self.capacity;
        &mut self.data[start..start + self.capacity]
    }

    /// Iterate the active region of every channel.
    pub fn iter_channels(&self) -> impl Iterator<Item = &[S]> {
        self.data.chunks_exact(self.capacity.max(1)).map(|c| &c[..self.frames])
    }

    /// Iterate the active region of every channel, writable.
    pub fn iter_channels_mut(&mut self) -> impl Iterator<Item = &mut [S]> {
        let frames = self.frames;
        self.data
            .chunks_exact_mut(self.capacity.max(1))
            .map(move |c| &mut c[..frames])
    }

    /// Zero the active region of every channel.
    pub fn silence(&mut self) {
        for ch in self.iter_channels_mut() {
            ch.fill(S::ZERO);
        }
    }

    /// Copy the active region of `src` into this buffer and adopt its frame
    /// count. Channel counts must match.
    ///
    /// # Panics
    ///
    /// Panics if the channel counts differ or `src.frames() > self.capacity()`.
    pub fn copy_from(&mut self, src: &Buffer<S>) {
        assert_eq!(
            self.channels,
            src.channels,
            "copy_from requires matching channel counts"
        );
        self.set_frames(src.frames());
        for ch in 0..self.channels {
            self.channel_mut(ch).copy_from_slice(src.channel(ch));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry() {
        let buf = Buffer::<f32>::new(6, 512);
        assert_eq!(buf.channels(), 6);
        assert_eq!(buf.capacity(), 512);
        assert_eq!(buf.frames(), 512);
        assert_eq!(buf.channel(5).len(), 512);
    }

    #[test]
    fn set_frames_narrows_active_region() {
        let mut buf = Buffer::<f32>::new(2, 8);
        buf.set_frames(3);
        assert_eq!(buf.frames(), 3);
        assert_eq!(buf.channel(0).len(), 3);
        assert_eq!(buf.channel_full(0).len(), 8);
    }

    #[test]
    #[should_panic(expected = "must not exceed capacity")]
    fn set_frames_rejects_overflow() {
        Buffer::<f32>::new(1, 4).set_frames(5);
    }

    #[test]
    fn channels_do_not_alias() {
        let mut buf = Buffer::<f32>::new(2, 4);
        buf.channel_mut(0).fill(1.0);
        buf.channel_mut(1).fill(2.0);
        assert_eq!(buf.channel(0), &[1.0; 4]);
        assert_eq!(buf.channel(1), &[2.0; 4]);
    }

    #[test]
    fn copy_from_adopts_frames() {
        let mut src = Buffer::<f32>::new(2, 8);
        src.set_frames(5);
        src.channel_mut(1).fill(0.5);

        let mut dst = Buffer::<f32>::new(2, 8);
        dst.copy_from(&src);
        assert_eq!(dst.frames(), 5);
        assert_eq!(dst.channel(1), &[0.5; 5]);
    }

    #[test]
    fn zero_capacity_is_harmless() {
        let mut buf = Buffer::<f32>::new(0, 0);
        assert_eq!(buf.iter_channels().count(), 0);
        buf.silence();
    }
}
