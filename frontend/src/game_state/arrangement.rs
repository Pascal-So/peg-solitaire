use anyhow::{Context, anyhow, bail};
use common::coord::Coord;
use common::{Direction, NR_HOLES, Position};

use crate::game_state::permutation::Permutation;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Arrangement {
    /// Current permutation of pegs on the board. The `forward` method maps
    /// from hole indices to peg identities.
    permutation: Permutation<NR_HOLES>,
    // todo: reduce size
    alive: [bool; NR_HOLES],
}
impl Arrangement {
    pub fn new() -> Self {
        let mut alive = [true; NR_HOLES];
        alive[16] = false;

        Self {
            permutation: Permutation::new(),
            alive,
        }
    }

    pub fn pegs(&self) -> [Peg; NR_HOLES] {
        let coords: Vec<_> = Coord::all().into_iter().collect();

        std::array::from_fn(|id| {
            let hole_idx = self.permutation.backward(id as u8) as usize;
            Peg {
                coord: coords[hole_idx],
                alive: self.alive[id],
            }
        })
    }

    /// Perform a move from the given source to the destination coordinate.
    ///
    /// This method works for both forwards and backwards moves.
    pub fn perform_move(&mut self, src: Coord, dst: Coord, dir: Direction) -> anyhow::Result<()> {
        let Some(middle) = get_move_middle(src, dst) else {
            bail!("Cannot move between {src} and {dst} since they're not 2 apart.");
        };

        let src_hole_idx = src.hole_idx();
        let dst_hole_idx = dst.hole_idx();
        let src_peg_id = self.permutation.forward(src_hole_idx) as usize;
        let dst_peg_id = self.permutation.forward(dst_hole_idx) as usize;
        let middle_peg_id = self.permutation.forward(middle.hole_idx()) as usize;
        match dir {
            Direction::Forward => if !self.alive[src_peg_id] {
                Err(anyhow!("No peg in source position {src}"))
            } else if self.alive[dst_peg_id] {
                Err(anyhow!("Destination position {dst} is occupied"))
            } else if !self.alive[middle_peg_id] {
                Err(anyhow!("No peg in middle position {middle}"))
            } else {
                Ok(())
            }
            .with_context(|| format!("Cannot perform forward move with src={src}, dst={dst}"))?,
            Direction::Backward => if self.alive[src_peg_id] {
                Err(anyhow!("No hole in source position {src}"))
            } else if !self.alive[dst_peg_id] {
                Err(anyhow!("No peg in destination position {dst}"))
            } else if self.alive[middle_peg_id] {
                Err(anyhow!("No hole in middle position {middle}"))
            } else {
                Ok(())
            }
            .with_context(|| format!("Cannot perform backward move with src={src}, dst={dst}"))?,
        }

        // The peg from the source position moves to the hole in the destination position. Since an `Arrangement`
        // has an invisible peg in every empty hole, we move that invisible peg from the destination position
        // to the soruce position. I.e., we just swap the two positions effectively.
        self.permutation.swap(src_hole_idx, dst_hole_idx);

        // Toggle the peg in the middle positon
        self.toggle_hole(middle);

        Ok(())
    }

    pub fn nr_pegs(&self) -> usize {
        self.alive.iter().fold(0, |i, b| i + *b as usize)
    }

    pub fn toggle_hole(&mut self, coord: Coord) {
        let peg_id = self.permutation.forward(coord.hole_idx());
        self.alive[peg_id as usize] ^= true;
    }

    pub fn as_position(&self) -> Position {
        let mut out = 0;
        for p in self.pegs() {
            if p.alive {
                out |= p.coord.bitmask();
            }
        }
        Position(out)
    }

    pub fn is_occupied(&self, coord: Coord) -> bool {
        let peg_id = self.permutation.forward(coord.hole_idx());
        self.alive[peg_id as usize]
    }
}

/// If src and dst are exactly 2 apart in an axis aligned direction, get the
/// coordinate of the hole between them.
fn get_move_middle(src: Coord, dst: Coord) -> Option<Coord> {
    let (dx, dy) = dst - src;
    if !(dx.abs() == 2 && dy == 0 || dx == 0 && dy.abs() == 2) {
        return None;
    }
    let mid = src
        .shift(dx / 2, dy / 2)
        .expect("center between valid positions should be valid");

    Some(mid)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Peg {
    pub coord: Coord,
    /// Is the peg still on the board?
    pub alive: bool,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_reverse_move() {
        let mut a = Arrangement::new();
        a.toggle_hole(Coord::new(1, 0).unwrap());
        // we now have a "__x" situation starting from the centre

        a.perform_move(
            Coord::center(),
            Coord::new(2, 0).unwrap(),
            Direction::Backward,
        )
        .unwrap();

        let expected = Position::from_ascii([
            "    ###    ",
            "    ###    ",
            "  #######  ",
            "  #####.#  ",
            "  #######  ",
            "    ###    ",
            "    ###    ",
        ]);
        let actual = a.as_position();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_nr_pegs() {
        let mut a = Arrangement::new();

        assert_eq!(a.nr_pegs(), 32);
        a.perform_move(
            Coord::new(2, 0).unwrap(),
            Coord::center(),
            Direction::Forward,
        )
        .unwrap();
        assert_eq!(a.nr_pegs(), 31);
    }

    #[test]
    fn test_initial_position() {
        let pos = Arrangement::new().as_position();
        assert_eq!(pos, Position::default_start());
    }
}
