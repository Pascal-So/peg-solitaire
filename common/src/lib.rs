pub const NR_HOLES: usize = 33;
pub const NR_PEGS: usize = 32;

pub type Coord = (i16, i16);

pub fn coordinate_to_index((x, y): Coord) -> Option<i16> {
    match (y, x) {
        (0..=1, 2..=4) => Some((x - 2) + y * 3),
        (2..=4, 0..=6) => Some(x + (y - 2) * 7 + 6),
        (5..=6, 2..=4) => Some((x - 2) + (y - 5) * 3 + 27),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coords() {
        let mut next_idx = 0;
        for y in 0..7 {
            for x in 0..7 {
                if let Some(idx) = coordinate_to_index((x, y)) {
                    assert_eq!(next_idx, idx);
                    next_idx += 1;
                }
            }
        }

        assert_eq!(next_idx, 33);
    }
}
