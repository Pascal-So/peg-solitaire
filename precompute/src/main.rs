#![allow(dead_code)]

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    path::PathBuf,
    time::Instant,
};

use primal::Primes;
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use common::{BloomFilter, Jump, Position, ALL_JUMPS};
use precompute::{
    positions::{get_difficult_positions, get_solvable_positions},
    VisitMap,
};

fn build_bloom_filter(size: u32, solvability_map: &VisitMap) -> BloomFilter {
    let start = Instant::now();
    let filename = PathBuf::from(format!("filter_{size:0>9}_norm.bin"));
    if filename.is_file() {
        let filter = BloomFilter::load_from_file(filename);
        println!("loaded filter {size} in {}s", start.elapsed().as_secs_f32());
        return filter;
    }

    let mut filter = BloomFilter::new(size);
    for (pos, b) in solvability_map.iter().enumerate() {
        if b {
            let pos = Position(pos as u64).normalize();
            filter.insert(pos);
        }
    }

    filter.save_to_file(filename);
    println!("built filter {size} in {}s", start.elapsed().as_secs_f32());
    filter
}

#[derive(Serialize, Clone)]
struct CandidateSize {
    size: u32,
    size_category: String,
}

#[derive(Serialize)]
struct SizeStats {
    #[serde(flatten)]
    candidate: CandidateSize,
    false_positives: u64,
    false_positives_in_one_past: u64,
}

/// Count the number of false positives through exhaustive enumeration of the
/// entire input space.
fn evaluate_false_positives(
    solvability_map: &VisitMap,
    one_past_map: &VisitMap,
    filters: Vec<(&BloomFilter, CandidateSize)>,
) -> Vec<SizeStats> {
    let start = Instant::now();
    let mut total_negatives: u64 = 0;
    let mut total_one_past: u64 = 0;

    let mut stats: Vec<_> = filters
        .into_iter()
        .map(|(filter, candidate)| {
            (
                filter,
                SizeStats {
                    candidate,
                    false_positives: 0,
                    false_positives_in_one_past: 0,
                },
            )
        })
        .collect();

    let mut false_negatives: HashMap<u32, i32> = HashMap::new();
    for (pos, b) in solvability_map.iter().enumerate() {
        let pos = Position(pos as u64);
        let pos_normalized = pos.normalize();

        #[derive(Copy, Clone, PartialEq, Eq)]
        enum Case {
            OnPath,
            OnePastPath,
            OffPath,
        }

        let case = if b {
            Case::OnPath
        } else {
            total_negatives += 1;
            if one_past_map.is_visited(pos) {
                total_one_past += 1;
                Case::OnePastPath
            } else {
                Case::OffPath
            }
        };

        for (filter, stats) in &mut stats {
            let q = filter.query(pos_normalized);

            match (case, q) {
                (Case::OnPath, false) => {
                    *false_negatives.entry(stats.candidate.size).or_default() += 1;
                    // println!(
                    //     "false negative for filter {}, pos {pos_normalized:?} ({pos:?})",
                    //     stats.candidate.size
                    // );
                }
                (Case::OnePastPath, true) => {
                    stats.false_positives_in_one_past += 1;
                    stats.false_positives += 1;
                }
                (Case::OffPath, true) => {
                    stats.false_positives += 1;
                }
                _ => {}
            }
        }
    }

    dbg!(false_negatives);

    // 42m:      3005572974/8402298294 = 0.357708435
    // 32m:      3278797910/8402298294 = 0.390226316
    // 42m_norm:  701029794/8402298294 = 0.0834331
    // 32m_norm: 1124432841/8402298294 = 0.133824437
    // 10m_norm: 2275692739/8402298294 = 0.270841698
    //  8m_norm: 2863052278/8402298294 = 0.340746327
    //  6m_norm: 3264923608/8402298294 = 0.388575065
    //  4m_norm: 4062083420/8402298294 = 0.483449085
    //

    println!("total negatives: {total_negatives}");
    println!("total one past: {total_one_past}");
    println!(
        "evaluated stats for {} filters in {}s",
        stats.len(),
        start.elapsed().as_secs_f32()
    );
    stats.into_iter().map(|(_, stats)| stats).collect()
}

/// Count the number of unique solvable positions, modulo normalization.
fn count_normalized_solvability(solvability_map: &VisitMap) -> u64 {
    let mut map = VisitMap::new();
    let mut count = 0;

    for (pos, b) in solvability_map.iter().enumerate() {
        if !b {
            continue;
        }
        let pos = Position(pos as u64).normalize();

        if !map.is_visited(pos) {
            count += 1;
            map.visit(pos);
        }
    }

    count
}

