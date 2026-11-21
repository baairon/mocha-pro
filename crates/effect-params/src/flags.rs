//! Parameter flags as a small bit set with this crate's own names.

/// A set of parameter flags. Compared as bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ParamFlags(pub u32);

impl ParamFlags {
    pub const NONE: ParamFlags = ParamFlags(0);
    pub const SUPERVISED: ParamFlags = ParamFlags(1 << 0);
    pub const COLLAPSED: ParamFlags = ParamFlags(1 << 1);
    pub const HIDDEN: ParamFlags = ParamFlags(1 << 2);
    pub const NOT_ANIMATABLE: ParamFlags = ParamFlags(1 << 3);
    pub const REDRAW_ON_CHANGE: ParamFlags = ParamFlags(1 << 4);

    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn contains(self, other: ParamFlags) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn union(self, other: ParamFlags) -> ParamFlags {
        ParamFlags(self.0 | other.0)
    }

    /// Bits set in exactly one of the two sets.
    pub fn diff(self, other: ParamFlags) -> ParamFlags {
        ParamFlags(self.0 ^ other.0)
    }
}
