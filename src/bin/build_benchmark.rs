use std::fs::File;
use std::io::Write;
use std::time::Instant;
use chrono::Local;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rand::SeedableRng;
use bit_vector::BitVector;

const ITERATIONS :usize = 25;

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

fn main() {
    let bit_string = generate_bit_string(1 << ITERATIONS);
    let mut out = format!("% build benchmark {}\nx r s b\n", Local::now().format("%d/%m/%Y %H:%M"));
    
    for i in 0..ITERATIONS {
        let mut vector = BitVector::load_from_string(&bit_string[..(1 << i)]);

        let start = Instant::now();
        vector.init_rank_structures();
        let end = Instant::now();
        let rank = (end - start).as_millis();

        let start = Instant::now();
        vector.init_select_structures();
        let end = Instant::now();
        let select = (end - start).as_millis();
        
        out += &format!("{} {} {} {}\n", 1 << i, rank, select, rank + select);
    }
    let mut file = File::create("./build_benchmark.tex").unwrap();
    file.write_all(out.as_bytes()).unwrap();
}