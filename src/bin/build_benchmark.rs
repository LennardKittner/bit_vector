use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;
use chrono::Local;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rand::SeedableRng;
use bit_vector::BitVector;

const POINTS: usize = 32;
const ITERATIONS: usize = 10;

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

fn main() {
    let bit_string = generate_bit_string(1usize << (POINTS-1));
    let mut out = format!("% build benchmark {} points: {POINTS} iterations: {ITERATIONS}\nbits rankT selectT bothT rankS selectS bothS\n", Local::now().format("%d/%m/%Y %H:%M"));

    for i in 0..POINTS {
        let mut vector = BitVector::load_from_string(&bit_string[..(1usize << i)]);
        let mut rank = 0f64;
        let mut select = 0f64;
        for _ in 0..ITERATIONS {
            let start = Instant::now();
            vector.init_rank_structures();
            let end = Instant::now();
            rank += (end - start).as_secs_f64() / ITERATIONS as f64;

            let start = Instant::now();
            vector.init_select_structures();
            let end = Instant::now();
            select += (end - start).as_secs_f64() / ITERATIONS as f64;
        }
        out += &format!("{} {} {} {} {} {} {} \n", 1usize << i, rank, select, rank + select, vector.get_size_rank(), vector.get_size_select_1() + vector.get_size_select_0(), vector.get_size());
    }
    let mut file = File::create("./build_benchmark.tex").unwrap();
    file.write_all(out.as_bytes()).unwrap();
}