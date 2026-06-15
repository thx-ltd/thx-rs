//! THX channel masks.
//!
//! The bit layout mirrors the Microsoft `WAVEFORMATEXTENSIBLE::dwChannelMask`
//! convention (which SMPTE/ITU layouts also follow), so masks round-trip
//! cleanly with WAV files and OS audio APIs.

use bitflags::bitflags;

bitflags! {
    /// A set of speaker positions, identifying both *which* channels are present
    /// and, by ascending bit order, *what order* they appear in a planar buffer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ChannelMask: u32 {
        const FRONT_LEFT            = 0x0000_0001;
        const FRONT_RIGHT           = 0x0000_0002;
        const FRONT_CENTER          = 0x0000_0004;
        const LOW_FREQUENCY         = 0x0000_0008;
        const BACK_LEFT             = 0x0000_0010;
        const BACK_RIGHT            = 0x0000_0020;
        const FRONT_LEFT_OF_CENTER  = 0x0000_0040;
        const FRONT_RIGHT_OF_CENTER = 0x0000_0080;
        const BACK_CENTER           = 0x0000_0100;
        const SIDE_LEFT             = 0x0000_0200;
        const SIDE_RIGHT            = 0x0000_0400;
        const TOP_CENTER            = 0x0000_0800;
        const TOP_FRONT_LEFT        = 0x0000_1000;
        const TOP_FRONT_CENTER      = 0x0000_2000;
        const TOP_FRONT_RIGHT       = 0x0000_4000;
        const TOP_BACK_LEFT         = 0x0000_8000;
        const TOP_BACK_CENTER       = 0x0001_0000;
        const TOP_BACK_RIGHT        = 0x0002_0000;
    }
}

impl ChannelMask {
    // Standard speaker layouts, mirroring the `KSAUDIO_SPEAKER_*` definitions in
    // Windows `ksmedia.h`.

    /// No speakers (`KSAUDIO_SPEAKER_DIRECTOUT`).
    pub const MASK_DIRECT_OUT: Self = Self::empty();

    /// 1.0 mono: front center (`KSAUDIO_SPEAKER_MONO`).
    pub const MASK_MONO: Self = Self::FRONT_CENTER;

    /// 1.1: front center + LFE (`KSAUDIO_SPEAKER_1POINT1`).
    pub const MASK_1_1: Self =
        Self::from_bits_retain(Self::FRONT_CENTER.bits() | Self::LOW_FREQUENCY.bits());

    /// 2.0 stereo: front left + front right (`KSAUDIO_SPEAKER_STEREO`).
    pub const MASK_STEREO: Self =
        Self::from_bits_retain(Self::FRONT_LEFT.bits() | Self::FRONT_RIGHT.bits());

    /// 2.1: [`MASK_STEREO`](Self::MASK_STEREO) + LFE (`KSAUDIO_SPEAKER_2POINT1`).
    pub const MASK_2_1: Self =
        Self::from_bits_retain(Self::MASK_STEREO.bits() | Self::LOW_FREQUENCY.bits());

    /// 3.0: [`MASK_STEREO`](Self::MASK_STEREO) + center (`KSAUDIO_SPEAKER_3POINT0`).
    pub const MASK_3_0: Self =
        Self::from_bits_retain(Self::MASK_STEREO.bits() | Self::FRONT_CENTER.bits());

    /// 3.1: [`MASK_STEREO`](Self::MASK_STEREO) + center + LFE (`KSAUDIO_SPEAKER_3POINT1`).
    pub const MASK_3_1: Self =
        Self::from_bits_retain(Self::MASK_3_0.bits() | Self::LOW_FREQUENCY.bits());

    /// 4.0 quad: front pair + back pair (`KSAUDIO_SPEAKER_QUAD`).
    pub const MASK_QUAD: Self = Self::from_bits_retain(
        Self::MASK_STEREO.bits() | Self::BACK_LEFT.bits() | Self::BACK_RIGHT.bits(),
    );

    /// 4.0 Surround: front left/right/center + back center (`KSAUDIO_SPEAKER_SURROUND`).
    pub const MASK_4_0_SURROUND: Self =
        Self::from_bits_retain(Self::MASK_3_0.bits() | Self::BACK_CENTER.bits());

    /// 5.0: front left/right/center + side pair (`KSAUDIO_SPEAKER_5POINT0`).
    pub const MASK_5_0: Self = Self::from_bits_retain(
        Self::MASK_3_0.bits() | Self::SIDE_LEFT.bits() | Self::SIDE_RIGHT.bits(),
    );

