pub mod solvable_positions;

pub fn compress_hash() -> Vec<u8> {
    vec![0, 1, 2, 3, 4, 5]
}

pub fn read_hash(data: &[u8]) -> i32 {
    data.iter().map(|d| Into::<i32>::into(*d)).sum()
}
