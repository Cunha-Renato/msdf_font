//! Courtesy of fdsm.
//! See https://gitlab.com/Kyarei/fdsm/-/blob/main/fdsm/src/color.rs
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

// The number of channels used.
const NUM_CHANNELS: usize = 3;

// The color of an edge.
/// See Section 3.3 of (Chlumský, 2015) for more information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct EdgeColor(u8);
impl EdgeColor {
    /// A helepr function for choosing the next color when performing edge coloring.
    ///
    /// See [`crate::shape::ColoredContour::edge_coloring_simple`] for more details.
    // See https://github.com/Chlumsky/msdfgen/blob/master/core/edge-coloring.cpp#L28
    pub(crate) fn switch(self, seed: &mut usize, banned: EdgeColor) -> EdgeColor {
        let combined = self & banned;
        if matches!(combined, Self::RED | Self::GREEN | Self::BLUE) {
            !combined
        } else if matches!(self, Self::BLACK | Self::WHITE) {
            let color = [Self::CYAN, Self::MAGENTA, Self::YELLOW][*seed % 3];
            *seed /= 3;
            color
        } else {
            let shifted = self.0 << (1 + (*seed & 1));
            *seed >>= 1;
            Self::new(shifted | (shifted >> 3))
        }
    }

    /// Returns true if the red channel is on for this color.
    #[inline]
    pub(crate) fn has_red(&self) -> bool {
        (self.0 & 1) != 0
    }

    /// Returns true if the green channel is on for this color.
    #[inline]
    pub(crate) fn has_green(&self) -> bool {
        (self.0 & 2) != 0
    }

    /// Returns true if the blue channel is on for this color.
    #[inline]
    pub(crate) fn has_blue(&self) -> bool {
        (self.0 & 4) != 0
    }

    /// Returns true if this color has at least two channels set.
    #[inline]
    pub(crate) fn is_bright(&self) -> bool {
        (self.0 & (self.0 - 1)) != 0
    }

    /// Creates a new color from the underlying bits.
    ///
    /// Numbering the bits such that 0 is the least significant bit:
    ///
    /// * Bit 0 corresponds to the red channel.
    /// * Bit 1 corresponds to the green channel.
    /// * Bit 2 corresponds to the blue channel.
    ///
    /// Bits 3 and above are truncated in the resulting color.
    #[inline]
    fn new(value: u8) -> Self {
        Self(value & ((1 << NUM_CHANNELS) - 1))
    }

    pub(crate) const BLACK: EdgeColor = EdgeColor(0);
    pub(crate) const WHITE: EdgeColor = EdgeColor(7);
    pub(crate) const YELLOW: EdgeColor = EdgeColor(3);
    pub(crate) const CYAN: EdgeColor = EdgeColor(6);
    pub(crate) const MAGENTA: EdgeColor = EdgeColor(5);
    pub(crate) const RED: EdgeColor = EdgeColor(1);
    pub(crate) const GREEN: EdgeColor = EdgeColor(2);
    pub(crate) const BLUE: EdgeColor = EdgeColor(4);
}
impl BitAnd for EdgeColor {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for EdgeColor {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}
impl BitOr for EdgeColor {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for EdgeColor {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}
impl BitXor for EdgeColor {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for EdgeColor {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}
impl Not for EdgeColor {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(self.0 ^ 7)
    }
}