    /// 5.1: front left/right/center + LFE + back pair
    /// (`KSAUDIO_SPEAKER_5POINT1`, a.k.a. `5POINT1_BACK`).
    pub const MASK_5_1: Self = Self::from_bits_retain(
        Self::MASK_3_1.bits() | Self::BACK_LEFT.bits() | Self::BACK_RIGHT.bits(),
    );

    /// 5.1 with side speakers: front left/right/center + LFE + side pair
    /// (`KSAUDIO_SPEAKER_5POINT1_SURROUND`, a.k.a. `5POINT1_SIDE`).
    pub const MASK_5_1_SIDE: Self = Self::from_bits_retain(
        Self::MASK_3_1.bits() | Self::SIDE_LEFT.bits() | Self::SIDE_RIGHT.bits(),
    );

    /// 7.0: [`MASK_5_0`](Self::MASK_5_0) + back pair
    /// (`KSAUDIO_SPEAKER_7POINT0`).
    pub const MASK_7_0: Self = Self::from_bits_retain(
        Self::MASK_5_0.bits() | Self::BACK_LEFT.bits() | Self::BACK_RIGHT.bits(),
    );

    /// 7.1: [`MASK_5_1`](Self::MASK_5_1) + side pair
    /// (`KSAUDIO_SPEAKER_7POINT1_SURROUND`).
    pub const MASK_7_1: Self = Self::from_bits_retain(
        Self::MASK_5_1.bits() | Self::SIDE_LEFT.bits() | Self::SIDE_RIGHT.bits(),
    );

    /// 7.1 "wide" (legacy): front left/right/center + LFE + back pair +
    /// front-of-center pair (`KSAUDIO_SPEAKER_7POINT1`, a.k.a. `7POINT1_WIDE`).
    /// Obsolete, it lacks side speakers, prefer [`MASK_7_1`](Self::MASK_7_1).
    pub const MASK_7_1_WIDE: Self = Self::from_bits_retain(
        Self::MASK_5_1.bits()
            | Self::FRONT_LEFT_OF_CENTER.bits()
            | Self::FRONT_RIGHT_OF_CENTER.bits(),
    );

    /// 7.1.4: [`MASK_7_1`](Self::MASK_7_1) plus four height speakers
    /// (top front pair + top back pair).
    pub const MASK_7_1_4: Self = Self::from_bits_retain(
        Self::MASK_7_1.bits()
            | Self::TOP_FRONT_LEFT.bits()
            | Self::TOP_FRONT_RIGHT.bits()
            | Self::TOP_BACK_LEFT.bits()
            | Self::TOP_BACK_RIGHT.bits(),
    );

    /// The number of channels in this mask.
    pub const fn channel_count(self) -> usize {
        self.bits().count_ones() as usize
    }

    /// Iterates the speaker positions present in this mask in canonical
    /// (ascending-bit) order, the same order channels should appear in a planar
    /// [`Buffer`](crate::Buffer). Each item is a single-position mask.
    pub fn positions(self) -> impl Iterator<Item = ChannelMask> {
        self.iter()
    }

    /// Maps a single speaker `position` to its channel index in this layout.
    /// Inverse of [`positions`](Self::positions) (which maps index → position).
    ///
    /// Returns `None` if `position` is absent from the layout, or is not a
    /// single position (a multi-bit, whole-layout mask).
    pub fn index_of(self, position: ChannelMask) -> Option<usize> {
        if position.bits().count_ones() != 1 || !self.contains(position) {
            return None;
        }

        // Count set positions below the queried bit.
        let below = self.bits() & (position.bits() - 1);
        Some(below.count_ones() as usize)
    }
}

