use common::{
    BloomFilter, Direction, NR_PEGS, Position, SolveResult, coord::Coord, solve_with_bloom_filter,
};

use crate::game_state::game_state::Move;

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

    current_nr_pegs: usize,
}

impl SolvePath {
    /// Construct a new `SolvePath` that starts at the given position
    pub fn new(pos: Position) -> Self {
        let current_nr_pegs = pos.count() as usize;
        let forward;
        let backward;

        if pos == Position::default_start() {
            forward = Solvability::Solvable(DEFAULT_SOLVE_PATH[0]);
            backward = Solvability::Solved;
        } else if pos == Position::default_end() {
            forward = Solvability::Solved;
            backward = Solvability::Solvable(DEFAULT_SOLVE_PATH[NR_PEGS - 2]);
        } else {
            forward = Solvability::Unknown;
            backward = Solvability::Unknown;
        }

        Self {
            path: DEFAULT_SOLVE_PATH,
            forward,
            backward,
            current_nr_pegs,
        }
    }

    /// Query if the current position is solvable in the given direction, and
    /// if so, return the next move that should be taken towards solving.
    pub fn next_move(&self, dir: Direction) -> Solvability {
        match dir {
            Direction::Forward => self.forward,
            Direction::Backward => self.backward,
        }
    }

