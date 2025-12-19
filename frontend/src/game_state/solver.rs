use common::{
    BloomFilter, Direction, Move, NR_PEGS, Position, SolveResult, solve_with_bloom_filter,
};

/// Store the path to solve the current position.
///
/// This data structure updates the known solve path if a move is taken. If
/// the move follows the known solve path, then we remain on that path and
/// no new solve run is required. If we leave the known path, then we might
/// have to recompute.
#[derive(Debug, Clone, PartialEq)]
pub struct SolvePath {
    path: [Move; NR_PEGS - 1],

    forward: Solvability,
    backward: Solvability,

    current_nr_pegs: i32,
}

impl SolvePath {
    /// Construct a new `SolvePath` that starts at the given position
    pub fn new(pos: Position) -> Self {
        let forward;
        let backward;

        if pos == Position::default_start() {
            forward = Solvability::Solvable;
            backward = Solvability::Solved;
        } else if pos == Position::default_end() {
            forward = Solvability::Solved;
            backward = Solvability::Solvable;
        } else {
            forward = Solvability::Unknown;
            backward = Solvability::Unknown;
        }

        let current_nr_pegs = pos.count();
        assert!(
            (1..=NR_PEGS as i32).contains(&current_nr_pegs),
            "can't solve completely empty or completely full boards"
        );

        Self {
            path: DEFAULT_SOLVE_PATH,
            forward,
            backward,
            current_nr_pegs,
        }
    }

    /// If the current position is solvable in the given direction, return the
    /// next move that should be taken in order to solve the game.
    pub fn next_move(&self, dir: Direction) -> Option<Move> {
        let idx = self.get_index_in_direction(dir);
        let mv = idx.map(|idx| self.path[idx]);
        match dir {
            Direction::Forward => (self.forward == Solvability::Solvable)
                .then_some(mv)
                .flatten(),
            Direction::Backward => (self.backward == Solvability::Solvable)
                .then_some(mv)
                .flatten(),
        }
    }

    /// Check if the backwards and forwards directions are solvable
    pub fn is_solvable(&self) -> (Solvability, Solvability) {
        (self.backward, self.forward)
    }

    /// Apply a move to the current state.
    ///
    /// If the move follows the next move that was already suggested by the
    /// solver, then the solver can keep the current solve path cached and
    /// doesn't have to recompute anything.
    pub fn apply_move(&mut self, mv: Move, dir: Direction) {
        let next_move = self.next_move(dir);
        self.current_nr_pegs += match dir {
            Direction::Forward => -1,
            Direction::Backward => 1,
        };
        assert!((1..=NR_PEGS as i32).contains(&self.current_nr_pegs));

        if next_move == Some(mv) {
            // we moved along the known path

            match dir {
                Direction::Forward => {
                    self.forward = self.get_solvability_in_direction(Direction::Forward);
                    if self.backward.solvable() {
                        self.backward = self.get_solvability_in_direction(Direction::Backward);
                    } else {
                        self.backward = Solvability::Unknown;
                    }
                }
                Direction::Backward => {
                    if self.forward.solvable() {
                        self.forward = self.get_solvability_in_direction(Direction::Forward);
                    } else {
                        self.forward = Solvability::Unknown;
                    }
                    self.backward = self.get_solvability_in_direction(Direction::Backward);
                }
            }
        } else {
            // we left the last computed solve path
            match dir {
                Direction::Forward => {
                    self.forward = Solvability::Unknown;
                    self.backward =
                        self.append_to_solvability(self.backward, Direction::Backward, mv);
                }
                Direction::Backward => {
                    self.forward = self.append_to_solvability(self.forward, Direction::Forward, mv);
                    self.backward = Solvability::Unknown;
                }
            }
        }
    }

    /// After a move has been made away from a known solvable position, then we
    /// know that we can solve backwards from this new position through the old
    /// solvable position. Here we update the `Solvability` and the path
    /// accordingly.
    fn append_to_solvability(
        &mut self,
        solvability: Solvability,
        dir: Direction,
        mv: Move,
    ) -> Solvability {
        if solvability.solvable() {
            let idx = self
                .get_index_in_direction(dir)
                .expect("we can't be at the end of the solve path because we just did a move");
            self.path[idx] = mv;
            Solvability::Solvable
        } else {
            Solvability::Unknown
        }
    }

