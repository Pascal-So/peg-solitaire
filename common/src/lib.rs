pub mod coord;
pub mod debruijn;

use std::fmt::Display;
#[cfg(not(target_family = "wasm"))]
use std::path::Path;

use bincode::config;
use bitvec::{bitbox, boxed::BitBox, prelude::Lsb0};
use rand::{SeedableRng, seq::SliceRandom};
use rand_pcg::Pcg64Mcg;

use crate::{coord::Coord, debruijn::de_bruijn_solvable};

pub const NR_PEGS: usize = 32;
pub const NR_HOLES: usize = 33;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Position(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Jump {
    remove_bits: u64,
    add_bits: u64,
    pub src: Coord,
    mid: Coord,
    pub dst: Coord,
}

impl Jump {
    pub fn from_coordinate_pair(src: Coord, dst: Coord) -> Option<Jump> {
        let (dx, dy) = dst - src;
        if !matches!((dx.abs(), dy.abs()), (0, 2) | (2, 0)) {
            // coordinates are not axis-aligned and two holes apart
            return None;
        }

        let mid = src
            .shift(dx / 2, dy / 2)
            .expect("center between valid positions should be valid");

        let remove_bits = src.bitmask() | mid.bitmask();
        let add_bits = dst.bitmask();

        Some(Jump {
            remove_bits,
            add_bits,
            src,
            mid,
            dst,
        })
    }
}

impl Position {
    pub fn from_ascii(lines: [&str; 7]) -> Self {
        let mut position = 0;
        let mut current_peg_bitmask = 1;
        let max_bitmask = 1 << 33;
        for line in lines {
            for c in line.chars() {
                match c {
                    '.' => {
                        current_peg_bitmask *= 2;
                    }
                    '#' => {
                        position += current_peg_bitmask;
                        current_peg_bitmask *= 2;
                    }
                    ' ' => {}
                    _ => panic!("invalid char {c} in ascii"),
                }

                if current_peg_bitmask > max_bitmask {
                    panic!("too many chars in ascii");
                }
            }
        }
        if current_peg_bitmask < max_bitmask {
            panic!("not enough chars in ascii");
        }
        Self(position)
    }

    pub fn default_start() -> Position {
        Self::from_ascii([
            "    ###    ",
            "    ###    ",
            "  #######  ",
            "  ###.###  ",
            "  #######  ",
            "    ###    ",
            "    ###    ",
        ])
    }

    pub fn default_end() -> Position {
        Self::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  ...#...  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ])
    }

    /// Number of occupied holes in this position
    pub fn count(&self) -> i32 {
        self.0.count_ones() as i32
    }

    pub fn inverse(&self) -> Self {
        Self(self.0 ^ ((1u64 << 33) - 1))
    }

    pub fn can_jump(&self, jump: Jump) -> bool {
        (self.0 & jump.add_bits) == 0 && (self.0 & jump.remove_bits).count_ones() == 2
    }
    pub fn can_jump_inverse(&self, jump: Jump) -> bool {
        (self.0 & jump.remove_bits) == 0 && (self.0 & jump.add_bits) > 0
    }
    pub fn apply_jump(&self, jump: Jump) -> Position {
        let mut next = self.0;
        next &= !jump.remove_bits;
        next |= jump.add_bits;
        Position(next)
    }
    pub fn apply_jump_inverse(&self, jump: Jump) -> Position {
        let mut next = self.0;
        next |= jump.remove_bits;
        next &= !jump.add_bits;
        Position(next)
    }
    pub fn rotate(&self) -> Position {
        let pos = self.0 & ((1u64 << 33) - 1);

        let mut out = 0;
        let mut out_mask = 1;

        for x in (0..7).rev() {
            let mut bit_index;
            let start_y;
            let count;
            if (2..=4).contains(&x) {
                bit_index = 0 + (x - 2);
                start_y = 0;
                count = 7;
            } else {
                bit_index = 6 + x;
                start_y = 2;
                count = 3;
            }

            for y in start_y..(start_y + count) {
                let bitmask = 1u64 << bit_index;

                if pos & bitmask > 0 {
                    out += out_mask;
                }
                out_mask *= 2;

                bit_index += match y {
                    1 | 4 => 5,
                    2 | 3 => 7,
                    _ => 3,
                };
            }
        }

        Position(out)
    }

    pub fn mirror(&self) -> Position {
        let pos = self.0 & ((1u64 << 33) - 1);

        let mut out = 0;
        let short_row_mask: u64 = 0b111;
        let long_row_mask: u64 = 0b1111111;

        out |= (pos & (short_row_mask << 0)) << 30;
        out |= (pos & (short_row_mask << 3)) << 24;
        out |= (pos & (long_row_mask << 6)) << 14;
        out |= (pos & (long_row_mask << 13)) << 0;
        out |= (pos & (long_row_mask << 20)) >> 14;
        out |= (pos & (short_row_mask << 27)) >> 24;
        out |= (pos & (short_row_mask << 30)) >> 30;

        Position(out)
    }

    pub fn normalize(&self) -> Position {
        let mut candidates = [*self; 8];

        for i in 1..4 {
            candidates[i] = candidates[i - 1].rotate();
        }
        for i in 4..8 {
            candidates[i] = candidates[i - 4].mirror();
        }

        Position(candidates.iter().map(|p| p.0).min().unwrap())
    }

    pub fn is_occupied(&self, coord: Coord) -> bool {
        self.0 & coord.bitmask() > 0
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let side_space = "  ";

        let pos = self.0;
        let mut mask = 1u64;
        let mut get_bit_char = || {
            let char = if pos & mask > 0 { '#' } else { '.' };
            mask *= 2;
            char
        };

        for _ in 0..2 {
            write!(f, "{side_space}")?;
            for _ in 0..3 {
                write!(f, "{}", get_bit_char())?;
            }
            writeln!(f, "{side_space}")?;
        }
        for _ in 0..3 {
            for _ in 0..7 {
                write!(f, "{}", get_bit_char())?;
            }
            writeln!(f)?;
        }
        for _ in 0..2 {
            write!(f, "{side_space}")?;
            for _ in 0..3 {
                write!(f, "{}", get_bit_char())?;
            }
            writeln!(f, "{side_space}")?;
        }

        Ok(())
    }
}

#[cfg_attr(not(target_family = "wasm"), derive(bincode::Encode))]
#[derive(bincode::Decode)]
pub struct BloomFilter {
    nr_bits: u32,
    k: u32,
    bits: BincodeBitBox,
}

impl Eq for BloomFilter {}

impl PartialEq for BloomFilter {
    fn eq(&self, other: &Self) -> bool {
        let nr_bits = self.nr_bits;
        self.nr_bits == other.nr_bits
            && self.bits.0[..nr_bits as usize] == other.bits.0[..nr_bits as usize]
    }
}

impl BloomFilter {
    pub fn new(nr_bits: u32, k: u32) -> Self {
        let filter = Self {
            nr_bits,
            k,
            bits: BincodeBitBox(bitbox![u32, Lsb0; 0; nr_bits as usize]),
        };
        filter.check_valid_k();
        filter
    }

    /// The size of the bloom filter in bits
    pub fn nr_bits(&self) -> u32 {
        self.nr_bits
    }

    fn hash(&self, pos: Position) -> usize {
        let nr_bits = self.nr_bits() as u64;
        (pos.0 % nr_bits) as usize
    }

    pub fn insert(&mut self, position: Position) {
        let hash = self.hash(position);
        self.bits.0.set(hash, true);
    }

    /// Check if a value is present in the filter.
    ///
    /// This may return false positives, but never false negatives.
    pub fn query(&self, position: Position) -> bool {
        let hash = self.hash(position);
        *self.bits.0.get(hash).unwrap()
    }

    fn check_valid_k(&self) {
        assert_eq!(self.k, 1, "only k=1 supported currently");
    }

    pub fn load_from_slice(data: &[u8]) -> Self {
        let (filter, _) =
            bincode::decode_from_slice::<BloomFilter, _>(data, bincode_config()).unwrap();
        filter.check_valid_k();
        filter
    }

    #[cfg(test)]
    /// Generate a Bloom Filter that returns true on every query.
    fn always_true() -> Self {
        Self {
            nr_bits: 1,
            k: 1,
            bits: BincodeBitBox(bitbox![u32, Lsb0; 1; 1]),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
impl BloomFilter {
    pub fn save_to_file(&self, path: impl AsRef<Path>) {
        let mut file = std::fs::File::create(path).unwrap();
        bincode::encode_into_std_write(self, &mut file, bincode_config()).unwrap();
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Self {
        let mut file = std::fs::File::open(path).unwrap();
        bincode::decode_from_std_read(&mut file, bincode_config()).unwrap()
    }
}

const BYTES_LIMIT_BLOOM_FILTER: usize = 100 * 1024 * 1024;
fn bincode_config() -> config::Configuration<
    config::LittleEndian,
    config::Fixint,
    config::Limit<BYTES_LIMIT_BLOOM_FILTER>,
> {
    config::Configuration::default()
}

pub struct BincodeBitBox(pub BitBox<u32>);

impl<'de, Context> bincode::BorrowDecode<'de, Context> for BincodeBitBox {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        Ok(Self(BitBox::from_boxed_slice(
            bincode::BorrowDecode::borrow_decode(decoder)?,
        )))
    }
}

impl<Context> bincode::Decode<Context> for BincodeBitBox {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(Self(BitBox::from_boxed_slice(bincode::Decode::decode(
            decoder,
        )?)))
    }
}

impl bincode::Encode for BincodeBitBox {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(self.0.as_raw_slice(), encoder)?;
        Ok(())
    }
}

/// Time (or move) direction
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    /// Forward move, removing pegs from the board
    Forward,
    /// Backward move, adding pegs to the board
    Backward,
}

#[derive(PartialEq, Eq)]
pub enum SolveResult {
    Solved(Vec<Jump>),
    Unsolvable,
    TimedOut,
}

/// Additional statistics about the solve process
pub struct SolveInfo {
    pub nr_steps: u32,
    pub nr_attempts: u32,
}

/// Find a path from the given position to the default end position using DFS
/// based on a bloom filter.
/// If the direction is set to backward, then we search a path to the start
/// instead, i.e. solving the problem in reverse.
pub fn solve_with_bloom_filter(
    mut pos: Position,
    filter: &BloomFilter,
    dir: Direction,
    seed: u64,
) -> (SolveResult, SolveInfo) {
    let mut solve_info = SolveInfo {
        nr_steps: 0,
        nr_attempts: 0,
    };
    if !de_bruijn_solvable(pos) {
        return (SolveResult::Unsolvable, solve_info);
    }

    fn depth_first_search(
        pos: Position,
        filter: &BloomFilter,
        end: Position,
        nr_steps: &mut u32,
        jumps: &[Jump; 76],
        step_limit: u32,
    ) -> SolveResult {
        if *nr_steps > step_limit {
            return SolveResult::TimedOut;
        }
        *nr_steps += 1;

        for &jump in jumps {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);

                // Check if we've reached the end position
                if next == end {
                    return SolveResult::Solved(vec![jump]);
                }

                // If the next position only has a single peg left somewhere
                // other than in the end position then we skip it.
                if next.count() == 1 {
                    continue;
                }

                if !filter.query(next.normalize()) {
                    continue;
                }

                match depth_first_search(next, filter, end, nr_steps, jumps, step_limit) {
                    SolveResult::Solved(mut list) => {
                        list.push(jump);
                        return SolveResult::Solved(list);
                    }
                    SolveResult::Unsolvable => {}
                    SolveResult::TimedOut => return SolveResult::TimedOut,
                }
            }
        }

        SolveResult::Unsolvable
    }

    if !filter.query(pos.normalize()) {
        return (SolveResult::Unsolvable, solve_info);
    }

    let mut jumps = all_jumps();
    let mut rng = Pcg64Mcg::seed_from_u64(seed);

    if dir == Direction::Backward {
        pos = pos.inverse();
    }

    let end = Position::default_end();
    if pos == end {
        return (SolveResult::Solved(vec![]), solve_info);
    }

    let mut step_limit = 50;
    let nr_attempts = 100;
    for attempt in 0..nr_attempts {
        let last_attempt = attempt + 1 == nr_attempts;
        if last_attempt {
            step_limit = 10000;
        }

        let mut nr_steps = 0;
        let result = depth_first_search(pos, filter, end, &mut nr_steps, &jumps, step_limit);
        solve_info.nr_steps += nr_steps;
        solve_info.nr_attempts += 1;

        match result {
            SolveResult::Solved(mut list) => {
                list.reverse();
                return (SolveResult::Solved(list), solve_info);
            }
            SolveResult::Unsolvable => return (SolveResult::Unsolvable, solve_info),
            SolveResult::TimedOut => {}
        }

        jumps.shuffle(&mut rng);
    }

    (SolveResult::TimedOut, solve_info)
}

