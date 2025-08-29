use common::{solve_with_bloom_filter, BloomFilter, Coord, Position, NR_HOLES, NR_PEGS};

#[rustfmt::skip]
pub static HOLE_COORDS: [Coord; NR_HOLES] = [
    (2, 0), (3, 0), (4, 0),
    (2, 1), (3, 1), (4, 1),
    (0, 2), (1, 2), (2, 2), (3, 2), (4, 2), (5, 2), (6, 2),
    (0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (6, 3),
    (0, 4), (1, 4), (2, 4), (3, 4), (4, 4), (5, 4), (6, 4),
    (2, 5), (3, 5), (4, 5),
    (2, 6), (3, 6), (4, 6),
];

#[derive(Clone)]
pub struct GameState {
    pub pegs: [Peg; NR_PEGS],
    history: Vec<MoveInfo>,
    redo: Vec<MoveInfo>,
}

fn default_pegs() -> [Peg; NR_PEGS] {
    let mut pegs = [Peg {
        coord: (0, 0),
        alive: true,
    }; NR_PEGS];

    let mut idx = 0;
    for &c in &HOLE_COORDS {
        if c != (3, 3) {
            pegs[idx].coord = c;
            idx += 1;
        }
    }

    pegs
}

/// Acts as a token, proving that the move is possible. This token is
/// not completely fool-proof, since it's possible that the game state
/// has been changed in between, but as long as tokens are immediately
/// used, this is fine.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct MoveInfo {
    moved_idx: usize,
    eliminated_idx: usize,
    src: Coord,
    dst: Coord,
}

pub enum LookupResult {
    Invalid,
    Peg(usize),
    Empty,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            pegs: default_pegs(),
            history: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Check if the move is possible, and if yes, return a token that can be used
    /// to apply the move.
    pub fn check_move(&self, src: Coord, dst: Coord) -> Option<MoveInfo> {
        let mut moved_idx = None;
        let mut eliminated_idx = None;

        let dx = dst.0 - src.0;
        let dy = dst.1 - src.1;
        if !(dx.abs() == 2 && dy == 0 || dx == 0 && dy.abs() == 2) {
            log::info!("Moves must be 2-jumps");
            return None;
        }
        let mid = ((src.0 + dst.0) / 2, (src.1 + dst.1) / 2);

        for (i, p) in self.pegs.iter().enumerate() {
            if !p.alive {
                continue;
            }

            if p.coord == src {
                moved_idx = Some(i);
            } else if p.coord == mid {
                eliminated_idx = Some(i);
            } else if p.coord == dst {
                log::info!("dst already occupied");
                return None;
            }
        }

        Some(MoveInfo {
            moved_idx: moved_idx?,
            eliminated_idx: eliminated_idx?,
            src,
            dst,
        })
    }

    pub fn nr_pegs(&self) -> i32 {
        self.pegs.iter().filter(|peg| peg.alive).count() as i32
    }

    pub fn solvable(&self, filter: &BloomFilter) -> &str {
        match solve_with_bloom_filter(self.as_position().normalize(), filter) {
            common::SolveResult::Solved => "yes",
            common::SolveResult::Unsolvable => "no",
            common::SolveResult::TimedOut => "?",
        }
    }

    /// Only manipulate the peg positions, doesn't include history handling.
    fn apply_move_inner(mut self, mut move_info: MoveInfo, reverse: bool) -> Self {
        if reverse {
            std::mem::swap(&mut move_info.src, &mut move_info.dst);
        }
        assert_eq!(self.pegs[move_info.moved_idx].coord, move_info.src);
        self.pegs[move_info.eliminated_idx].alive = reverse;
        self.pegs[move_info.moved_idx].coord = move_info.dst;
        self
    }

    pub fn apply_move(mut self, move_info: MoveInfo) -> Self {
        self = self.apply_move_inner(move_info, false);

        self.history.push(move_info);
        if self.redo.pop() != Some(move_info) {
            self.redo.clear();
        }

        self
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(mut self) -> Self {
        let Some(last_move) = self.history.pop() else {
            return self;
        };

        self.redo.push(last_move);
        self.apply_move_inner(last_move, true)
    }

    pub fn redo(mut self) -> Self {
        let Some(move_info) = self.redo.pop() else {
            return self;
        };

        self.history.push(move_info);
        self.apply_move_inner(move_info, false)
    }

    pub fn lookup(&self, coord: Coord) -> LookupResult {
        if (coord.0 < 2 || coord.0 > 4) && (coord.1 < 2 || coord.1 > 4) {
            return LookupResult::Invalid;
        }

        for (i, p) in self.pegs.iter().enumerate() {
            if p.coord == coord && p.alive {
                return LookupResult::Peg(i);
            }
        }

        LookupResult::Empty
    }

    pub fn edit_toggle_peg(mut self, coord: Coord) -> Self {
        if let LookupResult::Peg(idx) = self.lookup(coord) {
            self.pegs[idx].alive = false;
            self.history.clear();
            self.redo.clear();
            self
        } else {
            for p in self.pegs.iter_mut() {
                if !p.alive {
                    p.alive = true;
                    p.coord = coord;
                    self.history.clear();
                    self.redo.clear();
                    break;
                }
            }
            self
        }
    }

    pub fn as_position(&self) -> Position {
        let mut out = 0;
        for p in self.pegs.iter() {
            if p.alive {
                let idx = common::coordinate_to_index(p.coord)
                    .expect("pegs should only have valid coordinates");
                out |= 1 << idx as u64;
            }
        }
        Position(out)
    }
}

#[derive(Clone, Copy)]
pub struct Peg {
    pub coord: Coord,
    pub alive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_position() {
        let gs = GameState::new();

        assert_eq!(gs.as_position(), Position::default_start());
    }

    #[test]
    fn test_move() {
        let gs = GameState::new();
        let move_info = gs.check_move((5, 3), (3, 3)).unwrap();

        let gs = gs.apply_move(move_info);
        gs.as_position().print();

        // TODO: The coordinates in GameState and Position appear to be mirrored, fix this at some point.
        let expected = Position::from_ascii([
            "    ###    ",
            "    ###    ",
            "  #######  ",
            "  #..####  ",
            "  #######  ",
            "    ###    ",
            "    ###    ",
        ]);
        assert_eq!(gs.as_position(), expected);
    }
}
