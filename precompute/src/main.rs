#![allow(dead_code)]

use std::{collections::HashMap, ops::Range, path::PathBuf, time::Instant};

use primal::Primes;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use common::{
    BloomFilter, Jump, Position, all_jumps, debruijn::de_bruijn_solvable, solve_with_bloom_filter,
};
use precompute::VisitMap;

fn build_bloom_filter(size: u32, solvability_map: &VisitMap, k: u32) -> BloomFilter {
    let start = Instant::now();
    let filename = PathBuf::from(format!("filters/filter_{size:0>9}_{k}_norm.bin"));
    if filename.is_file() {
        let filter = BloomFilter::load_from_file(filename);
        println!("loaded filter {size} in {}s", start.elapsed().as_secs_f32());
        return filter;
    }

    let mut filter = BloomFilter::new(size, k);
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
struct CandidateSpec {
    size: u32,
    k: u32,
    size_category: String,
}

#[derive(Serialize)]
struct SizeStats {
    #[serde(flatten)]
    candidate: CandidateSpec,
    false_positives: u64,
    false_positives_in_one_past: u64,
}

/// Count the number of false positives through exhaustive enumeration of the
/// entire input space.
fn evaluate_false_positives(
    solvability_map: &VisitMap,
    one_past_map: &VisitMap,
    filters: Vec<(&BloomFilter, CandidateSpec)>,
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

    fn step(visit_map: &mut VisitMap, pos: Position, total_visited: &mut u64, jumps: &[Jump; 76]) {
        for &jump in jumps {
            if pos.can_jump_inverse(jump) {
                let next = pos.apply_jump_inverse(jump);
                if visit_map.is_visited(next) {
                    continue;
                }
                visit_map.visit(next);
                *total_visited += 1;
                if next.count() < Position::default_start().count() {
                    step(visit_map, next, total_visited, jumps);
                }
            }
        }
    }

    let start = Position::default_end();
    solvability_map.visit(start);
    total_visited += 1;

    step(
        &mut solvability_map,
        start,
        &mut total_visited,
        &all_jumps(),
    );

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

    let jumps = all_jumps();

    for (pos, b) in solvability_map.iter().enumerate() {
        if !b {
            continue;
        }
        let pos = Position(pos as u64);

        if pos.count() <= 1 {
            continue;
        }

        for jump in jumps {
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
    // factor chosen such that we get a similar density in the prime number
    // candidates and in the round numbers.
    let min_factor = 1.1939;

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

#[derive(Serialize)]
struct SolverStats {
    max_steps: u64,
    total_steps: u64,
    nr_samples: u64,
    nr_timeouts: u64,
}

fn evaluate_solver_stats(filter: &BloomFilter, start_positions: &[Position]) -> SolverStats {
    let start_time = Instant::now();
    let mut total_steps = 0;
    let mut max_steps = 0;
    let mut nr_timeouts = 0;
    let nr_samples = 1.max(10000 / start_positions.len() as u64);
    let mut actual_nr_samples = 0;
    for start_pos in start_positions {
        for i in 0..nr_samples {
            let (result, stats) =
                solve_with_bloom_filter(*start_pos, filter, common::Direction::Forward, i);

            if result == common::SolveResult::TimedOut {
                nr_timeouts += 1;
            }
            let steps = stats.nr_steps as u64;
            max_steps = max_steps.max(steps);
            total_steps += steps;
            actual_nr_samples += 1;
        }
    }
    println!(
        "evaluated solver steps {actual_nr_samples} times in {}s",
        start_time.elapsed().as_secs_f32()
    );

    SolverStats {
        max_steps,
        total_steps,
        nr_samples: actual_nr_samples,
        nr_timeouts,
    }
}

/// Generate a list of numbers where the prime factorization moostly consists
/// of factors 2.
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

fn get_candidates_groups() -> [(Vec<u32>, String); 2] {
    let kb = 1024 * 8;
    let mb = 1024 * kb;
    let range = 512 * kb..42 * mb;
    [
        (prime_candidates(range.clone()), "prime".to_string()),
        (round_candidates(range.clone()), "round".to_string()),
        // (
        //     round_minus_one_candidates(range.clone()),
        //     "round_minus_one".to_string(),
        // ),
    ]
}

fn build_data_and_perform_false_positive_evaluation() {
    let solvability_map = build_solvability_map();

    dbg!(count_normalized_solvability(&solvability_map));
    let one_past_map = build_one_past_solvable_map(&solvability_map);

    let mut all_filters: Vec<(BloomFilter, CandidateSpec)> = vec![];

    for (candidate_sizes, category) in get_candidates_groups() {
        let results = candidate_sizes.par_iter().map(|&size| {
            let filter = build_bloom_filter(size, &solvability_map, 1);
            (
                filter,
                CandidateSpec {
                    size,
                    k: 1,
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

fn build_data_and_perform_false_positive_evaluation_for_primes_with_k() {
    let solvability_map = build_solvability_map();

    dbg!(count_normalized_solvability(&solvability_map));
    let one_past_map = build_one_past_solvable_map(&solvability_map);

    let mut all_filters: Vec<(BloomFilter, CandidateSpec)> = vec![];

    let candidates = prime_candidates(512 * 1024 * 8..42 * 1024 * 1024 * 8);
    for k in 1..=4 {
        let results = candidates.par_iter().map(|&size| {
            let filter = build_bloom_filter(size, &solvability_map, k);
            (
                filter,
                CandidateSpec {
                    size,
                    k,
                    size_category: "prime".to_string(),
                },
            )
        });

        all_filters.append(&mut results.collect());
    }

    let start_time = Instant::now();
    let chunks: Vec<_> = all_filters.chunks(8).collect();
    let stats: Vec<_> = chunks
        .par_iter()
        .map(|candidates| {
            let candidates = candidates.iter().map(|(c, s)| (c, s.clone())).collect();
            evaluate_false_positives(&solvability_map, &one_past_map, candidates)
        })
        .collect();

    println!("evaluated stats in {}s", start_time.elapsed().as_secs_f32());
    serde_json::to_writer_pretty(std::fs::File::create("data-primes-k.json").unwrap(), &stats)
        .unwrap();
}

fn count_positive_children(filter: &BloomFilter, pos: Position) -> (u64, u64) {
    let mut positives = 0;
    let mut total = 0;
    for jump in all_jumps() {
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
fn get_random_solvable_start_positions(solvability_map: &VisitMap) -> Vec<Position> {
    let nr_positions = 1 << 16;
    let mut start_positions = Vec::with_capacity(nr_positions);
    let mut rng = Pcg64Mcg::seed_from_u64(123);

    let mut i: usize = 1;
    for (pos, b) in solvability_map.iter().enumerate() {
        if b {
            let pos = Position(pos as u64);

            if pos.count() < 26 {
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

/// Draw a random sample of positions that are not solvable, but are deBruijn
/// solvable, using reservoir sampling.
fn get_random_unsolvable_start_positions(solvability_map: &VisitMap) -> Vec<Position> {
    let nr_positions = 1 << 16;
    let mut start_positions = Vec::with_capacity(nr_positions);
    let mut rng = Pcg64Mcg::seed_from_u64(123);

    let mut i: usize = 1;
    for (pos, b) in solvability_map.iter().enumerate() {
        if !b {
            let pos = Position(pos as u64);
            if pos.count() < 23 {
                continue;
            }

            if !de_bruijn_solvable(pos) {
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

fn analyze_state_space() {
    let solvability_map = build_solvability_map();

    #[derive(Serialize)]
    struct Info {
        solvable_at: Vec<i32>,
        solvable_norm_at: Vec<i32>,
        via_solvable_at: Vec<i32>,
        via_solvable_norm_at: Vec<i32>,
        de_bruijn_solvable_at: Vec<i32>,
        de_bruijn_solvable_norm_at: Vec<i32>,
    }

    let mut info = Info {
        solvable_at: vec![0; 34],
        solvable_norm_at: vec![0; 34],
        via_solvable_at: vec![0; 34],
        via_solvable_norm_at: vec![0; 34],
        de_bruijn_solvable_at: vec![0; 34],
        de_bruijn_solvable_norm_at: vec![0; 34],
    };

    for (pos, b) in solvability_map.iter().enumerate() {
        let pos = Position(pos as u64);
        let is_normalized = pos == pos.normalize();
        let is_de_bruijn_solvable = de_bruijn_solvable(pos);
        let count = pos.count() as usize;

        if b {
            let is_via_solvable = solvability_map.is_visited(pos.inverse());

            info.solvable_at[count] += 1;

            if is_normalized {
                info.solvable_norm_at[count] += 1;
            }
            if is_via_solvable {
                info.via_solvable_at[count] += 1;
            }
            if is_via_solvable && is_normalized {
                info.via_solvable_norm_at[count] += 1;
            }
        }

        if is_de_bruijn_solvable {
            info.de_bruijn_solvable_at[count] += 1;
        }
        if is_de_bruijn_solvable && is_normalized {
            info.de_bruijn_solvable_norm_at[count] += 1;
        }
    }

    serde_json::to_writer_pretty(
        std::fs::File::create("state-space-sizes.json").unwrap(),
        &info,
    )
    .unwrap();
}

fn main() {
    // analyze_state_space();
    // return;
    // let prime_filter = BloomFilter::load_from_file("filters/filter_173378771_norm.bin");
    // evaluate_difficult_positions(&prime_filter);
    // evaluate_various_positions(&BloomFilter::load_from_file(
    //     "filters/filter_083886080_norm.bin",
    // ));

    // return;

    let solvability_map = build_solvability_map();
    let mut solver_stats = vec![];

    let solvable_positions = get_random_solvable_start_positions(&solvability_map);
    let unsolvable_positions = get_random_unsolvable_start_positions(&solvability_map);

    for (candidate_sizes, _group) in get_candidates_groups() {
        let results = candidate_sizes.par_iter().map(|&size| {
            let filter =
                BloomFilter::load_from_file(format!("filters/modulo/filter_{size:0>9}_1_norm.bin"));
            let stats_default_start = evaluate_solver_stats(&filter, &[Position::default_start()]);
            let stats_solvable = evaluate_solver_stats(&filter, &solvable_positions);
            let stats_unsolvable = evaluate_solver_stats(&filter, &unsolvable_positions);
            (stats_default_start, stats_solvable, stats_unsolvable, size)
        });

        // solver_stats.append(&mut results.collect());
        let results: Vec<_> = results.collect();
        for r in results {
            solver_stats.push(serde_json::json!({
                "size": r.3,
                "default_max": r.0.max_steps,
                "default_avg": r.0.total_steps as f64 / r.0.nr_samples as f64,
                "default_completed": 1.0 - (r.0.nr_timeouts as f64 / r.0.nr_samples as f64),
                "solvable_max": r.1.max_steps,
                "solvable_avg": r.1.total_steps as f64 / r.1.nr_samples as f64,
                "solvable_completed": 1.0 - (r.1.nr_timeouts as f64 / r.1.nr_samples as f64),
                "unsolvable_max": r.2.max_steps,
                "unsolvable_avg": r.2.total_steps as f64 / r.2.nr_samples as f64,
                "unsolvable_completed": 1.0 - (r.2.nr_timeouts as f64 / r.2.nr_samples as f64),
            }));
        }
    }

    serde_json::to_writer_pretty(
        std::fs::File::create("solver-stats.json").unwrap(),
        &solver_stats,
    )
    .unwrap();
}