pub fn all_jumps() -> [Jump; 76] {
    let mut v = Vec::new();

    for direction in 0..4 {
        let jumps = Coord::all().into_iter().filter_map(|coord| {
            let mut coord_a = coord;
            let mut coord_b = coord_a.shift(1, 0)?;
            let mut coord_c = coord_a.shift(2, 0)?;

            for _ in 0..direction {
                coord_a = coord_a.rotate();
                coord_b = coord_b.rotate();
                coord_c = coord_c.rotate();
            }

            let remove_bits = coord_a.bitmask() | coord_b.bitmask();
            let add_bits = coord_c.bitmask();
            let j = Jump {
                remove_bits,
                add_bits,
                src: coord_a,
                mid: coord_b,
                dst: coord_c,
            };
            Some(j)
        });

        v.extend(jumps);
    }

    v.try_into().expect("should find exactly 76 jumps")
}

#[cfg(test)]
mod tests {
    use proptest::proptest;
    use rand::{RngCore, SeedableRng};
    use tempfile::tempdir;

    use crate::coord::Coord;

    use super::*;

    fn position_from_ascii_multiline(text: &str) -> Position {
        let lines = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        Position::from_ascii(lines)
    }

    #[test]
    // test if the coordinate bits appear in the expected sequential order
    fn test_coords() {
        let mut next_mask = 1;
        for coord in Coord::all() {
            assert_eq!(next_mask, coord.bitmask());
            next_mask *= 2;
        }

        assert_eq!(next_mask, 1u64 << 33);
    }