/// Build a list of all solvable positions, i.e. positions that can reach the
/// default end position.
fn build_solvability_map() -> VisitMap {
    let start_time = Instant::now();

    let filename = PathBuf::from("solvability_map.bin");
    if filename.is_file() {
        let map = VisitMap::load_from_file(filename);
        println!(
            "loaded solvability map in {}s",
            start_time.elapsed().as_secs_f32()
        );
        return map;
    }

    let mut solvability_map = VisitMap::new();
    let mut total_visited: u64 = 0;

    fn step(visit_map: &mut VisitMap, pos: Position, total_visited: &mut u64) {
        for jump in ALL_JUMPS {
            if pos.can_jump_inverse(jump) {
                let next = pos.apply_jump_inverse(jump);
                if visit_map.is_visited(next) {
                    continue;
                }
                visit_map.visit(next);
                *total_visited += 1;
                if next.count() < Position::default_start().count() {
                    step(visit_map, next, total_visited);
                }
            }
        }
    }

    let start = Position::default_end();
    solvability_map.visit(start);
    total_visited += 1;

    step(&mut solvability_map, start, &mut total_visited);

    println!("Built solvability map. Total solvable positions: {total_visited}");

    solvability_map.save_to_file(filename);
    println!(
        "built solvability map in {}s",
        start_time.elapsed().as_secs_f32()
    );
    solvability_map
}

/// Build a list of all positions that are reachable within one step from any
/// solvable position. They're at most one move off the correct path.
fn build_one_past_solvable_map(solvability_map: &VisitMap) -> VisitMap {
    let start_time = Instant::now();

    let filename = PathBuf::from("one_past_map.bin");
    if filename.is_file() {
        let map = VisitMap::load_from_file(filename);
        println!(
            "loaded one_past_solvable map in {}s",
            start_time.elapsed().as_secs_f32()
        );
        return map;
    }

    let mut one_past_map = VisitMap::new();

    for (pos, b) in solvability_map.iter().enumerate() {
        if !b {
            continue;
        }
        let pos = Position(pos as u64);

        if pos.count() <= 1 {
            continue;
        }

        for jump in ALL_JUMPS {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);
                one_past_map.visit(next);
            }
        }
    }

    one_past_map.save_to_file(filename);
    println!(
        "built one_past_solvable map in {}s",
        start_time.elapsed().as_secs_f32()
    );
    one_past_map
}

fn prime_candidates(range: Range<u32>) -> Vec<u32> {
    let mut previous_value = 0;
    let min_factor = 1.1939; // factor chosen such that we get 26 candidates

    let mut candidates = vec![];

    let primes = Primes::all()
        .map(|p| p as u32)
        .skip_while(|p| !range.contains(p))
        .take_while(|p| range.contains(p));

    for p in primes {
        if previous_value as f64 * min_factor <= p as f64 {
            candidates.push(p);
            previous_value = p;
        }
    }

    candidates
}

