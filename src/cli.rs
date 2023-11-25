// Masashi Kiyomi, Tomomi Matsui. Integer Programming Based Algorithms for Peg Solitaire Problems, December 2001

// x^k_j := 1 iff `k`-th move is jump `j`
// a_ij := negative peg difference in hole `i` during jump `j`

use bitvec::{
    bitbox,
    prelude::{BitBox, Lsb0},
};
use colored::Colorize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct Position(u64);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Jump(u64, u64);

const ALL_JUMPS: [Jump; 76] = [
    Jump(192, 256),
    Jump(24576, 32768),
    Jump(3145728, 4194304),
    Jump(384, 512),
    Jump(49152, 65536),
    Jump(6291456, 8388608),
    Jump(3, 4),
    Jump(24, 32),
    Jump(768, 1024),
    Jump(98304, 131072),
    Jump(12582912, 16777216),
    Jump(402653184, 536870912),
    Jump(3221225472, 4294967296),
    Jump(1536, 2048),
    Jump(196608, 262144),
    Jump(25165824, 33554432),
    Jump(3072, 4096),
    Jump(393216, 524288),
    Jump(50331648, 67108864),
    Jump(36, 1024),
    Jump(18, 512),
    Jump(9, 256),
    Jump(1056, 131072),
    Jump(528, 65536),
    Jump(264, 32768),
    Jump(528384, 67108864),
    Jump(264192, 33554432),
    Jump(132096, 16777216),
    Jump(66048, 8388608),
    Jump(33024, 4194304),
    Jump(16512, 2097152),
    Jump(8256, 1048576),
    Jump(16908288, 536870912),
    Jump(8454144, 268435456),
    Jump(4227072, 134217728),
    Jump(553648128, 4294967296),
    Jump(276824064, 2147483648),
    Jump(138412032, 1073741824),
    Jump(100663296, 16777216),
    Jump(786432, 131072),
    Jump(6144, 1024),
    Jump(50331648, 8388608),
    Jump(393216, 65536),
    Jump(3072, 512),
    Jump(6442450944, 1073741824),
    Jump(805306368, 134217728),
    Jump(25165824, 4194304),
    Jump(196608, 32768),
    Jump(1536, 256),
    Jump(48, 8),
    Jump(6, 1),
    Jump(12582912, 2097152),
    Jump(98304, 16384),
    Jump(768, 128),
    Jump(6291456, 1048576),
    Jump(49152, 8192),
    Jump(384, 64),
    Jump(1207959552, 4194304),
    Jump(2415919104, 8388608),
    Jump(4831838208, 16777216),
    Jump(138412032, 32768),
    Jump(276824064, 65536),
    Jump(553648128, 131072),
    Jump(1056768, 64),
    Jump(2113536, 128),
    Jump(4227072, 256),
    Jump(8454144, 512),
    Jump(16908288, 1024),
    Jump(33816576, 2048),
    Jump(67633152, 4096),
    Jump(33024, 8),
    Jump(66048, 16),
    Jump(132096, 32),
    Jump(264, 1),
    Jump(528, 2),
    Jump(1056, 4),
];

impl Position {
    fn default_start() -> Position {
        Position(0b111111111111111101111111111111111)
    }

    fn default_end() -> Position {
        Position(0b000000000000000010000000000000000)
    }

    fn heart() -> Position {
        Position(0b000000000000000010000000000000000)
    }

    fn parse(s: &str) -> Position {
        let mut p = 0;
        let mut pow = 0;
        for c in s.chars() {
            match c {
                '.' => pow += 1,
                '#' => {
                    p |= 1 << pow;
                    pow += 1
                }
                _ => {}
            }
        }

        Position(p)
    }

    fn count(&self) -> i32 {
        self.0.count_ones() as i32
    }

    fn draw(&self) {
        for i in 0..33 {
            match i {
                0 => print!("  "),
                3 | 27 | 30 => print!("\n  "),
                6 | 13 | 20 => print!("\n"),
                _ => {}
            }

            if self.0 & (1 << i) != 0 {
                print!("#");
            } else {
                print!(".");
            }
        }
        print!("\n");
    }

    fn draw_with_jump(&self, jump: Jump) {
        for i in 0..33 {
            match i {
                0 => print!("  "),
                3 | 27 | 30 => print!("\n  "),
                6 | 13 | 20 => print!("\n"),
                _ => {}
            }

            let idx = 1 << i;

            if self.0 & idx != 0 {
                if jump.1 & idx != 0 {
                    print!("{}", "#".on_red());
                } else {
                    print!("#");
                }
            } else {
                if jump.0 & idx != 0 {
                    print!("{}", ".".on_blue());
                } else {
                    print!(".");
                }
            }
        }
        print!("\n");
    }

    fn can_jump(&self, jump: Jump) -> bool {
        (self.0 & jump.0).count_ones() == 2 && (self.0 & jump.1) == 0
    }
    fn apply_jump(&mut self, jump: Jump) {
        self.0 &= !jump.0;
        self.0 |= jump.1;
    }
    fn apply_jump_inverse(&mut self, jump: Jump) {
        self.0 |= jump.0;
        self.0 &= !jump.1;
    }
}

