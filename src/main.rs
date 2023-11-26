mod game_state;

use yew::prelude::*;
use yew_hooks::prelude::*;
use yew_icons::{Icon, IconId};

use crate::game_state::{Coord, GameState, LookupResult, HOLE_COORDS};

#[function_component]
fn App() -> Html {
    let scale = |x: i16| x * 34;
    let b2f = |b: bool| if b { 1.0 } else { 0.0 };

    let game_state = use_state(|| GameState::new());
    let selected = use_state(|| None);
    let display_scale = use_state(|| 1.0);
    let edit_mode = use_state(|| false);

    let reset = {
        let game_state = game_state.clone();
        Callback::from(move |_| {
            log::info!("reset");
            game_state.set(GameState::new());
        })
    };

    let undo = {
        let game_state = game_state.clone();
        Callback::from(move |_| {
            if game_state.can_undo() {
                log::info!("undo");
                game_state.set(GameState::clone(&game_state).undo());
            }
        })
    };

    let redo = {
        let game_state = game_state.clone();
        Callback::from(move |_| {
            if game_state.can_redo() {
                log::info!("redo");
                game_state.set(GameState::clone(&game_state).redo());
            }
        })
    };

    let move_peg = Callback::from({
        let game_state = game_state.clone();
        move |(src, dst): (Coord, Coord)| {
            let Some(move_info) = game_state.check_move(src, dst) else {
                return;
            };
            log::debug!("moving from {src:?} to {dst:?}");
            game_state.set(GameState::clone(&game_state).apply_move(move_info));
        }
    });

    let holeclick = {
        let selected = selected.clone();
        let game_state = game_state.clone();
        let move_peg = move_peg.clone();
        let edit_mode = edit_mode.clone();

        move |&coord: &Coord| {
            let selected = selected.clone();
            let move_peg = move_peg.clone();
            let game_state = game_state.clone();
            let edit_mode = edit_mode.clone();

            Callback::from(move |_: MouseEvent| {
                log::debug!("click {coord:?}");

                if *edit_mode {
                    game_state.set(GameState::clone(&game_state).edit_toggle_peg(coord));
                    return;
                }

                if *selected == Some(coord) {
                    selected.set(None);
                    return;
                }
                log::debug!("selected {selected:?}");

                match game_state.lookup(coord) {
                    LookupResult::Invalid => {}
                    LookupResult::Peg(_) => selected.set(Some(coord)),
                    LookupResult::Empty => {
                        if let Some(src) = *selected {
                            move_peg.emit((src, coord));
                            selected.set(None);
                        }
                    }
                }
            })
        }
    };

    let cell_classes = {
        let edit_mode = edit_mode.clone();
        let selected = selected.clone();

        move |(x, y): Coord| {
            let mut classes = Classes::new();
            classes.push("game-cell");
            if *selected == Some((x, y)) && !*edit_mode {
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

    let window_size = use_window_size();
    let debounced_size_update = {
        let window_size = window_size.clone();
        let display_scale = display_scale.clone();
        use_debounce(
            move || {
                let new_scale = window_size.0.min(window_size.1) / 234.0 * 0.9;
                display_scale.set(new_scale);
            },
            200,
        )
    };
    use_memo(window_size, |_| {
        debounced_size_update.run();
        || {}
    });

    let edit = {
        let edit_mode = edit_mode.clone();
        let selected = selected.clone();
        Callback::from(move |_| {
            edit_mode.set(!*edit_mode);
            selected.set(None);
        })
    };

    html! {
        <div class={overall_classes} style={format!("transform: scale({})", *display_scale)}>
            <button
                style={format!("grid-row: 1; grid-column: 1/3; opacity: {};", b2f(game_state.can_undo() || *edit_mode))}
                onclick={reset}
            >
                {"reset"}
            </button>
            <button
                style={format!("grid-row: 2; grid-column: 1; opacity: {};", b2f(game_state.can_undo() && !*edit_mode))}
                onclick={undo}
            >
                <Icon icon_id={IconId::LucideUndo2} class="icon"/>
            </button>
            <button
                style={format!("grid-row: 2; grid-column: 2; opacity: {};", b2f(game_state.can_redo() && !*edit_mode))}
                onclick={redo}
            >
                <Icon icon_id={IconId::LucideRedo2} class="icon"/>
            </button>

            <button
                style="grid-row: 1; grid-column: 6/8;"
                onclick={edit}
            >
                if *edit_mode {<>{"done"}</>} else {<>{"edit"}</>}
            </button>

            { for HOLE_COORDS.iter().map(|&(x, y)| html! {
                <div
                    class={cell_classes((x, y))}
                    onmousedown={holeclick(&(x, y))}
                    style={format!("grid-row: {}; grid-column: {};", y + 1, x + 1)}
                />
            }) }

            { for game_state.pegs.iter().enumerate().map(|(i, p)| {
                let left = scale(p.coord.0);
                let top = scale(p.coord.1);
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

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
