use std::collections::HashMap;

use rayon::prelude::*;

// const DATA: [u64; 28] = [
//     17, 60, 296, 1338, 5648, 21842, 77559, 249690, 717788, 1834379, 4138302, 8171208, 14020166,
//     20773236, 26482824, 28994876, 27286330, 22106348, 15425572, 9274496, 4792664, 2120101, 800152,
//     255544, 14727, 2529, 334, 32,
// ];

const DATA: [u64; 33] = [
    0, 1, 4, 12, 60, 296, 1338, 5648, 21842, 77559, 249690, 717788, 1834379, 4138302, 8171208,
    14020166, 20773236, 26482824, 28994876, 27286330, 22106348, 15425572, 9274496, 4792664,
    2120101, 800152, 255544, 68236, 14727, 2529, 334, 32, 5,
];

fn step(
    buckets: &mut Vec<u64>,
    position: usize,
    mut best_so_far: usize,
    limit: u64,
    current_level_to_bucket_assignments: &mut HashMap<i32, usize>,
) -> Option<(Vec<u64>, HashMap<i32, usize>)> {
    if position == DATA.len() {
        if buckets.len() < best_so_far {
            return Some((buckets.clone(), current_level_to_bucket_assignments.clone()));
        } else {
            return None;
        }
    }

    let val = DATA[position];
    let mut best = None;
    for i in 0..buckets.len() {
        if buckets[i] + val <= limit {
            buckets[i] += val;
            current_level_to_bucket_assignments.insert(position as i32, i);
            let ret = step(
                buckets,
                position + 1,
                best_so_far,
                limit,
                current_level_to_bucket_assignments,
            );
            if let Some(ret) = ret {
                if ret.0.len() < best_so_far {
                    best_so_far = ret.0.len();
                    best = Some(ret);
                }
            }
            buckets[i] -= val;
            current_level_to_bucket_assignments.remove(&(position as i32));
        }
    }

    if buckets.len() + 1 < best_so_far {
        buckets.push(val);
        assert!(val <= limit);
        let ret = step(
            buckets,
            position + 1,
            best_so_far,
            limit,
            current_level_to_bucket_assignments,
        );
        if let Some(ret) = ret {
            if ret.0.len() < best_so_far {
                best = Some(ret);
            }
        }
        buckets.pop();
    }

    best
}

fn find_best_limit_for_n_buckets(buckets: usize) -> u64 {
    let max = *DATA.iter().max().unwrap();
    let sum: u64 = DATA.iter().sum();
    let mut lower = max;
    let mut upper = sum; // 52545445;

    let mut iters = 0;
    while upper - lower > 10 {
        let mid = (upper + lower) / 2;
        let mut v = vec![];
        let mut assignments = HashMap::new();
        let ret = step(&mut v, 3, DATA.len(), mid, &mut assignments);

        if ret.unwrap().0.len() > buckets {
            lower = mid;
        } else {
            upper = mid;
        }

        iters += 1;
        if iters % 1 == 0 {
            println!("{buckets:>2} - step {iters:>7}. range {:>8}", upper - lower);
        }
    }

    println!("{buckets}: {upper}");
    upper
}

fn run_binary_searches() {
    let results: Vec<_> = (4..16)
        .into_par_iter()
        .map(|n| (n, find_best_limit_for_n_buckets(n)))
        .collect();

    println!(
        "1 is achievable in bucket size {}",
        DATA.iter().sum::<u64>()
    );
    for (n, limit) in results {
        println!("{n} is achievable in bucket size {limit}");
    }
}

fn get_map(size_limit: u64) -> HashMap<i32, usize> {
    let mut v = vec![];
    let mut assignments = HashMap::new();
    let ret = step(&mut v, 0, DATA.len(), size_limit, &mut assignments).unwrap();

    println!("length: {}", ret.0.len());
    ret.1
}

fn main() {
    dbg!(get_map(93818156));
    dbg!(get_map(46945815));
}

//  1 is achievable in bucket size 187636299
//  2 is achievable in bucket size 93818156
//  3 is achievable in bucket size 62545452
//  4 is achievable in bucket size 46945815
//  5 is achievable in bucket size 37533894
//  6 is achievable in bucket size 31424634
//  7 is achievable in bucket size 28994881
//  8 is achievable in bucket size 28994881
//  9 is achievable in bucket size 28994881
// 10 is achievable in bucket size 28994881
// 11 is achievable in bucket size 28994881
// 12 is achievable in bucket size 28994881
// 13 is achievable in bucket size 28994881
// 14 is achievable in bucket size 28994881
// 15 is achievable in bucket size 28994881

// get_map(93818156) = {
//     1: 0,
//     2: 0,
//     3: 0,
//     4: 0,
//     5: 0,
//     6: 0,
//     7: 0,
//     8: 0,
//     9: 0,
//     11: 1,
//     12: 0,
//     13: 1,
//     14: 1,
//     15: 0,
//     16: 1,
//     17: 0,
//     18: 0,
//     19: 1,
//     20: 0,
//     21: 1,
//     22: 1,
//     23: 1,
//     24: 1,
//     25: 1,
//     26: 0,
//     27: 1,
//     28: 0,
//     29: 0,
//     30: 1,
//     31: 1,
//     32: 1,

// get_map(46945815) = {
//     1: 0,
//     2: 0,
//     3: 0,
//     4: 0,
//     5: 0,
//     6: 0,
//     7: 0,
//     8: 0,
//     9: 0,
//     11: 0,
//     13: 0,
//     14: 1,
//     16: 2,
//     17: 0,
//     18: 1,
//     19: 3,
//     20: 2,
//     21: 0,
//     22: 1,
//     23: 3,
//     24: 2,
//     25: 3,
//     26: 1,
//     27: 0,
//     28: 2,
//     29: 0,
//     30: 0,
//     31: 0,
//     32: 0,