    #[test]
    fn test_jump_list_contains_all_unique_jumps() {
        let jumps = all_jumps();

        for i in 0..jumps.len() {
            for j in 0..i {
                assert_ne!(jumps[i], jumps[j]);
            }
        }
    }

    #[test]
    fn test_from_ascii() {
        let a = Position::from_ascii([
            "    #..    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ]);
        assert_eq!(a.0, 1);

        let a = Position::from_ascii([
            "    .#.    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ]);
        assert_eq!(a.0, 2);

        let a = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  .......  ",
            "    ...    ",
            "    ..#    ",
        ]);
        assert_eq!(a.0, 1u64 << 32);
    }

    proptest! {
        #[test]
        fn test_from_ascii_reverses_print(mask in 0u64..8589934592) {
            let position = Position(mask);
            let ascii = format!("{position}");
            let parsed = position_from_ascii_multiline(&ascii);
            assert_eq!(position, parsed);
        }
    }

    #[test]
    fn test_rotate() {
        let a = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .##....  ",
            "  ..#....  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ]);

        let b = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  ..##...  ",
            "    #..    ",
            "    ...    ",
        ]);

        assert_eq!(a.rotate(), b);
    }

    #[test]
    fn test_mirror() {
        let a = Position::from_ascii([
            "    ...    ",
            "    ..#    ",
            "  .##....  ",
            "  ..#....  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ]);

        let b = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  ..#....  ",
            "  .##....  ",
            "    ..#    ",
            "    ...    ",
        ]);

        assert_eq!(a.mirror(), b);
    }

