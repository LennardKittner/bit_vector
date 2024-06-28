use std::fs::File;
use std::io::Write;
use std::ops::Range;
use std::time::Instant;
use chrono::Local;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rand::SeedableRng;
use bit_vector::BitVector;

const ITERATIONS: usize = 25;
const NUMBER_OPERATIONS: usize = 100000;

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

fn generate_rank_queries(num: usize, range: Range<usize>) -> Vec<(bool, usize)> {
    let mut result = Vec::new();
    result.reserve_exact(num);
    let mut rng = ChaCha8Rng::seed_from_u64(1234567);
    for _ in 0..num {
        result.push((rng.gen_range(0..=1) == 0, rng.gen_range(range.clone())));
    }
    result
}

fn main() {
    let bit_string = generate_bit_string(1 << ITERATIONS);
    let mut out = format!("% rank benchmark {} iterations: {ITERATIONS} number operations: {NUMBER_OPERATIONS} \nx r\n", Local::now().format("%d/%m/%Y %H:%M"));
    
    for i in 0..ITERATIONS {
        let mut vector = BitVector::load_from_string(&bit_string[..(1 << i)]);
        vector.init_rank_structures();
        let commands = generate_rank_queries(NUMBER_OPERATIONS, 0..(1 << i));

        let start = Instant::now();
        for command in commands {
            vector.rank(command.0, command.1);
        }
        let end = Instant::now();
        let t = (end - start).as_secs_f64();

        out += &format!("{} {}\n", 1 << i, NUMBER_OPERATIONS as f64 / t);
    }
    let mut file = File::create("./rank_benchmark.tex").unwrap();
    file.write_all(out.as_bytes()).unwrap();
}