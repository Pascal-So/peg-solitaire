use bitvec::{bitbox, boxed::BitBox, prelude::Lsb0};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Position(u64);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Jump(u64, u64);

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
    pub fn default_start() -> Position {
        Position(0b111111111111111101111111111111111)
    }

    pub fn default_end() -> Position {
        Position(0b000000000000000010000000000000000)
    }

    fn heart() -> Position {
        Position(0b000000000000000010000000000000000)
    }

    fn count(&self) -> i32 {
        self.0.count_ones() as i32
    }

    fn can_jump(&self, jump: Jump) -> bool {
        (self.0 & jump.0).count_ones() == 2 && (self.0 & jump.1) == 0
    }
    fn can_jump_inverse(&self, jump: Jump) -> bool {
        (self.0 & jump.0) == 0 && (self.0 & jump.1) > 0
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

pub fn search(start: Position, end: Position) -> bool {
    let mut map = bitbox![u32, Lsb0; 0; 1usize<<33];

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
    };

    let ok = search_inner(start, end, len, &mut map, &mut state);

    state.path.reverse();
    for (p, j) in state.path {
        println!("{p:?} {j:?} {}", p.count());
    }
    println!(
        "explored {} positions. skipped {}. result {ok}",
        state.explored, state.hash_skipped
    );

    ok
}

struct State {
    explored: u64,
    hash_skipped: u64,
    path: Vec<(Position, Jump)>,
}

fn search_inner(
    mut start: Position,
    end: Position,
    remaining_moves: i32,
    map: &mut BitBox<u32>,
    state: &mut State,
) -> bool {
    state.explored += 1;

    if remaining_moves <= 0 {
        return start == end;
    }

    for j in ALL_JUMPS {
        if !start.can_jump(j) {
            continue;
        }

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
            start.apply_jump_inverse(j);
        }
    }
    return false;
}

#[cfg(test)]
fn compute_all_jumps() -> [Jump; 76] {
    use common::coordinate_to_index;

    let mut v = Vec::new();

    for i in 0..4 {
        let (a1, a2, a3, a4, ox, oy) = match i {
            0 => (1, 0, 0, 1, 0, 0),
            1 => (0, 1, -1, 0, 6, 0),
            2 => (-1, 0, 0, -1, 6, 6),
            3 => (0, -1, 1, 0, 0, 6),
            _ => unreachable!(),
        };

        let rot = |x: i16, y: i16| -> (i16, i16) { (x * a1 + y * a3 + ox, x * a2 + y * a4 + oy) };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jumps() {
        assert_eq!(ALL_JUMPS, compute_all_jumps());
    }
}
