use common::{BloomFilter, NR_HOLES, coord::Coord};
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use web_sys::HtmlElement;
use yew::prelude::*;
use yew_hooks::prelude::*;
use yew_icons::{Icon, IconId};

use crate::game_state::Peg;

const PX_HOLE_DISTANCE: i16 = 34;

#[derive(Properties, PartialEq)]
pub struct BoardProps {
    pub has_made_first_move: bool,
    pub edit_mode: bool,
    pub selected: Option<Coord>,
    pub reset: Callback<()>,
    pub undo: Option<Callback<()>>,
    pub redo: Option<Callback<()>>,
    pub holeclick: Callback<Coord>,
    pub toggle_solver: Callback<()>,
    pub toggle_edit_mode: Callback<()>,
    pub pegs: [Peg; NR_HOLES],
}

/// Render the game board with pegs and holes, plus some surrounding buttons.
#[function_component]
fn Board(
    BoardProps {
        has_made_first_move,
        edit_mode,
        selected,
        reset,
        undo,
        redo,
        holeclick,
        toggle_solver,
        toggle_edit_mode,
        pegs,
    }: &BoardProps,
) -> Html {
    let cell_classes = {
        move |coord: Coord| {
            let mut classes = Classes::new();
            classes.push("game-cell");
            if *selected == Some(coord) && !edit_mode {
                classes.push("selected");
            }
            classes
        }
    };

    let mut overall_classes = Classes::new();
    overall_classes.push("game-grid");
    if *edit_mode {
        overall_classes.push("edit-mode");
    }

    let holeclick = holeclick.clone();
    let reset = {
        let reset = reset.clone();
        move |_| reset.emit(())
    };
    let can_undo = undo.is_some();
    let undo = {
        let undo = undo.clone();
        move |_| {
            undo.as_ref().map(|cb| cb.emit(()));
        }
    };
    let can_redo = redo.is_some();
    let redo = {
        let redo = redo.clone();
        move |_| {
            redo.as_ref().map(|cb| cb.emit(()));
        }
    };
    let toggle_edit_mode = {
        let toggle_edit_mode = toggle_edit_mode.clone();
        move |_| toggle_edit_mode.emit(())
    };
    let toggle_solver = {
        let toggle_solver = toggle_solver.clone();
        move |_| toggle_solver.emit(())
    };

    html! {
        <div class={overall_classes}>
            <button
                style={format!("grid-row: 1; grid-column: 1/3; opacity: {};", b2f(can_undo || *edit_mode))}
                onclick={reset}
            >
                {"reset"}
            </button>
            <button
                style={format!("grid-row: 2; grid-column: 1; opacity: {};", b2f(can_undo && !*edit_mode))}
                onclick={undo}
            >
                <Icon icon_id={IconId::LucideUndo2} class="icon"/>
            </button>
            <button
                style={format!("grid-row: 2; grid-column: 2; opacity: {};", b2f(can_redo && !*edit_mode))}
                onclick={redo}
            >
                <Icon icon_id={IconId::LucideRedo2} class="icon"/>
            </button>

            <button
                style="grid-row: 1; grid-column: 6/8;"
                onclick={toggle_edit_mode}
            >
                {if *edit_mode {"done"} else {"edit"}}
            </button>

            <button
                style={format!("grid-row: 7; grid-column: 6/8; opacity: {};", b2f(*has_made_first_move))}
                onclick={toggle_solver}
            >
                {"solver"}
            </button>

            { for Coord::all().into_iter().map(|coord| {let holeclick = holeclick.clone(); html! {
                <div
                    class={cell_classes(coord)}
                    onmousedown={move |_| holeclick.emit(coord)}
                    style={format!("grid-row: {}; grid-column: {};", coord.y() + 4, coord.x() + 4)}
                />
            }}) }

            { for pegs.into_iter().enumerate().map(|(i, p)| {
                let left = PX_HOLE_DISTANCE * (p.coord.x() as i16 + 3);
                let top = PX_HOLE_DISTANCE * (p.coord.y() as i16 + 3);
                html!{
                    <div
                        class="peg"
                        key={i}
                        style={format!("left: {left}px; top: {top}px; opacity: {};", b2f(p.alive))}
                    />
                }
            }) }
        </div>
    }
}

/// Convert a bool to a float, which is useful for CSS opacity
fn b2f(b: bool) -> f32 {
    if b { 1.0 } else { 0.0 }
}
