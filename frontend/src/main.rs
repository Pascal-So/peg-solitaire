mod game_state;

use common::{BloomFilter, coord::Coord};
use gloo_net::http::Request;
use web_sys::HtmlElement;
use yew::prelude::*;
use yew_hooks::prelude::*;
use yew_icons::{Icon, IconId};

use crate::game_state::{GameState, Solvability};

const PX_HOLE_DISTANCE: i16 = 34;

#[derive(PartialEq)]
enum BloomFilterResource {
    Loaded(BloomFilter),
    Loading,
    NotRequested,
}

#[function_component]
fn App() -> Html {
    let b2f = |b: bool| if b { 1.0 } else { 0.0 };

    let game_state = use_state(|| GameState::new());
    let selected = use_state(|| None);
    let display_scale = use_state(|| 1.0);
    let edit_mode = use_state(|| false);
    let bloom_filter = use_state(|| BloomFilterResource::NotRequested);
    let div_ref = use_node_ref();
    let solver_visible = use_state(|| false);
    let has_made_first_move = use_state(|| false);

    use_effect_with(
        (
            game_state.clone(),
            solver_visible.clone(),
            bloom_filter.clone(),
        ),
        |(game_state, solver_visible, bloom_filter)| {
            if **solver_visible && let BloomFilterResource::Loaded(bloom_filter) = &**bloom_filter {
                game_state.set(GameState::clone(&**game_state).rerun_solver(bloom_filter));
            }
        },
    );

    let reset = {
        let game_state = game_state.clone();
        let selected = selected.clone();
        Callback::from(move |_| {
            log::info!("reset");
            selected.set(None);
            game_state.set(GameState::new());
        })
    };

    let undo = {
        let game_state = game_state.clone();
        let selected = selected.clone();
        Callback::from(move |_| {
            if game_state.can_undo() {
                log::info!("undo");
                selected.set(None);
                game_state.set(GameState::clone(&game_state).undo());
            }
        })
    };

    let redo = {
        let game_state = game_state.clone();
        let selected = selected.clone();
        Callback::from(move |_| {
            if game_state.can_redo() {
                log::info!("redo");
                selected.set(None);
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
        let has_made_first_move = has_made_first_move.clone();

        move |coord: Coord| {
            let selected = selected.clone();
            let move_peg = move_peg.clone();
            let game_state = game_state.clone();
            let edit_mode = edit_mode.clone();
            let has_made_first_move = has_made_first_move.clone();

            Callback::from(move |_: MouseEvent| {
                if *edit_mode {
                    game_state.set(GameState::clone(&game_state).edit_toggle_peg(coord));
                    has_made_first_move.set(true);
                    return;
                }

                if *selected == Some(coord) {
                    selected.set(None);
                    return;
                }

                match game_state.lookup(coord) {
                    Some(_) => selected.set(Some(coord)),
                    None => {
                        if let Some(src) = *selected {
                            move_peg.emit((src, coord));
                            has_made_first_move.set(true);
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

        move |coord: Coord| {
            let mut classes = Classes::new();
            classes.push("game-cell");
            if *selected == Some(coord) && !*edit_mode {
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
        let display_scale = display_scale.clone();
        let div_ref = div_ref.clone();
        use_debounce(
            move || {
                let Some(div) = div_ref.cast::<HtmlElement>() else {
                    return;
                };

                let div_width = div.client_width() as f64;
                let div_height = div_width * 1.3;

                let new_scale = (window_size.0 / div_width).min(window_size.1 / div_height) * 0.9;
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

    let toggle_solver = {
        let solver_visible = solver_visible.clone();
        Callback::from(move |_| {
            solver_visible.set(!*solver_visible);
        })
    };

    let download_solver = {
        let bloom_filter = bloom_filter.clone();
        Callback::from(move |_| {
            let bloom_filter = bloom_filter.clone();
            bloom_filter.set(BloomFilterResource::Loading);
            wasm_bindgen_futures::spawn_local(async move {
                let response = Request::get("http://localhost:8081/filter_173378771_1_norm.bin")
                    .send()
                    .await
                    .unwrap();

                let body = response.binary().await.unwrap();
                let filter = BloomFilter::load_from_slice(&body);
                bloom_filter.set(BloomFilterResource::Loaded(filter));
            });
        })
    };

    log::info!("Current position: {}", game_state.as_position().0);

    let current_nr_pegs = game_state.nr_pegs();

    html! {
        <div ref={div_ref} class="scaling-container" style={format!("transform: scale({})", *display_scale)}>
            <div class={overall_classes}>
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
                    {if *edit_mode {"done"} else {"edit"}}
                </button>

                <button
                    style={format!("grid-row: 7; grid-column: 6/8; opacity: {};", b2f(*has_made_first_move))}
                    onclick={toggle_solver}
                >
                    {"solver"}
                </button>

                { for Coord::all().into_iter().map(|coord| html! {
                    <div
                        class={cell_classes(coord)}
                        onmousedown={holeclick(coord)}
                        style={format!("grid-row: {}; grid-column: {};", coord.y() + 4, coord.x() + 4)}
                    />
                }) }

                { for game_state.pegs().enumerate().map(|(i, p)| {
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

            // todo: opacity transition
            <div style={format!("width: 234px; text-align: left; font-size: 0.4rem; opacity: {};", b2f(*solver_visible))}>
                {
                    match &*bloom_filter {
                        BloomFilterResource::Loaded(_) => {
                            let (backward, forward) = game_state.is_solvable();
                            html!{
                                <div>
                                    <div style="display: flex; flex-direction: row; width: 100%; text-align: center; font-size: 0.4rem; align-items: stretch">
                                        <span>{"start"}</span>
                                        <div style="flex-grow: 1; display: flex; flex-direction: row; align-items: center">

                                            <ProgressBarSegment solvability={backward} len={32 - current_nr_pegs} side={Side::Left}/>
                                            <img src="img/circle.svg"/>
                                            <ProgressBarSegment solvability={forward} len={current_nr_pegs - 1} side={Side::Right}/>
                                        </div>
                                        <span>{"end"}</span>
                                    </div>

                                    {for [(forward, "current position", "end"), (backward, "start", "current position")].map(|(solv, src, dst)| {
                                        let (path, word) = match solv {
                                            Solvability::Yes => ("img/yes.svg", "a"),
                                            _ => ("img/no.svg", "no"),
                                        };

                                        html!{
                                            <p>
                                                <img style="vertical-align: middle" src={path}/>
                                                <span style="vertical-align: middle">{format!(" There is {word} path from the {src} to the {dst}")}</span>
                                            </p>
                                        }
                                    })}
                                </div>
                            }
                        },
                        BloomFilterResource::Loading => html!{
                            {"loading"}
                        },
                        BloomFilterResource::NotRequested => html!{
                            <div>
                                <p>{"The solver can compute solution paths directly on your device. To activate the solver, roughly 10MB of data need to be downloaded once initially."}</p>
                                <button
                                    style="font-size: inherit"
                                    onclick={download_solver}
                                >
                                    {"download solver"}
                                </button>
                                <a href="todo.pdf" target="_blank" style="margin-left: 1em">
                                    <button
                                        style="font-size: inherit"
                                    >
                                        {"read the theory"}
                                    </button>
                                </a>
                            </div>
                        },
                    }
                }
            </div>
        </div>
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Side {
    Left,
    Right,
}

#[derive(Properties, Clone, PartialEq)]
struct ProgressBarSegmentProps {
    solvability: Solvability,
    len: i32,
    side: Side,
}

#[function_component]
fn ProgressBarSegment(props: &ProgressBarSegmentProps) -> Html {
    let ProgressBarSegmentProps {
        solvability,
        len,
        side,
    } = props;
    let (color, icon, borderstyle) = match solvability {
        Solvability::Yes => ("#555", "img/yes.svg", "solid"),
        Solvability::No => ("#822", "img/no.svg", "dotted"),
        Solvability::Maybe => ("#882", "img/maybe.svg", "dashed"),
    };
    let outer_margin = 4;
    let inner_margin = if *len > 0 { outer_margin } else { 0 };
    let margins = match side {
        Side::Left => format!("0 {inner_margin}px 0 {outer_margin}px"),
        Side::Right => format!("0 {outer_margin}px 0 {inner_margin}px"),
    };

    // todo: make it clickable
    html! {
        <div style={format!("flex-grow: {len}; flex-shrink: 1; flex-basis: auto; margin: {margins}; border-top: 1px {borderstyle} {color}; transition: all 200ms ease; height: 0")}>
        </div>
    }
}
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
