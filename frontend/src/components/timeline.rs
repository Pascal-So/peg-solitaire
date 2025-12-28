use common::Direction;
use web_sys::HtmlElement;
use yew::prelude::*;

use crate::{components::b2f, game_state::Solvability};

#[derive(Properties, PartialEq, Clone)]
pub struct TimelineProps {
    pub nr_pegs: i32,
    pub solvability_backward: Solvability,
    pub solvability_forward: Solvability,
    pub scroll_to: Callback<i32>,
    pub step: Callback<Direction>,
}

/// A progress bar/timeline showing the progression of the game's solution from
/// the start to end position. The user can scrub through the solution by
/// clicking on the timeline, kinda like in a video player.
#[function_component]
pub fn Timeline(
    TimelineProps {
        nr_pegs,
        solvability_backward,
        solvability_forward,
        scroll_to,
        step,
    }: &TimelineProps,
) -> Html {
    let scroll_to_start = {
        let scroll_to = scroll_to.clone();
        move |_| scroll_to.emit(32)
    };
    let scroll_to_end = {
        let scroll_to = scroll_to.clone();
        move |_| scroll_to.emit(1)
    };
    let step_backward = {
        let step = step.clone();
        move |_| step.emit(Direction::Backward)
    };
    let step_forward = {
        let step = step.clone();
        move |_| step.emit(Direction::Forward)
    };

    html! {
        <div style="display: flex; flex-direction: row; width: 100%; text-align: center; align-items: stretch; user-select: none; margin-bottom: 1em">
            <TimelineEndpoint solvability={*solvability_backward} side={Side::Left} callback={scroll_to_start} />
            <div style="flex-grow: 1; display: flex; flex-direction: row">
                <TimelineSegment solvability={*solvability_backward} len={32 - nr_pegs} side={Side::Left} callback={step_backward}/>
                <div class="timeline-segment" style="align-items: center">
                    <span class="timeline-segment-upper" style="font-size: 0.35rem">
                        {nr_pegs}
                    </span>
                    <div class="timeline-segment-lower">
                        <img src="img/circle.svg"/>
                    </div>
                </div>
                <TimelineSegment solvability={*solvability_forward} len={nr_pegs - 1} side={Side::Right} callback={step_forward}/>
            </div>
            <TimelineEndpoint solvability={*solvability_forward} side={Side::Right} callback={scroll_to_end} />
        </div>
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Side {
    /// To the left of the current playhead, i.e. adding more pegs
    Left,
    /// To the right of the current playhead, i.e. with fewer pegs
    Right,
}

#[derive(Properties, Clone, PartialEq)]
struct TimelineSegmentProps {
    solvability: Solvability,
    len: i32,
    side: Side,
    callback: Callback<i32>,
}

#[function_component]
fn TimelineSegment(
    TimelineSegmentProps {
        solvability,
        len,
        side,
        callback,
    }: &TimelineSegmentProps,
) -> Html {
    let (color, borderstyle, clickable) = match solvability {
        Solvability::Solvable | Solvability::Solved => ("#555", "solid", true),
        Solvability::Unsolvable => ("#822", "dotted", false),
        Solvability::Unknown => ("#882", "dashed", false),
    };
    let outer_margin = 4;
    let inner_margin = if *len > 0 { outer_margin } else { 0 };
    let margins = match side {
        Side::Left => format!("0 {inner_margin}px 0 {outer_margin}px"),
        Side::Right => format!("0 {outer_margin}px 0 {inner_margin}px"),
    };

    let div_ref = use_node_ref();

    let classes = classes!("timeline-segment", clickable.then_some("clickable"));

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

    let show_arrow = *len > 0 && clickable;
    let arrow;
    let arrow_style;
    match side {
        Side::Left => {
            arrow = "img/chevron-left.svg";
            arrow_style = "justify-content: end";
        }
        Side::Right => {
            arrow = "img/chevron-right.svg";
            arrow_style = "justify-content: start";
        }
    };

    html! {
        <div ref={div_ref} class={classes} style={format!("flex-grow: {len}; margin: {margins}")} onclick={onclick}>
            <div class="timeline-segment-upper" style={arrow_style}>
                if show_arrow {
                    <img src={arrow} class="timeline-icon" style="width: 6px" />
                }
            </div>
            <div class="timeline-segment-lower">
                <div class="timeline-line" style={format!("border-top-style: {borderstyle}; border-top-color: {color}")}>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
struct TimelineEndpointProps {
    solvability: Solvability,
    side: Side,
    callback: Callback<()>,
}

#[function_component]
fn TimelineEndpoint(
    TimelineEndpointProps {
        solvability,
        side,
        callback,
    }: &TimelineEndpointProps,
) -> Html {
    let icon;
    let text;
    match side {
        Side::Left => {
            icon = "img/skip-back.svg";
            text = "start";
        }
        Side::Right => {
            icon = "img/skip-forward.svg";
            text = "end";
        }
    }

    let solvable = *solvability == Solvability::Solvable;
    let callback = callback.clone();

    let classes = classes!("timeline-segment", solvable.then_some("clickable"));

    html! {
        <div style="min-width: 12px; flex: none" class={classes} onclick={move |_| callback.emit(())}>
            <span class="timeline-segment-upper" style={format!("opacity: {}", b2f(solvable))}>
                <img src={icon} class="timeline-icon" style="width: 12px" />
            </span>
            <div class="timeline-segment-lower">
                <span style="margin-bottom: 1px">{text}</span>
            </div>
        </div>
    }
}