fn evaluate_solver_steps(filter: &BloomFilter) -> Vec<u64> {
    let mut steps = vec![];

    #[derive(Eq, PartialEq, Debug)]
    enum Result {
        Solved,
        NotSolved,
        TimedOut,
        Suspended(usize),
    }

    fn step_inner_reshuffle(
        filter: &BloomFilter,
        pos: Position,
        nr_steps: &mut u64,
        end: Position,
        rng: &mut impl Rng,
        limit: u64,
    ) -> Result {
        if *nr_steps >= limit {
            return Result::TimedOut;
        }
        *nr_steps += 1;

        let mut jumps = ALL_JUMPS;
        jumps.shuffle(rng);

        for jump in jumps {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);
                if next == end {
                    return Result::Solved;
                }
                if filter.query(next.normalize()) {
                    match step_inner_reshuffle(filter, next, nr_steps, end, rng, limit) {
                        Result::Solved => return Result::Solved,
                        Result::NotSolved => {}
                        Result::TimedOut => return Result::TimedOut,
                        Result::Suspended(_) => unreachable!(),
                    }
                }
            }
        }

        Result::NotSolved
    }

    fn step_inner_preshuffled(
        filter: &BloomFilter,
        pos: Position,
        nr_steps: &mut u64,
        end: Position,
        jumps: &[Jump; 76],
        limit: u64,
    ) -> Result {
        if *nr_steps >= limit {
            return Result::TimedOut;
        }
        *nr_steps += 1;

        for &jump in jumps {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);
                if next == end {
                    return Result::Solved;
                }
                if filter.query(next.normalize()) {
                    match step_inner_preshuffled(filter, next, nr_steps, end, jumps, limit) {
                        Result::Solved => return Result::Solved,
                        Result::NotSolved => {}
                        Result::TimedOut => return Result::TimedOut,
                        Result::Suspended(_) => unreachable!(),
                    }
                }
            }
        }

        Result::NotSolved
    }

    fn step_inner_with_suspense(
        filter: &BloomFilter,
        pos: Position,
        nr_steps: &mut u64,
        end: Position,
        jumps: &[Jump; 76],
        limit: u64,
    ) -> Result {
        if *nr_steps >= limit {
            return Result::TimedOut;
        }
        *nr_steps += 1;

        let mut suspended = vec![];

        for (idx, &jump) in jumps.iter().enumerate() {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);
                if next == end {
                    return Result::Solved;
                }
                if filter.query(next.normalize()) {
                    match step_inner_with_suspense(filter, next, nr_steps, end, jumps, limit) {
                        Result::Solved => return Result::Solved,
                        Result::NotSolved => {
                            if (next.0 + *nr_steps) % 193 <= 1 {
                                return Result::Suspended(idx);
                            }
                        }
                        Result::TimedOut => return Result::TimedOut,
                        Result::Suspended(idx) => {
                            suspended.push(idx);
                        }
                    }
                }
            }
        }

        // todo: iterate over suspended states

        Result::NotSolved
    }

    fn step_inner_with_cache(
        filter: &BloomFilter,
        pos: Position,
        nr_steps: &mut u64,
        end: Position,
        jumps: &[Jump; 76],
        limit: u64,
        cache: &mut HashSet<Position>,
    ) -> Result {
        if *nr_steps >= limit {
            return Result::TimedOut;
        }
        *nr_steps += 1;

        let mut has_descended = false;

        for &jump in jumps.iter() {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);
                if next == end {
                    return Result::Solved;
                }
                if filter.query(next.normalize()) && !cache.contains(&next.normalize()) {
                    has_descended = true;
                    match step_inner_with_cache(filter, next, nr_steps, end, jumps, limit, cache) {
                        Result::Solved => return Result::Solved,
                        Result::NotSolved => {}
                        Result::TimedOut => return Result::TimedOut,
                        Result::Suspended(_) => unreachable!(),
                    }
                }
            }
        }

        if has_descended {
            cache.insert(pos.normalize());
        }

        Result::NotSolved
    }

    let mut largest_cache: usize = 0;
    let start_time = Instant::now();
    let nr_samples = 100;
    for i in 0..nr_samples {
        let mut rng = Pcg64Mcg::seed_from_u64(i);

        let mut nr_steps = 0;
        let pos = Position::default_start();
        let end = Position::default_end();

        let mut jumps = ALL_JUMPS;
        jumps.shuffle(&mut rng);

        let nr_attempts = 5;
        for attempt in 0..nr_attempts {
            let last_attempt = attempt == nr_attempts - 1;

            let mut cache = HashSet::new();
            let solved = step_inner_with_cache(
                filter,
                pos,
                &mut nr_steps,
                end,
                &jumps,
                if last_attempt { 1 << 17 } else { 60 },
                &mut cache,
            );
            match solved {
                Result::Solved => {
                    largest_cache = largest_cache.max(cache.len());
                }
                Result::NotSolved => panic!("puzzle was not solved using bloom filter"),
                Result::TimedOut => {
                    jumps.shuffle(&mut rng);
                }
                Result::Suspended(_) => {
                    println!("puzzle was suspended");
                    jumps.shuffle(&mut rng);
                }
            }
        }

        steps.push(nr_steps);
    }

    dbg!(largest_cache);

    println!(
        "evaluated solver steps {nr_samples} times in {}s",
        start_time.elapsed().as_secs_f32()
    );

    steps
}

fn round_candidates(range: Range<u32>) -> Vec<u32> {
    let mut candidates = vec![];

    'outer: for exponent in 1.. {
        for factor in 4..8 {
            let number = (1u32 << exponent) * factor;

            if number < range.start {
                continue 'outer;
            }
            if !range.contains(&number) {
                break 'outer;
            }

            candidates.push(number);
        }
    }

    candidates
}