impl serde::Serialize for ChannelMask {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> serde::Deserialize<'de> for ChannelMask {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        u32::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_counts() {
        assert_eq!(ChannelMask::MASK_DIRECT_OUT.channel_count(), 0);
        assert_eq!(ChannelMask::MASK_MONO.channel_count(), 1);
        assert_eq!(ChannelMask::MASK_1_1.channel_count(), 2);
        assert_eq!(ChannelMask::MASK_STEREO.channel_count(), 2);
        assert_eq!(ChannelMask::MASK_2_1.channel_count(), 3);
        assert_eq!(ChannelMask::MASK_3_0.channel_count(), 3);
        assert_eq!(ChannelMask::MASK_3_1.channel_count(), 4);
        assert_eq!(ChannelMask::MASK_QUAD.channel_count(), 4);
        assert_eq!(ChannelMask::MASK_4_0_SURROUND.channel_count(), 4);
        assert_eq!(ChannelMask::MASK_5_0.channel_count(), 5);
        assert_eq!(ChannelMask::MASK_5_1.channel_count(), 6);
        assert_eq!(ChannelMask::MASK_5_1_SIDE.channel_count(), 6);
        assert_eq!(ChannelMask::MASK_7_0.channel_count(), 7);
        assert_eq!(ChannelMask::MASK_7_1.channel_count(), 8);
        assert_eq!(ChannelMask::MASK_7_1_WIDE.channel_count(), 8);
        assert_eq!(ChannelMask::MASK_7_1_4.channel_count(), 12);
    }

    #[test]
    fn positions_iterate_in_planar_order() {
        use ChannelMask as C;

        assert_eq!(
            C::MASK_QUAD.positions().collect::<Vec<_>>(),
            [C::FRONT_LEFT, C::FRONT_RIGHT, C::BACK_LEFT, C::BACK_RIGHT],
        );

        assert_eq!(
            C::MASK_5_1.positions().collect::<Vec<_>>(),
            [
                C::FRONT_LEFT,
                C::FRONT_RIGHT,
                C::FRONT_CENTER,
                C::LOW_FREQUENCY,
                C::BACK_LEFT,
                C::BACK_RIGHT,
            ],
        );

        assert_eq!(
            C::MASK_7_1.positions().collect::<Vec<_>>(),
            [
                C::FRONT_LEFT,
                C::FRONT_RIGHT,
                C::FRONT_CENTER,
                C::LOW_FREQUENCY,
                C::BACK_LEFT,
                C::BACK_RIGHT,
                C::SIDE_LEFT,
                C::SIDE_RIGHT,
            ],
        );

        assert_eq!(
            C::MASK_7_1_4.positions().collect::<Vec<_>>(),
            [
                C::FRONT_LEFT,
                C::FRONT_RIGHT,
                C::FRONT_CENTER,
                C::LOW_FREQUENCY,
                C::BACK_LEFT,
                C::BACK_RIGHT,
                C::SIDE_LEFT,
                C::SIDE_RIGHT,
                C::TOP_FRONT_LEFT,
                C::TOP_FRONT_RIGHT,
                C::TOP_BACK_LEFT,
                C::TOP_BACK_RIGHT,
            ],
        );
    }

    #[test]
    fn index_of_follows_bit_order() {
        use ChannelMask as C;

        // Expected planar order per layout, hard-coded (not derived from
        // `positions`) so a bug there can't mask a bug in `index_of`. The n-th
        // position must report index n.
        let cases: &[(C, &[C])] = &[
            (
                C::MASK_QUAD,
                &[C::FRONT_LEFT, C::FRONT_RIGHT, C::BACK_LEFT, C::BACK_RIGHT],
            ),
            (
                C::MASK_5_1,
                &[
                    C::FRONT_LEFT,
                    C::FRONT_RIGHT,
                    C::FRONT_CENTER,
                    C::LOW_FREQUENCY,
                    C::BACK_LEFT,
                    C::BACK_RIGHT,
                ],
            ),
            (
                C::MASK_7_1,
                &[
                    C::FRONT_LEFT,
                    C::FRONT_RIGHT,
                    C::FRONT_CENTER,
                    C::LOW_FREQUENCY,
                    C::BACK_LEFT,
                    C::BACK_RIGHT,
                    C::SIDE_LEFT,
                    C::SIDE_RIGHT,
                ],
            ),
            (
                C::MASK_7_1_4,
                &[
                    C::FRONT_LEFT,
                    C::FRONT_RIGHT,
                    C::FRONT_CENTER,
                    C::LOW_FREQUENCY,
                    C::BACK_LEFT,
                    C::BACK_RIGHT,
                    C::SIDE_LEFT,
                    C::SIDE_RIGHT,
                    C::TOP_FRONT_LEFT,
                    C::TOP_FRONT_RIGHT,
                    C::TOP_BACK_LEFT,
                    C::TOP_BACK_RIGHT,
                ],
            ),
        ];

        for (mask, expected) in cases {
            for (i, position) in expected.iter().enumerate() {
                assert_eq!(
                    mask.index_of(*position),
                    Some(i),
                    "{position:?} in {mask:?}"
                );
            }
        }

        // A position absent from the layout has no index.
        assert_eq!(C::MASK_QUAD.index_of(C::FRONT_CENTER), None);
        assert_eq!(C::MASK_5_1.index_of(C::SIDE_LEFT), None);

        // A multi-bit (whole-layout) mask is not a single position.
        assert_eq!(C::MASK_5_1.index_of(C::MASK_STEREO), None);
    }
}
