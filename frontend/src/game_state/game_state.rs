use std::rc::Rc;

use common::{BloomFilter, Direction, Move, NR_HOLES, Position, coord::Coord};
use yew::Reducible;

use crate::game_state::{
    Solvability,
    arrangement::{Arrangement, Peg},
    solver::SolvePath,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Play,
    Edit,
}

#[derive(Debug, Clone)]
pub enum GameAction {
    ClickHole { coord: Coord },
    SetMode { mode: Mode },
    Reset,
    Undo,
    Redo,
    RegisterSolver { solver: Rc<BloomFilter> },
    StepSolution { dir: Direction },
}

/// Game State as seen from the user interface. The interaction with this state
/// happens through [GameAction]s that are sent to Yew's
/// [`use_reducer`](https://docs.rs/yew/0.21.0/yew/functional/fn.use_reducer.html)
#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    history: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    solve_path: SolvePath,
    arrangement: Arrangement,
    selection: Option<Coord>,
    pub mode: Mode,
    has_made_first_move: bool,
    bloom_filter: Option<Rc<BloomFilter>>,
}

impl GameState {
    pub fn new() -> GameState {
        let arrangement = Arrangement::new();

        Self {
            history: vec![],
            redo: vec![],
            solve_path: SolvePath::new(arrangement.as_position()),
            arrangement,
            selection: None,
            mode: Mode::Play,
            has_made_first_move: false,
            bloom_filter: None,
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
    pub fn pegs(&self) -> [Peg; NR_HOLES] {
        self.arrangement.pegs()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn has_made_first_move(&self) -> bool {
        self.has_made_first_move
    }

    pub fn is_solvable(&self) -> (Solvability, Solvability) {
        self.solve_path.is_solvable()
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

                        let Some(mv) = Move::from_coords(selected_coord, coord) else {
                            // The selected coordinates are not two holes apart.
                            return self;
                        };

                        let mut state = (*self).clone();
                        match state.arrangement.perform_move(mv, Direction::Forward) {
                            Ok(_) => {
                                // successfully made a move
                                state.has_made_first_move = true;
                                state
                                    .history
                                    .push(HistoryEntry::Move(mv, Direction::Forward));
                                state.solve_path.apply_move(mv, Direction::Forward);
                                if let Some(bf) = &self.bloom_filter {
                                    state.solve_path.recompute(bf, state.as_position());
                                }
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
                // Clicking a hole (or peg) in edit mode means toggling peg
                // presence in that location.

                let mut state = (*self).clone();

                let old_arrangement = self.arrangement;

                state.arrangement.toggle_hole(coord);
                state.solve_path = SolvePath::new(state.as_position());
                if let Some(bf) = &self.bloom_filter {
                    state.solve_path.recompute(bf, state.as_position());
                }

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
                        state.solve_path = SolvePath::new(state.as_position());
                        if let Some(bf) = &self.bloom_filter {
                            state.solve_path.recompute(bf, state.as_position());
                        }
                    }
                    HistoryEntry::Move(mv, dir) => {
                        state.redo.push(HistoryEntry::Move(mv, dir));
                        state.arrangement.perform_move(mv, !dir).unwrap();
                        state.solve_path.apply_move(mv, !dir);
                        if let Some(bf) = &self.bloom_filter {
                            state.solve_path.recompute(bf, state.as_position());
                        }
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
                        state.solve_path = SolvePath::new(state.as_position());
                        if let Some(bf) = &self.bloom_filter {
                            state.solve_path.recompute(bf, state.as_position());
                        }
                    }
                    HistoryEntry::Move(mv, dir) => {
                        state.history.push(HistoryEntry::Move(mv, dir));
                        state.arrangement.perform_move(mv, dir).unwrap();
                        state.solve_path.apply_move(mv, dir);
                        if let Some(bf) = &self.bloom_filter {
                            state.solve_path.recompute(bf, state.as_position());
                        }
                    }
                }

                state.into()
            }
            (GameAction::Reset, _) => {
                let mut state = GameState::new();
                state.has_made_first_move = self.has_made_first_move;
                state.bloom_filter = self.bloom_filter.clone();
                state.into()
            }
            (GameAction::RegisterSolver { solver }, _) => {
                // todo: maybe add a way to disable the solver while we're not
                // showing the solver toolbar?
                let mut state = (*self).clone();
                state.solve_path.recompute(&solver, state.as_position());
                state.bloom_filter = Some(solver);
                state.into()
            }
            (GameAction::StepSolution { dir }, _) => {
                if let Some(mv) = self.solve_path.next_move(dir) {
                    let mut state = (*self).clone();
                    state.history.push(HistoryEntry::Move(mv, dir));
                    state.arrangement.perform_move(mv, dir).unwrap();
                    state.solve_path.apply_move(mv, dir);
                    if let Some(bf) = &self.bloom_filter {
                        state.solve_path.recompute(bf, state.as_position());
                    }

                    state.into()
                } else {
                    self
                }
            }
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

#[derive(Clone, Debug, PartialEq)]
enum HistoryEntry {
    Edit(Arrangement),
    Move(Move, Direction),
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
    fn test_invalid_undo_is_ignored() {
        let gs = game_state();
        gs.reduce(GameAction::Undo);
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
        assert!(!gs.has_made_first_move());

        let gs = gs.reduce(click_action(0, 0));
        assert!(gs.has_made_first_move());

        // resetting does not reset first move flag
        let gs = gs.reduce(GameAction::Reset);
        assert!(gs.has_made_first_move());
    }

    #[test]
    fn test_nr_pegs() {
        assert_eq!(game_state().nr_pegs(), 32);
        assert_eq!(game_state_after_one_move().nr_pegs(), 31);
    }

    #[test]
    fn undo_and_redo_keeps_solve_path_intact() {
        let gs = game_state();

        // todo
    }
}
