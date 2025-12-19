use std::{fmt::Display, ops::Sub};

/// A hole coordinate on the board.
/// Centre hole is 0,0, x increases to the right, y increases downwards.
///
/// Invariant: can only represent valid coordinates
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Coord {
    x: i8,
    y: i8,
}

impl Sub for Coord {
    type Output = (i8, i8);

    fn sub(self, rhs: Self) -> Self::Output {
        self.subtract(rhs)
    }
}

impl Coord {
    pub const fn new(x: i8, y: i8) -> Option<Self> {
        let coord = Coord { x, y };
        if coord.is_valid() { Some(coord) } else { None }
    }

    pub const fn center() -> Self {
        Coord { x: 0, y: 0 }
    }

    /// Get the offset from `other` to `self`
    pub const fn subtract(self, other: Self) -> (i8, i8) {
        (self.x - other.x, self.y - other.y)
    }

    pub const fn hole_idx(self) -> u8 {
        let x = self.x + 3;
        let y = self.y + 3;
        let idx = match (y, x) {
            (0..=1, 2..=4) => (x - 2) + y * 3,
            (2..=4, 0..=6) => x + (y - 2) * 7 + 6,
            (5..=6, 2..=4) => (x - 2) + (y - 5) * 3 + 27,
            _ => unreachable!(),
        };
        idx as u8
    }

    pub const fn bitmask(self) -> u64 {
        1u64 << self.hole_idx()
    }

    pub fn rotate(self) -> Coord {
        Coord {
            x: -self.y,
            y: self.x,
        }
    }

    pub const fn shift(self, x: i8, y: i8) -> Option<Coord> {
        Self::new(self.x + x, self.y + y)
    }

    const fn is_valid(self) -> bool {
        matches!(
            (self.x, self.y),
            (-3..=-2, -1..=1) | (-1..=1, -3..=3) | (2..=3, -1..=1)
        )
    }

    pub fn all() -> impl IntoIterator<Item = Self> {
        (-3..=3).flat_map(|y| (-3..=3).filter_map(move |x| Coord::new(x, y)))
    }

    pub fn x(self) -> i8 {
        self.x
    }
    pub fn y(self) -> i8 {
        self.y
    }
}

impl Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Default for Coord {
    fn default() -> Self {
        Self::center()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topleft_should_be_lowest_bit() {
        assert_eq!(Coord::new(-1, -3).unwrap().bitmask(), 1);
    }
}
