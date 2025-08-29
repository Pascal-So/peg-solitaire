#[cfg(not(target_family = "wasm"))]
use std::path::Path;

use bincode::config;
use bitvec::{bitbox, boxed::BitBox, prelude::Lsb0};
use rand::{seq::SliceRandom, SeedableRng};
use rand_pcg::Pcg64Mcg;

pub const NR_HOLES: usize = 33;
pub const NR_PEGS: usize = 32;

pub type Coord = (i16, i16);

pub fn coordinate_to_index((x, y): Coord) -> Option<i16> {
    match (y, x) {
        (0..=1, 2..=4) => Some((x - 2) + y * 3),
        (2..=4, 0..=6) => Some(x + (y - 2) * 7 + 6),
        (5..=6, 2..=4) => Some((x - 2) + (y - 5) * 3 + 27),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Position(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Jump(u64, u64);

pub const ALL_JUMPS: [Jump; 76] = [
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
    pub fn from_ascii(lines: [&str; 7]) -> Self {
        let mut position = 0;
        let mut counted_chars = 0;
        for line in lines {
            for c in line.chars() {
                (position, counted_chars) = match c {
                    '.' => (position * 2, counted_chars + 1),
                    '#' => (position * 2 + 1, counted_chars + 1),
                    ' ' => (position, counted_chars),
                    _ => panic!("invalid char in ascii"),
                };

                if counted_chars > 33 {
                    panic!("too many chars in ascii");
                }
            }
        }
        if counted_chars < 33 {
            panic!("not enough chars in ascii");
        }
        Self(position)
    }

    pub fn print(&self) {
        let side_space = "  ";

        let pos = self.0;
        let mut mask = 1u64 << 32;
        let mut print_bit = || {
            print!("{}", if pos & mask > 0 { '#' } else { '.' });
            mask /= 2;
        };

        for _ in 0..2 {
            print!("{side_space}");
            for _ in 0..3 {
                print_bit();
            }
            println!("{side_space}");
        }
        for _ in 0..3 {
            for _ in 0..7 {
                print_bit();
            }
            print!("\n");
        }
        for _ in 0..2 {
            print!("{side_space}");
            for _ in 0..3 {
                print_bit();
            }
            println!("{side_space}");
        }
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
        (self.0 & jump.1) == 0 && (self.0 & jump.0).count_ones() == 2
    }
    pub fn can_jump_inverse(&self, jump: Jump) -> bool {
        (self.0 & jump.0) == 0 && (self.0 & jump.1) > 0
    }
    pub fn apply_jump(&self, jump: Jump) -> Position {
        let mut next = self.0;
        next &= !jump.0;
        next |= jump.1;
        Position(next)
    }
    pub fn apply_jump_inverse(&self, jump: Jump) -> Position {
        let mut next = self.0;
        next |= jump.0;
        next &= !jump.1;
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
            if x >= 2 && x <= 4 {
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
}

#[cfg_attr(not(target_family = "wasm"), derive(bincode::Encode))]
#[derive(bincode::Decode)]
pub struct BloomFilter {
    nr_bits: u64, // invariant: this value always fits in u32
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
    pub fn new(nr_bits: u32) -> Self {
        Self {
            nr_bits: nr_bits as u64,
            bits: BincodeBitBox(bitbox![u32, Lsb0; 0; nr_bits as usize]),
        }
    }

    pub fn nr_bits(&self) -> u64 {
        self.nr_bits
    }

    pub fn hash(&self, position: Position) -> u64 {
        position.0 as u64 % self.nr_bits()
    }

    pub fn insert(&mut self, position: Position) {
        let hash = self.hash(position);
        self.bits.0.set(hash as usize, true);
    }

    pub fn query(&self, position: Position) -> bool {
        let hash = self.hash(position);
        *self.bits.0.get(hash as usize).unwrap()
    }

    pub fn load_from_slice(data: &[u8]) -> Self {
        let (filter, _) = bincode::decode_from_slice(data, bincode_config()).unwrap();
        filter
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

    /// Load files that were written in the old format that does not store the
    /// number of bits.
    pub fn load_from_old_file_with_nr_bits(path: impl AsRef<Path>, nr_bits: u32) -> Self {
        #[derive(bincode::Decode, bincode::Encode)]
        struct Old {
            bits: BincodeBitBox,
        }

        let mut file = std::fs::File::open(path).unwrap();
        let old: Old = bincode::decode_from_std_read(&mut file, bincode_config()).unwrap();

        Self {
            nr_bits: nr_bits as u64,
            bits: old.bits,
        }
    }
}

const BYTES_LIMIT_BLOOM_FILTER: usize = 50 * 1024 * 1024;
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

pub enum SolveResult {
    Solved,
    Unsolvable,
    TimedOut,
}

pub fn solve_with_bloom_filter(pos: Position, filter: &BloomFilter) -> SolveResult {
    const STEP_LIMIT: u32 = 120;
    fn inner(
        pos: Position,
        filter: &BloomFilter,
        end: Position,
        nr_steps: &mut u32,
        jumps: &[Jump; 76],
    ) -> SolveResult {
        *nr_steps += 1;
        if *nr_steps > STEP_LIMIT {
            return SolveResult::TimedOut;
        }

        for &jump in jumps {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);
                if next == end {
                    return SolveResult::Solved;
                }

                if next.count() == 1 {
                    continue;
                }

                if !filter.query(next.normalize()) {
                    continue;
                }

                match inner(next, filter, end, nr_steps, jumps) {
                    SolveResult::Solved => return SolveResult::Solved,
                    SolveResult::Unsolvable => {}
                    SolveResult::TimedOut => return SolveResult::TimedOut,
                }
            }
        }

        SolveResult::Unsolvable
    }

    if !filter.query(pos.normalize()) {
        return SolveResult::Unsolvable;
    }

    let mut jumps = ALL_JUMPS;
    let mut rng = Pcg64Mcg::seed_from_u64(0);

    let end = Position::default_end();
    for _ in 0..200 {
        match inner(pos, filter, end, &mut 0, &jumps) {
            SolveResult::Solved => return SolveResult::Solved,
            SolveResult::Unsolvable => return SolveResult::Unsolvable,
            SolveResult::TimedOut => {}
        }

        jumps.shuffle(&mut rng);
    }

    SolveResult::TimedOut
}

#[cfg(test)]
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
    use rand::{RngCore, SeedableRng};
    use tempfile::tempdir;

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
    fn test_from_ascii() {
        let a = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  .......  ",
            "    ...    ",
            "    ..#    ",
        ]);
        assert_eq!(a.0, 1);

        let a = Position::from_ascii([
            "    ...    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  .......  ",
            "    ...    ",
            "    .#.    ",
        ]);
        assert_eq!(a.0, 2);

        let a = Position::from_ascii([
            "    #..    ",
            "    ...    ",
            "  .......  ",
            "  .......  ",
            "  .......  ",
            "    ...    ",
            "    ...    ",
        ]);
        assert_eq!(a.0, 1u64 << 32);
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
    fn test_save_and_load_preserves_bloom_filter() {
        let mut filter = BloomFilter::new(13);
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
