mod components;
mod game_state;

use std::rc::Rc;

use common::{BloomFilter, Direction, coord::Coord};
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use web_sys::HtmlElement;
use yew::prelude::*;
use yew_hooks::prelude::*;
use yew_icons::{Icon, IconId};

use crate::components::board::Board;
use crate::game_state::{GameAction, GameState, Mode, Solvability};

const PX_HOLE_DISTANCE: i16 = 34;
const BLOOM_FILTER_URL: Option<&'static str> = option_env!("BLOOM_FILTER_URL");

#[derive(Eq)]
enum BloomFilterResource {
    Loaded(Rc<BloomFilter>),
    Loading,
    NotRequested,
}

/// We intentionally broaden the equivalence so that any two bloom filters are
/// considered equal. This is done to speed up the use_effect_with comparison.
/// This does not lead to problems because we never replace the bloom filter
/// once it has been loaded.
impl PartialEq for BloomFilterResource {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

#[function_component]
fn App() -> Html {
    let b2f = |b: bool| if b { 1.0 } else { 0.0 };

    let game_state = use_reducer(|| GameState::new());
    let display_scale = use_state(|| 1.0);
    let bloom_filter = use_state(|| BloomFilterResource::NotRequested);
    let div_ref = use_node_ref();
    let solver_visible = use_state(|| false);
    let scroll_target = use_state(|| None);
    let scroll_command_id = use_mut_ref(|| 0u64);

    // use_effect_with(
    //     (
    //         game_state.clone(),
    //         solver_visible.clone(),
    //         bloom_filter.clone(),
    //     ),
    //     |(game_state, solver_visible, bloom_filter)| {
    //         if **solver_visible && let BloomFilterResource::Loaded(bloom_filter) = &**bloom_filter {
    //             game_state.set(GameState::clone(&**game_state).rerun_solver(bloom_filter));
    //         }
    //     },
    // );

    // use_effect_with((game_state.clone(), scroll_target.clone()), {
    //     move |(game_state, scroll_target)| {
    //         let scroll_target = scroll_target.clone();
    //         let game_state = game_state.clone();
    //         *scroll_command_id.borrow_mut() += 1;
    //         let current_id = *scroll_command_id.borrow();
    //         let scroll_command_id = scroll_command_id.clone();

    //         wasm_bindgen_futures::spawn_local(async move {
    //             TimeoutFuture::new(80).await;

    //             if *scroll_command_id.borrow() != current_id {
    //                 return;
    //             }

    //             if let Some(target) = *scroll_target {
    //                 let nr_pegs = game_state.nr_pegs();
    //                 let dir = if nr_pegs < target {
    //                     common::Direction::Backward
    //                 } else if nr_pegs == target {
    //                     scroll_target.set(None);
    //                     return;
    //                 } else {
    //                     common::Direction::Forward
    //                 };

    //                 game_state.set(GameState::clone(&*game_state).move_along_solve_path(dir));
    //             };
    //         });
    //     }
    // });

    let reset = {
        let game_state = game_state.clone();
        let scroll_target = scroll_target.clone();
        Callback::from(move |_| {
            game_state.dispatch(GameAction::Reset);
            scroll_target.set(None);
        })
    };

    let undo = {
        let game_state = game_state.clone();
        let scroll_target = scroll_target.clone();
        if game_state.can_undo() {
            Some(Callback::from(move |_| {
                scroll_target.set(None);
                game_state.dispatch(GameAction::Undo);
            }))
        } else {
            None
        }
    };

    let redo = {
        let game_state = game_state.clone();
        let scroll_target = scroll_target.clone();
        if game_state.can_redo() {
            Some(Callback::from(move |_| {
                scroll_target.set(None);
                game_state.dispatch(GameAction::Redo);
            }))
        } else {
            None
        }
    };

    let holeclick = {
        let game_state = game_state.clone();
        let scroll_target = scroll_target.clone();

        move |coord: Coord| {
            scroll_target.set(None);
            game_state.dispatch(GameAction::ClickHole { coord });
        }
    };

    let edit_mode = game_state.mode == Mode::Edit;

    let cell_classes = {
        let selected = game_state.selected_coord();

        move |coord: Coord| {
            let mut classes = Classes::new();
            classes.push("game-cell");
            if selected == Some(coord) && !edit_mode {
                classes.push("selected");
            }
            classes
        }
    };

    let mut overall_classes = Classes::new();
    overall_classes.push("game-grid");
    if edit_mode {
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
        let new_mode = match game_state.mode {
            Mode::Play => Mode::Edit,
            Mode::Edit => Mode::Play,
        };
        let game_state = game_state.clone();
        Callback::from(move |_| {
            game_state.dispatch(GameAction::SetMode { mode: new_mode });
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
                let url = BLOOM_FILTER_URL.unwrap_or("/filter_173378771_1_norm.bin");
                let response = Request::get(url).send().await.unwrap();

                let body = response.binary().await.unwrap();
                let filter = BloomFilter::load_from_slice(&body);
                bloom_filter.set(BloomFilterResource::Loaded(Rc::new(filter)));
            });
        })
    };

    log::info!("Current position: {:?}", game_state.as_position());

    let current_nr_pegs = game_state.nr_pegs();

    html! {
        <div ref={div_ref} class="scaling-container" style={format!("transform: scale({})", *display_scale)}>
            <Board
                has_made_first_move={game_state.has_made_first_move()}
                edit_mode={edit_mode}
                selected={game_state.selected_coord()}
                reset={reset}
                undo={undo}
                redo={redo}
                holeclick={holeclick}
                toggle_solver={toggle_solver}
                toggle_edit_mode={edit}
                pegs={game_state.pegs()}
            />

            <div class="solver-box" style={format!("opacity: {};", b2f(*solver_visible))}>
                {
                    match &*bloom_filter {
                        BloomFilterResource::Loaded(_) => {
                            let (backward, forward) = (Solvability::No, Solvability::No); // game_state.is_solvable();
                            let move_backward = {
                                let game_state = game_state.clone();
                                let scroll_target = scroll_target.clone();
                                Callback::from(move |_| {
                                    scroll_target.set(None);
                                    game_state.dispatch(GameAction::StepSolution {dir: Direction::Backward});
                                })
                            };
                            let move_forward = {
                                let game_state = game_state.clone();
                                let scroll_target = scroll_target.clone();
                                Callback::from(move |_| {
                                    scroll_target.set(None);
                                    game_state.dispatch(GameAction::StepSolution {dir: Direction::Forward});
                                })
                            };
                            let move_to_start = {
                                let scroll_target = scroll_target.clone();
                                Callback::from(move |_| {
                                    scroll_target.set(Some(32));
                                })
                            };
                            let move_to_end = {
                                let scroll_target = scroll_target.clone();
                                Callback::from(move |_| {
                                    scroll_target.set(Some(1));
                                })
                            };

                            html!{
                                <div>
                                    <div style="display: flex; flex-direction: row; width: 100%; text-align: center; align-items: stretch">
                                        <span onclick={move_to_start}>{"start"}</span>
                                        <div style="flex-grow: 1; display: flex; flex-direction: row; align-items: center">

                                            <ProgressBarSegment solvability={backward} len={32 - current_nr_pegs} side={Side::Left} callback={move_backward}/>
                                            <div style="position: relative; line-height: 0">
                                                <img src="img/circle.svg"/>
                                                <span style="position: absolute; top: -6px; left: 0; right: 0; font-size: 0.35rem; text-align: center">
                                                    {game_state.nr_pegs()}
                                                </span>
                                            </div>
                                            <ProgressBarSegment solvability={forward} len={current_nr_pegs - 1} side={Side::Right} callback={move_forward}/>
                                        </div>
                                        <span onclick={move_to_end}>{"end"}</span>
                                    </div>

                                    {for [(forward, "current position", "end"), (backward, "start", "current position")].map(|(solv, src, dst)| {
                                        let (path, word) = match solv {
                                            Solvability::Yes => ("img/yes.svg", "a"),
                                            _ => ("img/no.svg", "no"),
                                        };

                                        html!{
                                            <p style="margin: 2px 0">
                                                <img style="vertical-align: middle; height: 0.4rem" src={path}/>
                                                <span style="vertical-align: middle">{format!(" There is {word} path from the {src} to the {dst}")}</span>
                                            </p>
                                        }
                                    })}

                                    <TheoryLink/>
                                </div>
                            }
                        },
                        BloomFilterResource::Loading => html!{
                            {"loading..."}
                        },
                        BloomFilterResource::NotRequested => html!{
                            <div>
                                <p>{"The solver can compute solution paths directly on your device. To activate the solver, roughly 10MB of data need to be loaded once initially."}</p>
                                <button
                                    style="font-size: inherit; margin-right: 1em"
                                    onclick={download_solver}
                                >
                                    {"activate solver"}
                                </button>
                                <TheoryLink/>
                            </div>
                        },
                    }
                }
            </div>
        </div>
    }
}

