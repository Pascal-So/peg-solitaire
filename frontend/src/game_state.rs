use common::{
    coord::{coordinate_to_index, Coord},
    debruijn::de_bruijn_solvable,
    solve_with_bloom_filter, BloomFilter, Position, NR_PEGS,
};

#[derive(Clone)]
pub struct GameState {
    pub pegs: [Peg; NR_PEGS],
    history: Vec<MoveInfo>,
    backwards_solve: Vec<MoveInfo>,
    forwards_solve: Vec<MoveInfo>,
    redo: Vec<MoveInfo>,
}

fn default_pegs() -> [Peg; NR_PEGS] {
    let mut pegs = [Peg {
        coord: Coord::center(),
        alive: true,
    }; NR_PEGS];

    let mut idx = 0;

    for c in Coord::all() {
        if c != Coord::center() {
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

impl GameState {
    pub fn new() -> Self {
        GameState {
            pegs: default_pegs(),
            history: Vec::new(),
            backwards_solve: vec![],
            forwards_solve: vec![],
            redo: Vec::new(),
        }
    }

    /// Check if the move is possible, and if yes, return a token that can be used
    /// to apply the move.
    pub fn check_move(&self, src: Coord, dst: Coord) -> Option<MoveInfo> {
        let mut moved_idx = None;
        let mut eliminated_idx = None;

        let (dx, dy) = dst - src;
        if !(dx.abs() == 2 && dy == 0 || dx == 0 && dy.abs() == 2) {
            log::info!("Moves must be 2-jumps");
            return None;
        }
        let mid = src
            .shift(dx / 2, dy / 2)
            .expect("center between valid positions should be valid");

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
        let pos = self.as_position();
        if !de_bruijn_solvable(pos) {
            return "no!";
        }
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

    pub fn lookup(&self, coord: Coord) -> Option<usize> {
        for (i, p) in self.pegs.iter().enumerate() {
            if p.coord == coord && p.alive {
                return Some(i);
            }
        }

        None
    }

    pub fn edit_toggle_peg(mut self, coord: Coord) -> Self {
        if let Some(idx) = self.lookup(coord) {
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
                let idx = coordinate_to_index(p.coord);
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
        let move_info = gs
            .check_move(Coord::new(2, 0).unwrap(), Coord::center())
            .unwrap();

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
