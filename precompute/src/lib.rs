pub mod positions;

use std::path::Path;

use bincode::config;
use bitvec::{bitbox, order::Lsb0};
use common::{BincodeBitBox, Position};

const BYTES_LIMIT: usize = (1usize << 33) / 8 + 1024;

fn bincode_config(
) -> config::Configuration<config::LittleEndian, config::Fixint, config::Limit<BYTES_LIMIT>> {
    config::Configuration::default()
}

pub struct VisitMap {
    bits: BincodeBitBox,
}

impl VisitMap {
    pub fn new() -> Self {
        Self {
            bits: BincodeBitBox(bitbox![u32, Lsb0; 0; 1usize << 33]),
        }
    }

    pub fn visit(&mut self, position: Position) {
        self.bits.0.set(position.0 as usize, true);
    }

    pub fn unvisit(&mut self, position: Position) {
        self.bits.0.set(position.0 as usize, false);
    }

    pub fn is_visited(&self, position: Position) -> bool {
        self.bits.0[position.0 as usize]
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) {
        let mut file = std::fs::File::create(path).unwrap();
        bincode::encode_into_std_write(&self.bits, &mut file, bincode_config()).unwrap();
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Self {
        let mut file = std::fs::File::open(path).unwrap();
        Self {
            bits: bincode::decode_from_std_read(&mut file, bincode_config()).unwrap(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + use<'_> {
        self.bits.0.iter().by_vals()
    }
}