    #[test]
    fn test_mirror_involutive() {
        let mut rng = rand::rngs::StdRng::from_seed([5; 32]);
        for _ in 0..500 {
            let pos = rng.next_u64() & ((1u64 << 33) - 1);
            let pos = Position(pos);

            assert_eq!(pos.mirror().mirror(), pos);
        }
    }

    #[test]
    fn test_normalize() {
        let a = Position::from_ascii([
            "    ...    ",
            "    ..#    ",
            "  .##....  ",
            "  ..#....  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ]);
        let b = a.rotate();

        assert_eq!(a.normalize(), b.normalize());

        let b = a.mirror();
        assert_eq!(a.normalize(), b.normalize());
    }

    #[test]
    fn test_solver_returns_valid_sequence_of_jumps() {
        let filter = BloomFilter::always_true();

        let mut pos = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  ..###..  ",
            "  ...#...  ",
            "    .#.    ",
            "    ...    ",
        ]);

        let SolveResult::Solved(jumps) =
            solve_with_bloom_filter(pos, &filter, Direction::Forward, 0).0
        else {
            panic!("should be solvable");
        };
        assert_eq!(jumps.len(), 4);

        for jump in jumps {
            assert!(pos.can_jump(jump));
            pos = pos.apply_jump(jump);
        }

        assert_eq!(pos, Position::default_end());
    }

    #[test]
    fn test_save_and_load_preserves_bloom_filter() {
        let mut filter = BloomFilter::new(13, 1);
        filter.insert(Position(3));
        filter.insert(Position(5));

        let tempdir = tempdir().unwrap();
        let filename = tempdir.path().join("asdf.bin");

        filter.save_to_file(&filename);
        let filter2 = BloomFilter::load_from_file(filename);

        dbg!(&filter.bits.0);
        dbg!(&filter2.bits.0);

        assert!(filter == filter2);

        for i in 0..20 {
            let pos = Position(i);
            assert_eq!(filter.query(pos), filter2.query(pos));
        }
    }
}
