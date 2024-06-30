use std::fs::File;
use std::io::{Read, Write};
use std::ops::Range;
use std::path::Path;
use std::time::Instant;
use chrono::Local;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rand::SeedableRng;
use bit_vector::BitVector;

const POINTS: usize = 32;
const ITERATIONS: usize = 1000000;

fn generate_bit_string(len: usize) -> String {
    let cache_path = Path::new("bit_vector.cache");
    if cache_path.exists() {
        let mut cache = File::open("bit_vector.cache").unwrap();
        let mut content = String::new();
        let cache_len = cache.read_to_string(&mut content).unwrap();
        if len == cache_len {
            return content;
        }
    }
    let mut data = String::new();
    let mut rng = ChaCha8Rng::seed_from_u64(1234567);
    for _ in 0..len {
        if rng.gen_range(0..=1) == 0 {
            data += "0";
        } else {
            data += "1";
        }
    }
    let mut cache = File::create("bit_vector.cache").unwrap();
    cache.write_all(data.as_bytes()).unwrap();
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
    let bit_string = generate_bit_string(1usize << POINTS);
    let mut out = format!("% rank benchmark {} points: {POINTS} iterations: {ITERATIONS}\nx r\n", Local::now().format("%d/%m/%Y %H:%M"));
    
    for i in 0..POINTS {
        let mut vector = BitVector::load_from_string(&bit_string[..(1usize << i)]);
        vector.init_rank_structures();
        let commands = generate_rank_queries(ITERATIONS, 0..(1usize << i));

        let start = Instant::now();
        for command in commands {
            vector.rank(command.0, command.1);
        }
        let end = Instant::now();
        let t = (end - start).as_secs_f64();

        out += &format!("{} {}\n", 1usize << i, ITERATIONS as f64 / t);
    }
    let mut file = File::create("./rank_benchmark.tex").unwrap();
    file.write_all(out.as_bytes()).unwrap();
}