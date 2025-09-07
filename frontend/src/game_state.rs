use common::{
    coord::Coord, debruijn::de_bruijn_solvable, solve_with_bloom_filter, BloomFilter, Jump,
    Position, NR_PEGS,
};

#[derive(Clone, PartialEq, Eq)]
enum HistoryEntry {
    Edit(Arrangement),
    Move(MoveInfo),
}

#[derive(Clone, PartialEq, Eq)]
struct Arrangement {
    pub pegs: [Peg; NR_PEGS],
}

impl Arrangement {
    fn new() -> Self {
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

        Self { pegs }
    }
}

impl Default for Arrangement {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GameState {
    arrangement: Arrangement,

    history: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,

    backwards_solve: Option<Vec<MoveInfo>>,
    forwards_solve: Option<Vec<MoveInfo>>,
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
            arrangement: Arrangement::default(),
            history: Vec::new(),
            backwards_solve: None,
            forwards_solve: None,
            redo: Vec::new(),
        }
    }

    pub fn pegs(&self) -> impl Iterator<Item = &Peg> {
        self.arrangement.pegs.iter()
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

        for (i, p) in self.pegs().enumerate() {
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
        self.pegs().filter(|peg| peg.alive).count() as i32
    }

    pub fn solvable(&self, filter: &BloomFilter) -> &str {
        let pos = self.as_position();
        if !de_bruijn_solvable(pos) {
            return "no!";
        }
        match solve_with_bloom_filter(self.as_position().normalize(), filter) {
            common::SolveResult::Solved(_) => "yes",
            common::SolveResult::Unsolvable => "no",
            common::SolveResult::TimedOut => "?",
        }
    }

    /// Only manipulate the peg positions, doesn't include history handling.
    fn apply_move_inner(mut self, mut move_info: MoveInfo, reverse: bool) -> Self {
        if reverse {
            std::mem::swap(&mut move_info.src, &mut move_info.dst);
        }
        assert_eq!(
            self.arrangement.pegs[move_info.moved_idx].coord,
            move_info.src
        );
        self.arrangement.pegs[move_info.eliminated_idx].alive = reverse;
        self.arrangement.pegs[move_info.moved_idx].coord = move_info.dst;
        self
    }

    pub fn apply_move(mut self, move_info: MoveInfo) -> Self {
        self = self.apply_move_inner(move_info, false);

        self.history.push(HistoryEntry::Move(move_info));
        if self.redo.pop() != Some(HistoryEntry::Move(move_info)) {
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
        let Some(entry) = self.history.pop() else {
            return self;
        };

        match entry {
            HistoryEntry::Edit(mut arrangement) => {
                std::mem::swap(&mut self.arrangement, &mut arrangement);
                self.redo.push(HistoryEntry::Edit(arrangement));
                self.backwards_solve = None;
                self.forwards_solve = None;
            }
            HistoryEntry::Move(last_move) => {
                self.redo.push(HistoryEntry::Move(last_move));
                self = self.apply_move_inner(last_move, true)
            }
        }

        self
    }

    pub fn redo(mut self) -> Self {
        let Some(entry) = self.redo.pop() else {
            return self;
        };

        match entry {
            HistoryEntry::Edit(mut arrangement) => {
                std::mem::swap(&mut self.arrangement, &mut arrangement);
                self.history.push(HistoryEntry::Edit(arrangement));
                self.backwards_solve = None;
                self.forwards_solve = None;
            }
            HistoryEntry::Move(move_info) => {
                self.history.push(HistoryEntry::Move(move_info));
                self = self.apply_move_inner(move_info, false);
            }
        }

        self
    }

    pub fn lookup(&self, coord: Coord) -> Option<usize> {
        for (i, p) in self.pegs().enumerate() {
            if p.coord == coord && p.alive {
                return Some(i);
            }
        }

        None
    }

    pub fn edit_toggle_peg(mut self, coord: Coord) -> Self {
        let old_arrangement = self.arrangement.clone();
        let mut changed = false;

        if let Some(idx) = self.lookup(coord) {
            self.arrangement.pegs[idx].alive = false;
            changed = true;
        } else {
            for p in self.arrangement.pegs.iter_mut() {
                if !p.alive {
                    p.alive = true;
                    p.coord = coord;
                    changed = true;
                    break;
                }
            }
        }

        if changed {
            // If the last history entry already contains an edit, then we
            // don't append another entry. This has the effect of combining
            // all the edits into one.
            // TODO: add an edit session id or something like that, so that
            // we can have one undo step per edit session.
            if !matches!(self.history.last(), Some(&HistoryEntry::Edit(_))) {
                self.history.push(HistoryEntry::Edit(old_arrangement));
            }
            self.redo.clear();
        }

        self
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
}

/// Given a list of jumps, convert this into moves which don't just consider from
/// where to where we move a peg, but also which peg is being moved.
///
/// Preconditions: jump sequence must be applicable to the given game state
fn convert_jump_sequence_to_moves(mut game_state: GameState, jumps: &[Jump]) -> Vec<MoveInfo> {
    let mut moves = vec![];

    for &jump in jumps {
        let m = game_state
            .check_move(jump.src, jump.dst)
            .expect("jump sequence should be applicable to initial game state");
        moves.push(m);

        game_state = game_state.apply_move(m);
    }

    moves
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    #[test]
    fn test_jump_sequence_to_moves_conversion() {
        let mut game_state = GameState {
            arrangement: Arrangement {
                pegs: [Peg {
                    coord: Coord::center(),
                    alive: false,
                }; NR_PEGS],
            },
            history: vec![],
            redo: vec![],
            backwards_solve: None,
            forwards_solve: None,
        };

        game_state.arrangement.pegs[0] = Peg {
            coord: Coord::center(),
            alive: true,
        };
        game_state.arrangement.pegs[1] = Peg {
            coord: Coord::new(1, 0).unwrap(),
            alive: true,
        };
        game_state.arrangement.pegs[2] = Peg {
            coord: Coord::new(-2, 0).unwrap(),
            alive: true,
        };

        let jumps = [
            Jump::from_coordinate_pair(Coord::new(1, 0).unwrap(), Coord::new(-1, 0).unwrap())
                .unwrap(),
            Jump::from_coordinate_pair(Coord::new(-2, 0).unwrap(), Coord::center()).unwrap(),
        ];

        let moves = convert_jump_sequence_to_moves(game_state.clone(), &jumps);

        for m in moves {
            game_state = game_state.apply_move(m);
        }

        assert_eq!(game_state.as_position(), Position::default_end());
    }
}
