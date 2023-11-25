use yew::prelude::*;

fn coord_valid(&(x, y): &(i32, i32)) -> bool {
    x >= 2 && x <= 4 || y >= 2 && y <= 4
}

#[derive(Clone, Copy)]
struct Peg {
    coord: (i32, i32),
    key: u32,
    alive: bool,
}

#[function_component]
fn App() -> Html {
    let scale = |x: i32| x * 34;

    let selected = use_state(|| None);

    let all_coords = (0..7)
        .flat_map(|y| (0..7).map(move |x| (x, y)))
        .collect::<Vec<_>>();
    let valid_coords = all_coords
        .iter()
        .cloned()
        .filter(coord_valid)
        .collect::<Vec<_>>();

    let compute_initial_pegs = || {
        let mut key = 0;
        valid_coords
            .clone()
            .into_iter()
            .filter(|&c| c != (3, 3))
            .map(|(x, y)| {
                key += 1;
                Peg {
                    coord: (x, y),
                    key,
                    alive: true,
                }
            })
            .collect::<Vec<_>>()
    };

    let pegs = use_state(|| compute_initial_pegs());

    let reset = {
        let pegs = pegs.clone();
        let original_pegs= compute_initial_pegs();
        Callback::from(move |_| {
            log::info!("reset");
            pegs.set(original_pegs.clone());
        })
    };

    let move_peg = Callback::from({
        let pegs = pegs.clone();
        move |(src, dst): ((i32, i32), (i32, i32))| {
            let dx = dst.0 - src.0;
            let dy = dst.1 - src.1;

            if !(dx.abs() == 2 && dy == 0 || dx == 0 && dy.abs() == 2) {
                return;
            }

            let middle = ((src.0 + dst.0) / 2, (src.1 + dst.1) / 2);

            let mut pegs_tmp = Vec::clone(&pegs);

            let mut src_peg = None;
            let mut middle_peg = None;
            for peg in &mut pegs_tmp {
                if !peg.alive {
                    continue;
                }
                if peg.coord == src {
                    src_peg = Some(peg);
                } else if peg.coord == middle {
                    middle_peg = Some(peg);
                }
            }

            let Some(src_peg) = src_peg else {
                return;
            };
            let Some(middle_peg) = middle_peg else {
                return;
            };

            middle_peg.alive = false;
            src_peg.coord = dst;
            log::debug!("moving from {src:?} to {dst:?}");
            pegs.set(pegs_tmp);
        }
    });

    let holeclick = {
        let selected = selected.clone();
        let pegs = pegs.clone();
        let move_peg = move_peg.clone();

        move |&(x, y): &(i32, i32)| {
            let selected = selected.clone();
            let pegs: UseStateHandle<Vec<Peg>> = pegs.clone();
            let move_peg = move_peg.clone();

            Callback::from(move |_: MouseEvent| {
                if *selected == Some((x, y)) {
                    selected.set(None);
                    return;
                }

                let peg = pegs.iter().find(|&&p| (x, y) == p.coord && p.alive);

                if let Some(p) = peg {
                    selected.set(Some(p.coord));
                } else if let Some((sx, sy)) = *selected {
                    move_peg.emit(((sx, sy), (x, y)));
                    selected.set(None);
                }
            })
        }
    };

    let cell_classes = move |(x, y): (i32, i32)| {
        let mut classes = Classes::new();
        classes.push("game-cell");

        let x_out = x < 2 || x > 4;
        let y_out = y < 2 || y > 4;
        if x_out && y_out {
            classes.push("empty");
        }
        if *selected == Some((x, y)) {
            classes.push("selected");
        }
        classes
    };

    html! {
        <div class="game-grid">
            <button style="grid-row: 1; grid-column: 1/3;" onclick={reset}>
                {"reset"}
            </button>

            { for valid_coords.iter().map(|&(x, y)| html! {
                <div
                    class={cell_classes((x, y))}
                    onmousedown={holeclick(&(x, y))}
                    style={format!("grid-row: {}; grid-column: {};", y + 1, x + 1)}
                />
            }) }

            { for pegs.iter().map(|p| {
                let left = scale(p.coord.0);
                let top = scale(p.coord.1);
                let op = if p.alive { 1.0 } else { 0.0 };
                html!{
                <div
                    class="peg"
                    key={p.key}
                    style={format!("left: {left}px; top: {top}px; opacity: {op};")}
                />
            }}) }
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
