use std::ops::{Add, AddAssign, Mul, MulAssign};

use crate::{coord::Coord, Position};

/// Galois Field with four elements.
///
/// We follow the naming conventions used by de Bruijn
#[derive(PartialEq, Eq, Debug, Clone, Copy, Default)]
pub enum GF4 {
    #[default]
    Zero,
    One,
    P,
    Q,
}

impl AddAssign for GF4 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl MulAssign for GF4 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Add for GF4 {
    type Output = GF4;

    fn add(self, rhs: GF4) -> Self::Output {
        match (self, rhs) {
            (GF4::Zero, other) => other,
            (other, GF4::Zero) => other,
            (GF4::One, GF4::One) => GF4::Zero,
            (GF4::One, GF4::P) => GF4::Q,
            (GF4::One, GF4::Q) => GF4::P,
            (GF4::P, GF4::One) => GF4::Q,
            (GF4::P, GF4::P) => GF4::Zero,
            (GF4::P, GF4::Q) => GF4::One,
            (GF4::Q, GF4::One) => GF4::P,
            (GF4::Q, GF4::P) => GF4::One,
            (GF4::Q, GF4::Q) => GF4::Zero,
        }
    }
}

impl Mul for GF4 {
    type Output = GF4;

    fn mul(self, rhs: GF4) -> Self::Output {
        match (self, rhs) {
            (GF4::Zero, _) => GF4::Zero,
            (_, GF4::Zero) => GF4::Zero,
            (GF4::One, other) => other,
            (other, GF4::One) => other,
            (GF4::P, GF4::P) => GF4::Q,
            (GF4::P, GF4::Q) => GF4::One,
            (GF4::Q, GF4::P) => GF4::One,
            (GF4::Q, GF4::Q) => GF4::P,
        }
    }
}

impl GF4 {
    /// Raise the element to a given whole-number power
    fn pow(self, exp: i8) -> Self {
        let exp = exp.rem_euclid(3);

        let mut out = GF4::One;
        for _ in 0..exp {
            out *= self;
        }

        out
    }
}

/// The values of functions A and B of the position.
pub fn de_bruijn_class(pos: Position) -> (GF4, GF4) {
    let mut a = GF4::Zero;
    let mut b = GF4::Zero;

    for x in -3..=3 {
        for y in -3..=3 {
            let Some(coord) = Coord::new(x, y) else {
                continue;
            };

            if !pos.is_occupied(coord) {
                continue;
            }

            let exponent_a = x + y;
            let exponent_b = x - y;

            a += GF4::P.pow(exponent_a);
            b += GF4::P.pow(exponent_b);
        }
    }

    (a, b)
}

/// A necessary, but not sufficient, condition that the given position is solvable.
pub fn de_bruijn_solvable(pos: Position) -> bool {
    de_bruijn_class(pos) == (GF4::One, GF4::One)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Check that equation (1) from "A solitaire game and its relations to a finite field" holds
    #[test]
    fn eq_one() {
        assert_eq!(GF4::One + GF4::P, GF4::P * GF4::P);
        assert_eq!(GF4::P + GF4::P * GF4::P, GF4::One);
    }

    #[test]
    fn start_and_end_classes() {
        assert_eq!(
            de_bruijn_class(Position::default_end()),
            (GF4::One, GF4::One)
        );
        assert_eq!(
            de_bruijn_class(Position::default_start()),
            (GF4::One, GF4::One)
        );
    }

    #[test]
    fn empty_board() {
        assert_eq!(de_bruijn_class(Position(0)), (GF4::Zero, GF4::Zero));
    }

    #[test]
    fn three_in_line_have_no_effect() {
        assert_eq!(de_bruijn_class(Position(7)), (GF4::Zero, GF4::Zero));
    }

    #[test]
    fn exapmle_situation_from_paper() {
        assert_eq!(GF4::P.pow(-1 + 1), GF4::One);
        assert_eq!(GF4::P.pow(0 + 2), GF4::Q);
        assert_eq!(GF4::P.pow(0 - 2), GF4::P);
        assert_eq!(GF4::P.pow(1 + 1), GF4::Q);
        assert_eq!(GF4::P.pow(2 + 1), GF4::One);
        assert_eq!(GF4::P.pow(3 + 2), GF4::Q);
    }
}