#[function_component]
fn TheoryLink() -> Html {
    html! {
        <a href="https://projects.pascalsommer.ch/pegsolitaire/precomputing-pegsolitaire-paper.pdf" target="_blank">
            <span style="vertical-align: middle">
                {"read the theory"}
            </span><img
                src="img/external-link.svg"
                style="height: 0.4rem; margin: 1px 0 0 2px; vertical-align: middle"
            />
        </a>
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
    callback: Callback<i32>,
}

#[function_component]
fn ProgressBarSegment(props: &ProgressBarSegmentProps) -> Html {
    let ProgressBarSegmentProps {
        solvability,
        len,
        side,
        callback,
    } = props;
    let (color, borderstyle, clickable) = match solvability {
        Solvability::Yes => ("#555", "solid", true),
        Solvability::No => ("#822", "dotted", false),
        Solvability::Maybe => ("#882", "dashed", false),
    };
    let outer_margin = 4;
    let inner_margin = if *len > 0 { outer_margin } else { 0 };
    let margins = match side {
        Side::Left => format!("0 {inner_margin}px 0 {outer_margin}px"),
        Side::Right => format!("0 {outer_margin}px 0 {inner_margin}px"),
    };

    let div_ref = use_node_ref();

    let classes = format!(
        "progress-bar-segment {}",
        if clickable { "clickable" } else { "" }
    );

    let onclick = {
        let callback = callback.clone();
        let div_ref = div_ref.clone();
        let len = *len;
        Callback::from(move |ev: MouseEvent| {
            let Some(div) = div_ref.cast::<HtmlElement>() else {
                return;
            };
            let fraction = ev.offset_x() as f64 / div.client_width() as f64;
            let position = ((fraction * len as f64) as i32).min(len - 1).max(0);

            callback.emit(position);
        })
    };

    html! {
        <div ref={div_ref} class={classes} style={format!("flex-grow: {len}; margin: {margins}")} onclick={onclick}>
            <div style={format!("border-top-style: {borderstyle}; border-top-color: {color}")}>
            </div>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
