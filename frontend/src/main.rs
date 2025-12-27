mod components;
mod game_state;

use std::rc::Rc;

use common::{BloomFilter, coord::Coord};
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use web_sys::HtmlElement;
use yew::prelude::*;
use yew_hooks::prelude::*;

use crate::components::board::Board;
use crate::components::timeline::Timeline;
use crate::game_state::{GameAction, GameState, Mode};

/// URL where the bloom filter .bin file will be downloaded from at runtime.
const BLOOM_FILTER_URL: &'static str = match option_env!("BLOOM_FILTER_URL") {
    Some(url) => url,
    None => "filter_502115651_1_norm.bin",
};

#[derive(Eq)]
enum BloomFilterResource {
    Loaded,
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

    use_effect_with((game_state.clone(), scroll_target.clone()), {
        move |(game_state, scroll_target)| {
            let scroll_target = scroll_target.clone();
            let game_state = game_state.clone();
            *scroll_command_id.borrow_mut() += 1;
            let current_id = *scroll_command_id.borrow();
            let scroll_command_id = scroll_command_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                TimeoutFuture::new(80).await;

                if *scroll_command_id.borrow() != current_id {
                    return;
                }

                if let Some(target) = *scroll_target {
                    let nr_pegs = game_state.nr_pegs();
                    let dir = if nr_pegs < target {
                        common::Direction::Backward
                    } else if nr_pegs == target {
                        scroll_target.set(None);
                        return;
                    } else {
                        common::Direction::Forward
                    };

                    game_state.dispatch(GameAction::StepSolution { dir });
                };
            });
        }
    });

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
        let game_state = game_state.clone();
        Callback::from(move |_| {
            let bloom_filter = bloom_filter.clone();
            let game_state = game_state.clone();
            bloom_filter.set(BloomFilterResource::Loading);
            wasm_bindgen_futures::spawn_local(async move {
                let response = Request::get(BLOOM_FILTER_URL).send().await.unwrap();

                let body = response.binary().await.unwrap();
                let filter = Rc::new(BloomFilter::load_from_slice(&body));
                bloom_filter.set(BloomFilterResource::Loaded);
                game_state.dispatch(GameAction::RegisterSolver { solver: filter });
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
                        BloomFilterResource::Loaded => {
                            let (backward, forward) = game_state.is_solvable();
                            let scroll_to = {
                                let scroll_target = scroll_target.clone();
                                Callback::from(move |nr_pegs| {
                                    scroll_target.set(Some(nr_pegs));
                                })
                            };
                            let step = {
                                let game_state = game_state.clone();
                                let scroll_target = scroll_target.clone();
                                Callback::from(move |dir| {
                                    scroll_target.set(None);
                                    game_state.dispatch(GameAction::StepSolution {dir});
                                })
                            };

                            html!{
                                <div>
                                    <Timeline nr_pegs={current_nr_pegs} solvability_forward={forward} solvability_backward={backward} scroll_to={scroll_to} step={step} />

                                    {for [(forward, "current position", "end"), (backward, "start", "current position")].map(|(solv, src, dst)| {
                                        let (path, word) = if solv.solvable() {
                                            ("img/yes.svg", "a")
                                        } else {
                                            ("img/no.svg", "no")
                                        };

                                        html!{
                                            <p style="margin: 2px 0">
                                                <img style="vertical-align: middle; height: 0.4rem" src={path}/>
                                                <span style="vertical-align: middle">{format!(" There is {word} path from the {src} to the {dst}")}</span>
                                            </p>
                                        }
                                    })}

                                    <ExternalLinks/>
                                </div>
                            }
                        },
                        BloomFilterResource::Loading => html!{
                            {"loading..."}
                        },
                        BloomFilterResource::NotRequested => html!{
                            <div>
                                <p>{"The solver can compute solution paths directly on your device. To activate the solver, roughly 10MB of data will be loaded once initially."}</p>
                                <button
                                    style="font-size: inherit; margin-right: 1em"
                                    onclick={download_solver}
                                >
                                    {"activate solver"}
                                </button>
                                <ExternalLinks/>
                            </div>
                        },
                    }
                }
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ExternalLinkProps {
    pub text: &'static str,
    pub link: &'static str,
}

#[function_component]
fn ExternalLink(ExternalLinkProps { text, link }: &ExternalLinkProps) -> Html {
    html! {
        <a href={*link} target="_blank" style="margin-right: 1em">
            <span style="vertical-align: middle">
                {text}
            </span><img
                src="img/external-link.svg"
                style="height: 0.4rem; margin: 1px 0 0 2px; vertical-align: middle"
            />
        </a>
    }
}

#[function_component]
fn ExternalLinks() -> Html {
    html! {
        <span>
            <ExternalLink
                text={"read the theory"}
                link={"https://projects.pascalsommer.ch/pegsolitaire/precomputing-pegsolitaire-paper.pdf"}
            />

            <ExternalLink
                text={"source code"}
                link={"https://github.com/Pascal-So/peg-solitaire"}
            />
        </span>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
