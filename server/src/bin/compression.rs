fn main() {
    let options = zopfli::Options::default();
    let mut out = std::fs::File::create("prime-compressed.bin").unwrap();
    zopfli::compress(
        options,
        zopfli::Format::Deflate,
        include_bytes!("../../filters/filter_173378771_norm.bin").as_slice(),
        &mut out,
    )
    .unwrap();

    // zopfli::Options {
    //     iteration_count: todo!(),
    //     iterations_without_improvement: todo!(),
    //     maximum_block_splits: todo!(),
    // }
}