fn round_minus_one_candidates(range: Range<u32>) -> Vec<u32> {
    let mut candidates = round_candidates(range);
    for c in &mut candidates {
        *c -= 1;
    }
    candidates
}

fn get_candidates_groups() -> [(Vec<u32>, String); 3] {
    // 512KB to 42MB
    let range = 512 * 1024 * 8..42 * 1024 * 1024 * 8;
    let candidates = [
        (prime_candidates(range.clone()), "prime".to_string()),
        (round_candidates(range.clone()), "round".to_string()),
        (
            round_minus_one_candidates(range.clone()),
            "round_minus_one".to_string(),
        ),
    ];

    candidates
}

fn build_data_and_perform_false_positive_evaluation() {
    let solvability_map = build_solvability_map();

    dbg!(count_normalized_solvability(&solvability_map));
    let one_past_map = build_one_past_solvable_map(&solvability_map);

    let mut all_filters: Vec<(BloomFilter, CandidateSize)> = vec![];

    for (candidate_sizes, category) in get_candidates_groups() {
        let results = candidate_sizes.par_iter().map(|&size| {
            let filter = build_bloom_filter(size, &solvability_map);
            (
                filter,
                CandidateSize {
                    size,
                    size_category: category.clone(),
                },
            )
        });

        all_filters.append(&mut results.collect());
    }

    let start_time = Instant::now();
    let chunks: Vec<_> = all_filters.chunks(6).collect();
    let stats: Vec<_> = chunks
        .par_iter()
        .map(|candidates| {
            let candidates = candidates.iter().map(|(c, s)| (c, s.clone())).collect();
            evaluate_false_positives(&solvability_map, &one_past_map, candidates)
        })
        .collect();

    println!("evaluated stats in {}s", start_time.elapsed().as_secs_f32());
    serde_json::to_writer_pretty(std::fs::File::create("data-MBrange.json").unwrap(), &stats)
        .unwrap();
}

fn evaluate_various_positions(filter: &BloomFilter) {
    let mut jumps = ALL_JUMPS;
    let mut rng = Pcg64Mcg::seed_from_u64(0);

    let mut start_positions = get_solvable_positions();
    start_positions.push(Position::default_start());
    let end_pos = Position::default_end();

    #[derive(Eq, PartialEq, Debug)]
    enum Result {
        TimeOut,
        Solved,
        NotSolved,
    }

    fn step(
        filter: &BloomFilter,
        pos: Position,
        nr_steps: &mut u64,
        end: Position,
        jumps: &[Jump; 76],
        cache: &mut HashSet<Position>,
        limit: u64,
    ) -> Result {
        *nr_steps += 1;

        if *nr_steps > limit {
            return Result::TimeOut;
        }

        for &jump in jumps {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);

                if next == end {
                    return Result::Solved;
                }

                if !filter.query(next.normalize()) || cache.contains(&next.normalize()) {
                    continue;
                }

                let result = step(filter, next, nr_steps, end, jumps, cache, limit);
                match result {
                    Result::TimeOut => return Result::TimeOut,
                    Result::Solved => return Result::Solved,
                    Result::NotSolved => {}
                }
            }
        }

        // if (pos.0 + *nr_steps * 11) % 17 < 2 {
        cache.insert(pos.normalize());
        // }

        Result::NotSolved
    }

    let mut step_counts = vec![];

    let limit = 1600;
    let limit_per_attempt = 150;

    for _ in 0..1000 {
        for _ in 0..3 {
            jumps.shuffle(&mut rng);
        }

        for start_pos in start_positions.clone() {
            let mut nr_steps;
            let mut attempt = 0;

            loop {
                attempt += 1;
                jumps.shuffle(&mut rng);

                nr_steps = 0;

                let mut cache = HashSet::new();
                let solved = step(
                    filter,
                    start_pos,
                    &mut nr_steps,
                    end_pos,
                    &jumps,
                    &mut cache,
                    limit_per_attempt,
                );

                match solved {
                    Result::Solved => {
                        break;
                    }
                    Result::NotSolved => panic!("puzzle was not solved using bloom filter"),
                    Result::TimeOut => {}
                }
            }

            step_counts.push(nr_steps);
            if attempt * limit_per_attempt > limit {
                dbg!(attempt);
            }
        }
        // println!("{step_counts:?}");
    }
}

