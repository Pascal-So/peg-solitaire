use std::rc::Rc;

use common::{
    BloomFilter, Direction, Jump, NR_PEGS, Position, coord::Coord, debruijn::de_bruijn_solvable,
    solve_with_bloom_filter,
};
use yew::Reducible;

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
    selection: Option<usize>,
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
        let idx = self.selection?;
        let peg = self.arrangement.pegs[idx];
        if !peg.alive {
            log::warn!("Selected peg {idx} at {} is not alive!", peg.coord);
        }
        Some(peg.coord)
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
        self.pegs().filter(|peg| peg.alive).count() as i32
    }
    pub fn pegs(&self) -> impl Iterator<Item = &Peg> {
        self.arrangement.pegs.iter()
    }

    fn apply_move(&mut self, move_info: MoveInfo, dir: Direction) {
        self.arrangement = self.arrangement.apply_move(move_info, dir);
        self.selection = None;
        self.has_made_first_move = true;
        // self.solve_paths.update(move_info, dir);

        self.history.push(HistoryEntry::Move(move_info));
        if self.redo.pop() != Some(HistoryEntry::Move(move_info)) {
            self.redo.clear();
        }
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
                        if let Some(idx) = self.arrangement.lookup(coord) {
                            let mut state = (*self).clone();
                            state.selection = Some(idx);
                            return state.into();
                        }
                        return self;
                    }
                    Some(selected_coord) => {
                        // A peg is currently selected

                        if selected_coord == coord {
                            // Same peg is clicked again, deselecting the peg.
                            let mut state = (*self).clone();
                            state.selection = None;
                            return state.into();
                        }

                        if let Some(clicked_peg) = self.arrangement.lookup(coord) {
                            // Clicked a different peg, select that one instead.
                            let mut state = (*self).clone();
                            state.selection = Some(clicked_peg);
                            return state.into();
                        }

                        let move_info = self.arrangement.check_move(selected_coord, coord);
                        if let Some(move_info) = move_info {
                            // Peform a move
                            let mut state = (*self).clone();
                            state.apply_move(move_info, Direction::Forward);
                            return state.into();
                        }

                        self
                    }
                }
            }
            (GameAction::ClickHole { coord }, Mode::Edit) => {
                let mut state = (*self).clone();

                let old_arrangement = self.arrangement;
                let mut changed = false;

                if let Some(idx) = state.arrangement.lookup(coord) {
                    state.arrangement.pegs[idx].alive = false;
                    changed = true;
                } else {
                    for p in state.arrangement.pegs.iter_mut() {
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
                        state.history.push(HistoryEntry::Edit(old_arrangement));
                    }
                    state.redo.clear();
                    state.into()
                } else {
                    // nothing changed, return the old state
                    self
                }
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
                    HistoryEntry::Move(last_move) => {
                        state.redo.push(HistoryEntry::Move(last_move));
                        state.arrangement =
                            state.arrangement.apply_move(last_move, Direction::Backward);
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
                    HistoryEntry::Move(move_info) => {
                        state.history.push(HistoryEntry::Move(move_info));
                        state.arrangement =
                            state.arrangement.apply_move(move_info, Direction::Forward);
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

#[derive(Clone, PartialEq, Eq, Debug)]
enum HistoryEntry {
    Edit(Arrangement),
    Move(MoveInfo),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    pub fn lookup(&self, coord: Coord) -> Option<usize> {
        for (i, p) in self.pegs.iter().enumerate() {
            if p.coord == coord && p.alive {
                return Some(i);
            }
        }

        None
    }

    pub fn check_move_backwards(&self, src: Coord, dst: Coord) -> Option<MoveInfo> {
        // let mut moved_idx = None;
        // let mut eliminated_idx = None;

        let mid = get_move_middle(src, dst)?;

        todo!()
    }

    /// Check if we can perform a move from `src` to `dst`.
    pub fn check_move(&self, src: Coord, dst: Coord) -> Option<MoveInfo> {
        let mut moved_idx = None;
        let mut eliminated_idx = None;

        let mid = get_move_middle(src, dst)?;

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

    fn apply_move(mut self, mut move_info: MoveInfo, dir: Direction) -> Self {
        if dir == Direction::Backward {
            std::mem::swap(&mut move_info.src, &mut move_info.dst);
        }
        // todo: this failed during an undo of a backwards solve seek?
        assert_eq!(self.pegs[move_info.moved_idx].coord, move_info.src);
        assert_eq!(
            self.pegs[move_info.eliminated_idx].alive,
            dir == Direction::Forward
        );
        self.pegs[move_info.eliminated_idx].alive = dir == Direction::Backward;
        self.pegs[move_info.moved_idx].coord = move_info.dst;
        self
    }
}

impl Default for Arrangement {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq)]
pub struct GameState2 {
    arrangement: Arrangement,

    path_from_start: Option<Vec<MoveInfo>>,
    path_to_end: Option<Vec<MoveInfo>>,
}

/// Acts as a token, proving that the move is possible. This token is
/// not completely fool-proof, since it's possible that the game state
/// has been changed in between, but as long as tokens are immediately
/// used, this is fine.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct MoveInfo {
    moved_idx: usize,
    eliminated_idx: usize,
    src: Coord,
    dst: Coord,
}

impl GameState2 {
    pub fn pegs(&self) -> impl Iterator<Item = &Peg> {
        self.arrangement.pegs.iter()
    }
    pub fn is_solvable(&self) -> (Solvability, Solvability) {
        let from_option = |opt: &Option<_>| {
            if opt.is_some() {
                Solvability::Yes
            } else {
                Solvability::No
            }
        };
        (
            from_option(&self.path_from_start),
            from_option(&self.path_to_end),
        )
    }

    pub fn rerun_solver(mut self, bloom_filter: &BloomFilter) -> Self {
        let pos = self.as_position();
        if !de_bruijn_solvable(pos) {
            self.path_from_start = None;
            self.path_to_end = None;
        } else {
            for (path, dir) in [
                (&mut self.path_to_end, Direction::Forward),
                (&mut self.path_from_start, Direction::Backward),
            ] {
                if path.is_none() {
                    if dir == Direction::Backward {
                        // backwards is currently not implemented yet
                        continue;
                    }

                    log::info!("running solver for direction {dir:?}");
                    match solve_with_bloom_filter(pos, bloom_filter, dir, 0).0 {
                        common::SolveResult::Solved(jumps) => {
                            let mut moves = convert_jump_sequence_to_moves(
                                self.arrangement.clone(),
                                &jumps,
                                dir,
                            );
                            moves.reverse();
                            *path = Some(moves);
                        }
                        _ => {}
                    }
                }
            }
        }
        self
    }

    /// Should be called after a move has been successfully applied to the arrangement.
    /// Here we try to keep the solve paths from the start and to the end updated without
    /// re-running the solver, but only by updating the existing cached values
    fn update_solve_paths(&mut self, move_info: MoveInfo, dir: Direction) {
        let (forwards, backwards) = match dir {
            Direction::Forward => (&mut self.path_to_end, &mut self.path_from_start),
            Direction::Backward => (&mut self.path_from_start, &mut self.path_to_end),
        };

        if let Some(backwards) = backwards {
            backwards.push(move_info);
        }

        if let Some(forwards) = forwards
            && forwards.last() == Some(&move_info)
        {
            forwards.pop();
        } else {
            *forwards = None;
        }
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
/// Preconditions: jump sequence must be applicable to the given arrangement
fn convert_jump_sequence_to_moves(
    mut arrangement: Arrangement,
    jumps: &[Jump],
    dir: Direction,
) -> Vec<MoveInfo> {
    let mut moves = vec![];

    for &jump in jumps {
        let m = match dir {
            Direction::Forward => arrangement.check_move(jump.src, jump.dst),
            Direction::Backward => arrangement.check_move_backwards(jump.src, jump.dst),
        };
        let m = m.expect("jump sequence should be applicable to initial game state");
        moves.push(m);

        arrangement = arrangement.apply_move(m, Direction::Forward);
    }

    moves
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Peg {
    pub coord: Coord,
    /// Is the peg still on the board?
    pub alive: bool,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Solvability {
    Yes,
    No,
    Maybe,
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
