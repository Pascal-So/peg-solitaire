use common::Direction;
use web_sys::HtmlElement;
use yew::prelude::*;

use crate::game_state::Solvability;

#[derive(Properties, PartialEq, Clone)]
pub struct TimelineProps {
    pub nr_pegs: i32,
    pub solvability_backward: Solvability,
    pub solvability_forward: Solvability,
    pub scroll_to: Callback<i32>,
    pub step: Callback<Direction>,
}

/// Render the game board with pegs and holes, plus some surrounding buttons.
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
        <div style="display: flex; flex-direction: row; width: 100%; text-align: center; align-items: stretch">
            <span onclick={scroll_to_start}>{"start"}</span>
            <div style="flex-grow: 1; display: flex; flex-direction: row; align-items: center">

                <TimelineSegment solvability={*solvability_backward} len={32 - nr_pegs} side={Side::Left} callback={step_backward}/>
                <div style="position: relative; line-height: 0">
                    <img src="img/circle.svg"/>
                    <span style="position: absolute; top: -6px; left: 0; right: 0; font-size: 0.35rem; text-align: center">
                        {nr_pegs}
                    </span>
                </div>
                <TimelineSegment solvability={*solvability_forward} len={nr_pegs - 1} side={Side::Right} callback={step_forward}/>
            </div>
            <span onclick={scroll_to_end}>{"end"}</span>
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