fn evaluate_difficult_positions(filter: &BloomFilter) {
    let mut jumps = ALL_JUMPS;
    let mut rng = Pcg64Mcg::seed_from_u64(0);

    let mut start_positions = get_difficult_positions();
    start_positions.push(Position::default_start());
    let end_pos = Position::default_end();

    #[derive(Eq, PartialEq, Debug)]
    enum Result {
        TimeOut,
        Solved,
        NotSolved,
    }

    fn step(
        filter: &BloomFilter,
        pos: Position,
        nr_steps: &mut u64,
        end: Position,
        jumps: &[Jump; 76],
        cache: &mut HashSet<Position>,
        limit: u64,
    ) -> Result {
        *nr_steps += 1;

        if *nr_steps > limit {
            return Result::TimeOut;
        }

        for &jump in jumps {
            if pos.can_jump(jump) {
                let next = pos.apply_jump(jump);

                if next == end {
                    return Result::Solved;
                }

                if !filter.query(next.normalize()) {
                    continue;
                }

                let result = step(filter, next, nr_steps, end, jumps, cache, limit);
                match result {
                    Result::TimeOut => return Result::TimeOut,
                    Result::Solved => return Result::Solved,
                    Result::NotSolved => {}
                }
            }
        }

        // if (pos.0 + *nr_steps * 11) % 17 < 2 {
        // cache.insert(pos.normalize());
        // }

        Result::NotSolved
    }

    let limit = 2000;
    let limit_per_attempt = 200;

    for start_pos in start_positions.clone() {
        start_pos.print();

        let mut nr_solved = 0;
        let mut nr_unsolved = 0;
        let mut nr_timed_out = 0;

        for _ in 0..1000 {
            jumps.shuffle(&mut rng);
            let mut nr_steps;

            let mut attempt = 0;
            loop {
                attempt += 1;

                jumps.shuffle(&mut rng);

                nr_steps = 0;

                let mut cache = HashSet::new();
                let solved = step(
                    filter,
                    start_pos,
                    &mut nr_steps,
                    end_pos,
                    &jumps,
                    &mut cache,
                    limit_per_attempt,
                );

                match solved {
                    Result::Solved => {
                        nr_solved += 1;
                        break;
                    }
                    Result::NotSolved => {
                        nr_unsolved += 1;
                        break;
                    }
                    Result::TimeOut => {}
                }

                if attempt * limit_per_attempt > limit {
                    nr_timed_out += 1;
                    break;
                }
            }
        }

        println!("                  y: {nr_solved:5}, n: {nr_unsolved:5}, ?: {nr_timed_out:5}. positive children: {:?}", count_positive_children(filter, start_pos));
    }
}

fn count_positive_children(filter: &BloomFilter, pos: Position) -> (u64, u64) {
    let mut positives = 0;
    let mut total = 0;
    for jump in ALL_JUMPS {
        if pos.can_jump(jump) {
            total += 1;
            let next = pos.apply_jump(jump);
            if filter.query(next.normalize()) {
                positives += 1;
            }
        }
    }

    (positives, total)
}

/// Draw a random sample of solvable positions using reservoir sampling.
fn get_random_start_positions(solvability_map: &VisitMap) -> Vec<Position> {
    let nr_positions = 128;
    let mut start_positions = Vec::with_capacity(nr_positions);
    let mut rng = Pcg64Mcg::seed_from_u64(123);

    let mut i: usize = 1;
    for (pos, b) in solvability_map.iter().enumerate() {
        if b {
            let pos = Position(pos as u64);

            if pos.count() < 27 {
                continue;
            }

            if start_positions.len() < nr_positions {
                start_positions.push(pos);
            } else {
                let j = rng.random_range(..i);
                if j < nr_positions {
                    start_positions[j] = pos;
                }
            }
            i += 1;
        }
    }

    start_positions
}

fn main() {
    let prime_filter = BloomFilter::load_from_file("filters/filter_173378771_norm.bin");
    evaluate_difficult_positions(&prime_filter);
    // evaluate_various_positions(&BloomFilter::load_from_file(
    //     "filters/filter_083886080_norm.bin",
    // ));

    return;

    let mut solver_steps = vec![];

    for (candidate_sizes, _) in get_candidates_groups() {
        let results = candidate_sizes.par_iter().map(|&size| {
            let filter = BloomFilter::load_from_file(format!("filter_{size:0>9}_norm.bin"));
            let steps = evaluate_solver_steps(&filter);
            (steps, size)
        });

        solver_steps.append(&mut results.collect());
    }

    serde_json::to_writer_pretty(
        std::fs::File::create("solver-steps-preshuffled-5-attempts-cache.json").unwrap(),
        &solver_steps,
    )
    .unwrap();
}
