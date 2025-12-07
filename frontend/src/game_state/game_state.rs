use std::rc::Rc;

use common::{
    BloomFilter, Direction, NR_PEGS, Position, coord::Coord, debruijn::de_bruijn_solvable,
    solve_with_bloom_filter,
};
use yew::Reducible;

use crate::game_state::arrangement::{Arrangement, Peg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Play,
    Edit,
}

#[derive(Debug, Clone, Copy)]
pub enum GameAction {
    ClickHole { coord: Coord },
    SetMode { mode: Mode },
    Reset,
    Undo,
    Redo,
    RegisterSolver { solver: () },
    StepSolution { dir: Direction },
}

/// Game State as seen from the user interface. The interaction with this state
/// happens through [GameAction]s that are sent to Yew's
/// [`use_reducer`](https://docs.rs/yew/0.21.0/yew/functional/fn.use_reducer.html)
#[derive(Debug, Clone)]
pub struct GameState {
    history: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    solve_path: SolvePath,
    arrangement: Arrangement,
    selection: Option<Coord>,
    pub mode: Mode,
    has_made_first_move: bool,
}

impl GameState {
    pub fn new() -> GameState {
        Self {
            history: vec![],
            redo: vec![],
            solve_path: SolvePath::new(),
            arrangement: Arrangement::new(),
            selection: None,
            mode: Mode::Play,
            has_made_first_move: false,
        }
    }
    pub fn selected_coord(&self) -> Option<Coord> {
        let coord = self.selection?;
        if !self.arrangement.is_occupied(coord) {
            log::warn!("Selected coordinate {coord} is not occupied!");
        }
        Some(coord)
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
    pub fn nr_pegs(&self) -> i32 {
        self.arrangement.nr_pegs() as i32
    }
    pub fn pegs(&self) -> impl IntoIterator<Item = Peg> {
        self.arrangement.pegs()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

impl Reducible for GameState {
    type Action = GameAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        log::debug!("Reducing with action {action:?}");

        match (action, self.mode) {
            (GameAction::ClickHole { coord }, Mode::Play) => {
                match self.selected_coord() {
                    None => {
                        // Nothing currently selected
                        if self.arrangement.is_occupied(coord) {
                            let mut state = (*self).clone();
                            state.selection = Some(coord);
                            return state.into();
                        } else {
                            return self;
                        }
                    }
                    Some(selected_coord) => {
                        // A peg is currently selected

                        if selected_coord == coord {
                            // Same peg is clicked again, deselecting the peg.
                            let mut state = (*self).clone();
                            state.selection = None;
                            return state.into();
                        }

                        if self.arrangement.is_occupied(coord) {
                            // Clicked a different peg, select that one instead.
                            let mut state = (*self).clone();
                            state.selection = Some(coord);
                            return state.into();
                        }

                        // Clicked an empty hole, try to peform a move
                        let mut state = (*self).clone();
                        match state.arrangement.perform_move(
                            selected_coord,
                            coord,
                            Direction::Forward,
                        ) {
                            Ok(_) => {
                                // successfully made a move
                                state.has_made_first_move = true;
                                state.history.push(HistoryEntry::Move {
                                    src: selected_coord,
                                    dst: coord,
                                });
                                state.redo.clear();
                                state.selection = None;
                            }
                            Err(_) => {
                                // User attempted to perform invalid
                                // move, ignoring..
                            }
                        }
                        state.into()
                    }
                }
            }
            (GameAction::ClickHole { coord }, Mode::Edit) => {
                let mut state = (*self).clone();

                let old_arrangement = self.arrangement;

                state.arrangement.toggle_hole(coord);

                // If the last history entry already contains an edit, then we
                // don't append another entry. This has the effect of combining
                // all the edits into one.
                // TODO: add an edit session id or something like that, so that
                // we can have one undo step per edit session.
                if !matches!(self.history.last(), Some(&HistoryEntry::Edit(_))) {
                    state.history.push(HistoryEntry::Edit(old_arrangement));
                }
                state.redo.clear();
                state.into()
            }
            (GameAction::Undo, _) => {
                if self.history.is_empty() {
                    // nothing to undo
                    return self;
                }

                let mut state = (*self).clone();
                let entry = state.history.pop().unwrap();
                state.selection = None;

                match entry {
                    HistoryEntry::Edit(mut arrangement) => {
                        std::mem::swap(&mut state.arrangement, &mut arrangement);
                        state.redo.push(HistoryEntry::Edit(arrangement));
                        // state.solve_path.clear();
                    }
                    HistoryEntry::Move { src, dst } => {
                        state.redo.push(HistoryEntry::Move { src, dst });
                        state
                            .arrangement
                            .perform_move(src, dst, Direction::Backward)
                            .unwrap();
                        // state.solve_path.update(last_move, Direction::Backward);
                    }
                }

                state.into()
            }
            (GameAction::Redo, _) => {
                if self.redo.is_empty() {
                    // nothing to redo
                    return self;
                }

                let mut state = (*self).clone();
                let entry = state.redo.pop().unwrap();

                match entry {
                    HistoryEntry::Edit(mut arrangement) => {
                        std::mem::swap(&mut state.arrangement, &mut arrangement);
                        state.history.push(HistoryEntry::Edit(arrangement));
                        // state.solve_path.clear();
                    }
                    HistoryEntry::Move { src, dst } => {
                        state.history.push(HistoryEntry::Move { src, dst });
                        state
                            .arrangement
                            .perform_move(src, dst, Direction::Forward)
                            .unwrap();
                        // state.solve_path.update(move_info, Direction::Forward);
                    }
                }

                state.into()
            }
            (GameAction::Reset, _) => {
                let mut state = GameState::new();
                state.has_made_first_move = self.has_made_first_move;
                state.into()
            }
            (GameAction::RegisterSolver { solver }, _) => todo!(),
            (GameAction::StepSolution { dir }, _) => todo!(),
            (GameAction::SetMode { mode }, _) => {
                if mode == self.mode {
                    return self;
                }

                let mut state = (*self).clone();
                state.mode = mode;
                state.selection = None;
                state.into()
            }
        }
    }
}

#[derive(Debug, Clone)]
struct SolvePath {
    path: [(); NR_PEGS - 2], // todo
}

impl SolvePath {
    pub fn new() -> Self {
        Self {
            path: [(); NR_PEGS - 2],
        }
    }

    // pub fn recompute(&mut self, solver: &BloomFilter, current_arrangement: Arrangement)

    pub fn clear(&mut self) {
        todo!()
    }
}

#[derive(Clone, Debug)]
enum HistoryEntry {
    Edit(Arrangement),
    Move { src: Coord, dst: Coord },
}

// pub fn rerun_solver(mut self, bloom_filter: &BloomFilter) -> Self {
//     let pos = self.as_position();
//     if !de_bruijn_solvable(pos) {
//         self.path_from_start = None;
//         self.path_to_end = None;
//     } else {
//         for (path, dir) in [
//             (&mut self.path_to_end, Direction::Forward),
//             (&mut self.path_from_start, Direction::Backward),
//         ] {
//             if path.is_none() {
//                 if dir == Direction::Backward {
//                     // backwards is currently not implemented yet
//                     continue;
//                 }

//                 log::info!("running solver for direction {dir:?}");
//                 match solve_with_bloom_filter(pos, bloom_filter, dir, 0).0 {
//                     common::SolveResult::Solved(jumps) => {
//                         let mut moves =
//                             convert_jump_sequence_to_moves(self.arrangement.clone(), &jumps, dir);
//                         moves.reverse();
//                         *path = Some(moves);
//                     }
//                     _ => {}
//                 }
//             }
//         }
//     }
//     self
// }

// /// Should be called after a move has been successfully applied to the arrangement.
// /// Here we try to keep the solve paths from the start and to the end updated without
// /// re-running the solver, but only by updating the existing cached values
// fn update_solve_paths(&mut self, move_info: MoveInfo, dir: Direction) {
//     let (forwards, backwards) = match dir {
//         Direction::Forward => (&mut self.path_to_end, &mut self.path_from_start),
//         Direction::Backward => (&mut self.path_from_start, &mut self.path_to_end),
//     };

//     if let Some(backwards) = backwards {
//         backwards.push(move_info);
//     }

//     if let Some(forwards) = forwards
//         && forwards.last() == Some(&move_info)
//     {
//         forwards.pop();
//     } else {
//         *forwards = None;
//     }
// }

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Solvability {
    Yes,
    No,
    Maybe,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_state() -> Rc<GameState> {
        Rc::new(GameState::new())
    }
    fn click_action(x: i8, y: i8) -> GameAction {
        GameAction::ClickHole {
            coord: Coord::new(x, y).unwrap(),
        }
    }
    fn game_state_after_one_move() -> Rc<GameState> {
        let gs = game_state();
        let gs = gs.reduce(click_action(2, 0));
        let gs = gs.reduce(click_action(0, 0));
        gs
    }

    #[test]
    fn test_select_deselect() {
        let gs = game_state();

        let gs = gs.reduce(click_action(2, 0));
        assert_eq!(gs.selected_coord(), Some(Coord::new(2, 0).unwrap()));

        let gs = gs.reduce(click_action(2, 0));
        assert_eq!(
            gs.selection, None,
            "clicking the same position again should deselect"
        );
    }

    #[test]
    fn test_cannot_select_empty_hole() {
        let gs = game_state();

        let gs = gs.reduce(click_action(0, 0));
        assert_eq!(gs.selected_coord(), None);
    }

    #[test]
    fn test_move() {
        let gs = game_state_after_one_move();

        let expected = Position::from_ascii([
            "    ###    ",
            "    ###    ",
            "  #######  ",
            "  ####..#  ",
            "  #######  ",
            "    ###    ",
            "    ###    ",
        ]);
        assert_eq!(gs.as_position(), expected);
    }

    #[test]
    fn test_undo_move() {
        let gs = game_state_after_one_move();
        assert!(gs.can_undo());

        let gs = gs.reduce(GameAction::Undo);

        assert_eq!(gs.as_position(), Position::default_start());
        assert!(!gs.can_undo());
    }

    #[test]
    fn test_redo_move() {
        let gs = game_state_after_one_move();
        let position = gs.as_position();

        assert!(!gs.can_redo());
        let gs = gs.reduce(GameAction::Undo);

        assert!(gs.can_redo());
        let gs = gs.reduce(GameAction::Redo);
        assert_eq!(position, gs.as_position());

        assert!(!gs.can_redo());
    }

    #[test]
    fn test_edit_mode() {
        let gs = game_state();
        let gs = gs.reduce(GameAction::SetMode { mode: Mode::Edit });

        assert_eq!(gs.as_position(), Position::default_start());

        let gs = gs.reduce(click_action(1, 2));

        let expected = Position::from_ascii([
            "    ###    ",
            "    ###    ",
            "  #######  ",
            "  ###.###  ",
            "  #######  ",
            "    ##.    ",
            "    ###    ",
        ]);
        assert_eq!(gs.as_position(), expected);

        let gs = gs.reduce(click_action(0, 2)).reduce(click_action(1, 2));

        let expected = Position::from_ascii([
            "    ###    ",
            "    ###    ",
            "  #######  ",
            "  ###.###  ",
            "  #######  ",
            "    #.#    ",
            "    ###    ",
        ]);
        assert_eq!(gs.as_position(), expected);
    }

    #[test]
    fn test_reset() {
        let gs = game_state_after_one_move().reduce(click_action(-1, -1));

        assert!(gs.selected_coord().is_some());

        let gs = gs.reduce(GameAction::Reset);

        assert!(gs.selected_coord().is_none());
        assert_eq!(gs.as_position(), Position::default_start());
    }
    #[test]
    fn test_undo_resets_selection() {
        let gs = game_state_after_one_move().reduce(click_action(-1, -1));

        assert!(gs.selected_coord().is_some());

        let gs = gs.reduce(GameAction::Undo);
        assert!(gs.selected_coord().is_none());
    }

    #[test]
    fn test_multiple_edits_count_as_one_undo_step() {
        let gs = game_state();
        let gs = gs
            .reduce(GameAction::SetMode { mode: Mode::Edit })
            .reduce(click_action(1, 2))
            .reduce(click_action(2, 0));

        let gs = gs.reduce(GameAction::Undo);
        assert_eq!(gs.as_position(), Position::default_start());
        assert!(!gs.can_undo());
    }

    #[test]
    fn test_has_made_first_move() {
        let gs = game_state();

        // selecting a peg does not count as a move
        let gs = gs.reduce(click_action(2, 0));
        assert!(!gs.has_made_first_move);

        let gs = gs.reduce(click_action(0, 0));
        assert!(gs.has_made_first_move);

        // resetting does not reset first move flag
        let gs = gs.reduce(GameAction::Reset);
        assert!(gs.has_made_first_move);
    }

    #[test]
    fn test_nr_pegs() {
        assert_eq!(game_state().nr_pegs(), 32);
        assert_eq!(game_state_after_one_move().nr_pegs(), 31);
    }

    #[test]
    fn undo_and_redo_keeps_solve_path_intact() {
        // todo
    }
}
