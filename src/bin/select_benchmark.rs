use std::fs::File;
use std::io::Write;
use std::ops::Range;
use std::time::Instant;
use chrono::Local;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rand::SeedableRng;
use bit_vector::BitVector;

const POINTS: usize = 25;
const ITERATIONS: usize = 1000000;

fn generate_bit_string(len: usize) -> String {
    let mut data = String::new();
    let mut rng = ChaCha8Rng::seed_from_u64(1234567);
    for _ in 0..len {
        if rng.gen_range(0..=1) == 0 {
            data += "0";
        } else {
            data += "1";
        }
    }
    data
}

fn generate_select_queries(num: usize, range0: Range<usize>, range1: Range<usize>) -> Vec<(bool, usize)> {
    let mut result = Vec::new();
    result.reserve_exact(num);
    let mut rng = ChaCha8Rng::seed_from_u64(1234567);
    for _ in 0..num {
        if rng.gen_range(0..1) == 0 && !range0.is_empty() {
            result.push((false, rng.gen_range(range0.clone())));
        } else {
            result.push((true, rng.gen_range(range1.clone())));
        }
    }
    result
}

fn main() {
    let bit_string = generate_bit_string(1 << POINTS);
    let mut out = format!("% select benchmark {} points: {POINTS} iterations: {ITERATIONS} \nx r\n", Local::now().format("%d/%m/%Y %H:%M"));
    
    for i in 1..POINTS {
        let mut vector = BitVector::load_from_string(&bit_string[..(1 << i)]);
        vector.init_select_structures();
        let ones = vector.count_ones(0..(1 << i));
  
        let commands = generate_select_queries(ITERATIONS, 1..((1 << i) - ones + 1), 1..(ones+1));

        let start = Instant::now();
        for command in commands {
            vector.select(command.0, command.1);
        }
        let end = Instant::now();
        let t = (end - start).as_secs_f64();

        out += &format!("{} {}\n", 1 << i, ITERATIONS as f64 / t);
    }
    let mut file = File::create("./select_benchmark.tex").unwrap();
    file.write_all(out.as_bytes()).unwrap();
}