    /// Get the index into the `path` variable. This returns `None` if there
    /// are no more moves in this direction because we've already reached
    /// the end.
    fn get_index_in_direction(&self, dir: Direction) -> Option<usize> {
        let current_nr_pegs = self.current_nr_pegs as usize;
        match dir {
            Direction::Forward => (current_nr_pegs > 1).then(|| NR_PEGS - current_nr_pegs),
            Direction::Backward => {
                (current_nr_pegs < NR_PEGS).then(|| NR_PEGS - current_nr_pegs - 1)
            }
        }
    }

    /// Given the precondition that the current position is either solvable
    /// or already solved in that direction, and that the path has been
    /// computed, get the `Solvability`
    fn get_solvability_in_direction(&self, dir: Direction) -> Solvability {
        match self.get_index_in_direction(dir) {
            Some(_) => Solvability::Solvable,
            None => Solvability::Solved,
        }
    }

    /// Recompute the solution path if needed.
    ///
    /// The given position must correspond to the position that the SolvePath
    /// state is already in.
    pub fn recompute(&mut self, bloom_filter: &BloomFilter, pos: Position) {
        assert_eq!(pos.count(), self.current_nr_pegs);

        if self.forward == Solvability::Unknown {
            let solve_result = solve_with_bloom_filter(pos, bloom_filter, Direction::Forward, 0).0;

            match solve_result {
                SolveResult::Solved(moves) => {
                    match self.get_index_in_direction(Direction::Forward) {
                        Some(idx) => {
                            let slice = &mut self.path[idx..];
                            slice.copy_from_slice(&moves);
                        }
                        None => {
                            log::warn!(
                                "solvability was set to Unknown even though we're at the end??"
                            )
                        }
                    }
                    self.forward = self.get_solvability_in_direction(Direction::Forward);
                }
                SolveResult::Unsolvable => {
                    self.forward = Solvability::Unsolvable;
                }
                SolveResult::TimedOut => {}
            }
        }
        if self.backward == Solvability::Unknown {
            let solve_result = solve_with_bloom_filter(pos, bloom_filter, Direction::Backward, 0).0;

            match solve_result {
                SolveResult::Solved(mut moves) => {
                    match self.get_index_in_direction(Direction::Backward) {
                        Some(idx) => {
                            let slice = &mut self.path[..=idx];
                            moves.reverse();
                            slice.copy_from_slice(&moves);
                        }
                        None => {
                            log::warn!(
                                "solvability was set to Unknown even though we're at the end??"
                            )
                        }
                    }
                    self.backward = self.get_solvability_in_direction(Direction::Backward);
                }
                SolveResult::Unsolvable => {
                    self.backward = Solvability::Unsolvable;
                }
                SolveResult::TimedOut => {}
            }
        }
    }
}

/// The solve path that passes via the heart shape
const DEFAULT_SOLVE_PATH: [Move; NR_PEGS - 1] = [
    Move::from_raw_coords((0, -2), (0, 0)),
    Move::from_raw_coords((-2, -1), (0, -1)),
    Move::from_raw_coords((-1, -3), (-1, -1)),
    Move::from_raw_coords((-1, 0), (-1, -2)),
    Move::from_raw_coords((1, -3), (-1, -3)),
    Move::from_raw_coords((-1, -3), (-1, -1)),
    Move::from_raw_coords((-1, 2), (-1, 0)),
    Move::from_raw_coords((-3, 1), (-1, 1)),
    Move::from_raw_coords((0, 1), (-2, 1)),
    Move::from_raw_coords((-3, -1), (-3, 1)),
    Move::from_raw_coords((-3, 1), (-1, 1)),
    Move::from_raw_coords((2, 1), (0, 1)),
    Move::from_raw_coords((1, 3), (1, 1)),
    Move::from_raw_coords((1, 0), (1, 2)),
    Move::from_raw_coords((-1, 3), (1, 3)),
    Move::from_raw_coords((1, 3), (1, 1)),
    Move::from_raw_coords((1, -2), (1, 0)),
    Move::from_raw_coords((3, -1), (1, -1)),
    Move::from_raw_coords((0, -1), (2, -1)),
    Move::from_raw_coords((3, 1), (3, -1)),
    Move::from_raw_coords((3, -1), (1, -1)),
    Move::from_raw_coords((0, 1), (2, 1)),
    Move::from_raw_coords((2, 1), (2, -1)),
    Move::from_raw_coords((2, -1), (0, -1)),
    Move::from_raw_coords((0, -1), (-2, -1)),
    Move::from_raw_coords((-2, -1), (-2, 1)),
    Move::from_raw_coords((-2, 1), (0, 1)),
    Move::from_raw_coords((0, 0), (-2, 0)),
    Move::from_raw_coords((0, 2), (0, 0)),
    Move::from_raw_coords((1, 0), (-1, 0)),
    Move::from_raw_coords((-2, 0), (0, 0)),
];