    /// Apply a move to the current state.
    ///
    /// If the move follows the next move that was already suggested by the
    /// solver, then the solver can keep the current solve path cached and
    /// doesn't have to recompute anything.
    pub fn apply_move(&mut self, mv: Move, dir: Direction) {
        match dir {
            Direction::Forward => {
                assert!(self.current_nr_pegs > 1);
                self.current_nr_pegs -= 1;

                if self.forward == Solvability::Solvable(mv) {
                    // we moved along the known path
                    self.forward = self.get_solvability_in_direction(Direction::Forward);
                    self.backward = self.get_solvability_in_direction(Direction::Backward);
                } else {
                    // we left the last computed solve path
                    self.forward = Solvability::Unknown;
                    self.backward =
                        self.append_to_solvability(self.backward, Direction::Backward, mv);
                }
            }
            Direction::Backward => {
                assert!(self.current_nr_pegs < NR_PEGS);
                self.current_nr_pegs += 1;

                if self.backward == Solvability::Solvable(mv) {
                    // we moved along the known path
                    self.forward = self.get_solvability_in_direction(Direction::Forward);
                    self.backward = self.get_solvability_in_direction(Direction::Backward);
                } else {
                    // we left the last computed solve path
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
        match solvability {
            Solvability::Solvable(_) | Solvability::Solved => {
                let idx = self
                    .get_index_in_direction(dir)
                    .expect("we can't be at the end of the solve path because we just did a move");
                self.path[idx] = mv;
                Solvability::Solvable(mv)
            }
            _ => Solvability::Unknown,
        }
    }

    /// Get the index into the `path` variable. This returns `None` if there
    /// are no more moves in this direction because we've already reached
    /// the end.
    fn get_index_in_direction(&self, dir: Direction) -> Option<usize> {
        match dir {
            Direction::Forward => {
                (self.current_nr_pegs > 1).then(|| NR_PEGS - self.current_nr_pegs)
            }
            Direction::Backward => {
                (self.current_nr_pegs < NR_PEGS).then(|| NR_PEGS - self.current_nr_pegs - 1)
            }
        }
    }

    /// Given the precondition that the current position is either solvable
    /// or already solved in that direction, and that the path has been
    /// computed, get the `Solvability`
    fn get_solvability_in_direction(&self, dir: Direction) -> Solvability {
        match self.get_index_in_direction(dir) {
            Some(idx) => Solvability::Solvable(self.path[idx]),
            None => Solvability::Solved,
        }
    }

    /// Recompute the solution path if needed.
    ///
    /// The given position must correspond to the position that the SolvePath
    /// state is already in.
    pub fn recompute(&mut self, bloom_filter: &BloomFilter, pos: Position) {
        assert_eq!(pos.count() as usize, self.current_nr_pegs);

        if self.forward == Solvability::Unknown {
            log::info!("recomputint forward");
            let solve_result = solve_with_bloom_filter(pos, bloom_filter, Direction::Forward, 0).0;

            match solve_result {
                SolveResult::Solved(jumps) => {
                    log::info!("solved");
                    match self.get_index_in_direction(Direction::Forward) {
                        Some(idx) => {
                            let slice = &mut self.path[idx..];
                            assert_eq!(slice.len(), jumps.len());
                            for (mv, jump) in slice.iter_mut().zip(jumps.into_iter()) {
                                mv.src = jump.src;
                                mv.dst = jump.dst;
                            }
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
                    log::info!("unsolvable");
                    self.forward = Solvability::Unsolvable;
                }
                SolveResult::TimedOut => {}
            }
        }
        if self.backward == Solvability::Unknown {
            log::info!("recomputint backward");
            let solve_result = solve_with_bloom_filter(pos, bloom_filter, Direction::Backward, 0).0;

            match solve_result {
                SolveResult::Solved(jumps) => {
                    log::info!("solved");
                    match self.get_index_in_direction(Direction::Backward) {
                        Some(idx) => {
                            let slice = &mut self.path[..=idx];
                            assert_eq!(slice.len(), jumps.len());
                            for (mv, jump) in slice.iter_mut().zip(jumps.into_iter().rev()) {
                                mv.src = jump.src;
                                mv.dst = jump.dst;
                            }
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
                    log::info!("unsolvable");
                    self.backward = Solvability::Unsolvable;
                }
                SolveResult::TimedOut => {}
            }
        }
    }
}

/// The solve path that passes via the heart shape
const DEFAULT_SOLVE_PATH: [Move; NR_PEGS - 1] = {
    const fn c(x: i8, y: i8) -> Coord {
        Coord::new(x, y).unwrap()
    }
    const fn mv(s: Coord, d: Coord) -> Move {
        Move { src: s, dst: d }
    }

    [
        mv(c(0, -2), c(0, 0)),
        mv(c(-2, -1), c(0, -1)),
        mv(c(-1, -3), c(-1, -1)),
        mv(c(-1, 0), c(-1, -2)),
        mv(c(1, -3), c(-1, -3)),
        mv(c(-1, -3), c(-1, -1)),
        mv(c(-1, 2), c(-1, 0)),
        mv(c(-3, 1), c(-1, 1)),
        mv(c(0, 1), c(-2, 1)),
        mv(c(-3, -1), c(-3, 1)),
        mv(c(-3, 1), c(-1, 1)),
        mv(c(2, 1), c(0, 1)),
        mv(c(1, 3), c(1, 1)),
        mv(c(1, 0), c(1, 2)),
        mv(c(-1, 3), c(1, 3)),
        mv(c(1, 3), c(1, 1)),
        mv(c(1, -2), c(1, 0)),
        mv(c(3, -1), c(1, -1)),
        mv(c(0, -1), c(2, -1)),
        mv(c(3, 1), c(3, -1)),
        mv(c(3, -1), c(1, -1)),
        mv(c(0, 1), c(2, 1)),
        mv(c(2, 1), c(2, -1)),
        mv(c(2, -1), c(0, -1)),
        mv(c(0, -1), c(-2, -1)),
        mv(c(-2, -1), c(-2, 1)),
        mv(c(-2, 1), c(0, 1)),
        mv(c(0, 0), c(-2, 0)),
        mv(c(0, 2), c(0, 0)),
        mv(c(1, 0), c(-1, 0)),
        mv(c(-2, 0), c(0, 0)),
    ]
};

/// Is the current position solvable, i.e. does a path exist
/// from the current positon to the end?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Solvability {
    /// Yes, the position is solvable, and to solve it you should go ahead
    /// with this following move.
    Solvable(Move),
    /// Yes, we have already reached the target position.
    Solved,
    /// No, the position is not solvable.
    Unsolvable,
    /// Maybe. Either we haven't computed the solution path yet, or the
    /// solver encountered an issue.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_forwards_backwards_move_preserves_solution_path() {
        let mut solve_path = SolvePath::new(Position::default_start());

        let mv = Move {
            src: Coord::new(0, -2).unwrap(),
            dst: Coord::center(),
        };
        solve_path.apply_move(mv, Direction::Forward);
        assert!(matches!(
            solve_path.next_move(Direction::Forward),
            Solvability::Solvable(_),
        ));
        assert_eq!(
            solve_path.next_move(Direction::Backward),
            Solvability::Solvable(mv),
        );
        solve_path.apply_move(mv, Direction::Backward);
        assert_eq!(
            solve_path.next_move(Direction::Forward),
            Solvability::Solvable(mv),
        );
        assert_eq!(
            solve_path.next_move(Direction::Backward),
            Solvability::Solved,
        );
    }

    #[test]
    fn test_moving_off_path_invalidates_cached_path() {
        let mut solve_path = SolvePath::new(Position::default_start());

        let other_mv = Move {
            src: Coord::new(2, 0).unwrap(),
            dst: Coord::center(),
        };

        solve_path.apply_move(other_mv, Direction::Forward);

        assert_eq!(
            solve_path.next_move(Direction::Forward),
            Solvability::Unknown
        );

        // Path back to the start should be known because that's where we
        // came from.
        assert_eq!(
            solve_path.next_move(Direction::Backward),
            Solvability::Solvable(other_mv)
        );
    }
}