fn coordinate_to_index((x, y): (i32, i32)) -> Option<i32> {
    match (y, x) {
        (0..=1, 2..=4) => Some((x - 2) + y * 3),
        (2..=4, 0..=6) => Some(x + (y - 2) * 7 + 6),
        (5..=6, 2..=4) => Some((x - 2) + (y - 5) * 3 + 27),
        _ => None,
    }
}

fn compute_all_jumps() -> [Jump; 76] {
    let mut v = Vec::new();

    for i in 0..4 {
        let (a1, a2, a3, a4, ox, oy) = match i {
            0 => (1, 0, 0, 1, 0, 0),
            1 => (0, 1, -1, 0, 6, 0),
            2 => (-1, 0, 0, -1, 6, 6),
            3 => (0, -1, 1, 0, 0, 6),
            _ => unreachable!(),
        };

        let rot = |x: i32, y: i32| -> (i32, i32) { (x * a1 + y * a3 + ox, x * a2 + y * a4 + oy) };

        for x in 0..7 {
            for y in 0..7 {
                let idxs = (
                    coordinate_to_index(rot(x + 0, y)),
                    coordinate_to_index(rot(x + 1, y)),
                    coordinate_to_index(rot(x + 2, y)),
                );

                if let (Some(a), Some(b), Some(c)) = idxs {
                    let j1 = (1u64 << a) + (1u64 << b);
                    let j2 = 1u64 << c;
                    let j = Jump(j1 as u64, j2 as u64);
                    v.push(j);
                }
            }
        }
    }

    v.try_into().expect("should find exactly 76 jumps")
}

fn main() {
    let start = Position::default_start();
    // let end = Position::parse(
    //     r#"

    //       ##.
    //       #..
    //     ###.##.
    //     #####.#
    //     #####.#
    //       ###
    //       ###

    // "#,
    // );
    // let end = Position::parse(
    //     r#"

    //       ###
    //       #.#
    //     ###.###
    //     #.....#
    //     ###.###
    //       #.#
    //       ###

    // "#,
    // );
    // let start = end;
    let end = Position::default_end();

    search(start, end);
}

fn search(start: Position, end: Position) -> bool {
    let mut map = bitbox![u32, Lsb0; 0; 1usize<<33];

    // map.set(4681374240, true);
    // map.set(4681368992, true);
    // map.set(4613739689, true);
    // map.set(3607627296, true);
    // map.set(3539998112, true);
    // map.set(8422688800, true);
    // map.set(8355059232, true);
    // map.set(4658300448, true);
    // map.set(4590671264, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    // map.set(468137, true);
    

    let len = start.count() - end.count();
    if len < 0 {
        return false;
    }
    if len == 0 {
        return true;
    }

    let mut state = State {
        explored: 0,
        hash_skipped: 0,
        path: vec![],
        smallest: (100, Position(0)),
    };

    let ok = search_inner(start, end, len, &mut map, &mut state);

    state.path.reverse();
    for (p, j) in state.path {
        println!("{p:?} {j:?} {}", p.count());
        p.draw_with_jump(j);
        println!();
    }
    println!(
        "explored {} positions. skipped {}. result {ok}",
        state.explored, state.hash_skipped
    );

    if !ok {
        println!("smallest reached:");
        state.smallest.1.draw();
    }

    ok
}

struct State {
    explored: u64,
    hash_skipped: u64,
    path: Vec<(Position, Jump)>,
    smallest: (i32, Position),
}

fn search_inner(
    mut start: Position,
    end: Position,
    /* upper bounds, */ remaining_moves: i32,
    map: &mut BitBox<u32>,
    state: &mut State,
) -> bool {
    state.explored += 1;
    let count = start.count();
    if count < state.smallest.0 {
        state.smallest = (count, start);
    }
    // start.draw();
    // println!("");

    if remaining_moves <= 0 {
        return start == end;
    }

    for j in ALL_JUMPS {
        if !start.can_jump(j) {
            continue;
        }

        // if (upper bound of the jump ≤ 0)
        //     continue; /* It is no use searching about this jump. */
        // upper bound of the jump = upper bound of the jump − 1;

        // update the configuration start by applying the jump operation.
        start.apply_jump(j);
        if map[start.0 as usize] {
            state.hash_skipped += 1;
            start.apply_jump_inverse(j);
            continue;
        }
        map.set(start.0 as usize, true);

        if search_inner(start, end, remaining_moves - 1, map, state) {
            state.path.push((start, j));
            start.apply_jump_inverse(j);
            return true;
        } else {
            // upper bound of the jump = upper bound of the jump + 1;
            start.apply_jump_inverse(j);
        }
    }
    return false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coords() {
        let mut next_idx = 0;
        for y in 0..7 {
            for x in 0..7 {
                if let Some(idx) = coordinate_to_index((x, y)) {
                    assert_eq!(next_idx, idx);
                    next_idx += 1;
                }
            }
        }

        assert_eq!(next_idx, 33);
    }

    #[test]
    fn test_jumps() {
        assert_eq!(ALL_JUMPS, compute_all_jumps());
    }

    #[test]
    fn test_parse() {
        let parsed = Position::parse(
            r#"

          ..#
          ###
        #######
        ###.###
        #######
          ###
          ###

        "#,
        );
        assert_eq!(parsed.0, 0b111111111111111101111111111111100);
    }
}