/// Is the current position solvable, i.e. does a path exist
/// from the current positon to the end?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Solvability {
    /// Yes, the position is solvable.
    Solvable,
    /// Yes, we have already reached the target position.
    Solved,
    /// No, the position is not solvable.
    Unsolvable,
    /// Maybe. Either we haven't computed the solution path yet, or the
    /// solver encountered an issue.
    Unknown,
}

impl Solvability {
    /// Check if the position is either solvable or already solved.
    pub fn solvable(self) -> bool {
        match self {
            Solvability::Solvable => true,
            Solvability::Solved => true,
            Solvability::Unsolvable => false,
            Solvability::Unknown => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use common::Move;

    use super::*;
    #[test]
    fn test_forwards_backwards_move_preserves_solution_path() {
        let mut solve_path = SolvePath::new(Position::default_start());

        let mv = Move::from_raw_coords((0, -2), (0, 0));
        solve_path.apply_move(mv, Direction::Forward);
        assert!(matches!(solve_path.next_move(Direction::Forward), Some(_)));
        assert_eq!(solve_path.next_move(Direction::Backward), Some(mv));

        solve_path.apply_move(mv, Direction::Backward);
        assert_eq!(solve_path.next_move(Direction::Forward), Some(mv));
        assert_eq!(solve_path.next_move(Direction::Backward), None);
    }

    #[test]
    fn test_moving_off_path_invalidates_cached_path() {
        let mut solve_path = SolvePath::new(Position::default_start());

        let second_move = Move::from_raw_coords((2, 0), (0, 0));
        solve_path.apply_move(second_move, Direction::Forward);

        assert_eq!(
            solve_path.is_solvable(),
            (Solvability::Solvable, Solvability::Unknown)
        );

        // Path back to the start should be known because that's where we
        // came from.
        assert_eq!(solve_path.next_move(Direction::Backward), Some(second_move));
    }

    #[test]
    fn test_backwards_from_unknown_is_unknown() {
        let pos = Position::from_ascii([
            "    ###    ",
            "    .#.    ",
            "  ..#..##  ",
            "  ....#.#  ",
            "  .##.#..  ",
            "    #..    ",
            "    ###    ",
        ]);
        let mut solve_path = SolvePath::new(pos);
        let mv = Move::from_raw_coords((-1, -1), (1, -1));

        solve_path.apply_move(mv, Direction::Backward);
        assert_eq!(solve_path.forward, Solvability::Unknown);
    }

    #[test]
    #[ignore]
    fn test_undoing_does_not_magically_make_forward_path_solvable() {
        let bf =
            BloomFilter::load_from_file("../precompute/filters/modulo/filter_502115651_1_norm.bin");

        // We start at a position that is unsolvable in
        // the forwards direction.
        let pos = Position::from_ascii([
            "    ###    ",
            "    .#.    ",
            "  ..#..##  ",
            "  ....#.#  ",
            "  .##.#..  ",
            "    #..    ",
            "    ###    ",
        ]);
        let mut solve_path = SolvePath::new(pos);
        solve_path.recompute(&bf, pos);
        assert_eq!(solve_path.forward, Solvability::Unsolvable);

        // Then move one step forwards.
        let mv = Move::from_raw_coords((1, 1), (1, -1));
        solve_path.apply_move(mv, Direction::Forward);
        solve_path.recompute(&bf, pos.apply_move(mv));
        assert_eq!(solve_path.forward, Solvability::Unsolvable);

        // Then move back again. Note that we don't recompute the forwards
        // path again here.
        solve_path.apply_move(mv, Direction::Backward);
        assert_eq!(solve_path.forward, Solvability::Unknown);

        // check if forwards is still unsolvable once we recompute the paths
        solve_path.recompute(&bf, pos);
        assert_eq!(solve_path.forward, Solvability::Unsolvable);
    }
}
