use std::env::args;
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use bit_vector::BitVector;
use crate::Command::{ACCESS, RANK, SELECT};

const NAME :&str = "Lennard_Kittner";

#[derive(Debug)]
enum Command {
    ACCESS{index: usize},
    RANK{bit: bool, index: usize},
    SELECT{bit: bool, index: usize},
}

impl Command {
    fn from_string(string: &str) -> Self {
        let input: Vec<&str> = string.split_whitespace().collect();
        match input.as_slice() {
            ["access", i] => ACCESS {index: i.parse().expect("Invalid access command parameter")},
            ["rank", b, i] => RANK {bit: b == &"1", index: i.parse().expect("Invalid rank command parameter")},
            ["select", b, i] => SELECT {bit: b == &"1", index: i.parse().expect("Invalid select command parameter")},
            _ => panic!("Invalid command or parameter: {}", input.join(" "))
        }
    }
}

fn main() {
    let args :Vec<String> = args().collect();
    let path_in = &args[1];
    let path_out = &args[2];

    let (bit_vector, commands) = parse_input(path_in);

    bit_vector.init();

    let mut results = Vec::new();

    for command in commands {
        let mut time = Duration::new(0, 0);
        let mut space = 0;
        results.push(match command {
            ACCESS{index} => {
                let start_time = Instant::now();
                let result = bit_vector.access(index);
                let end_time = Instant::now();
                time = end_time - start_time;
                if result { 1 } else { 0 } },
            RANK{bit , index} => {
                let start_time = Instant::now();
                let result = bit_vector.rank(bit, index);
                let end_time = Instant::now();
                time = end_time - start_time;
                result
            },
            SELECT{bit, index} => {
                let start_time = Instant::now();
                let result = bit_vector.select(bit, index);
                let end_time = Instant::now();
                time = end_time - start_time;
                result
            },
        }.to_string());
        println!("RESULT name={NAME} time={time} space={space}")
    }

    let mut file_out = File::create(path_out).unwrap();
    file_out.write_all(&results.as_bytes()).expect("Failed to write output file");
}

fn parse_input(path_in: &str) -> (BitVector, Vec<Command>) {
    let mut file_in = File::open(path_in).unwrap();
    let mut content = String::new();
    file_in.read_to_string(&mut content).expect("Failed to read input file");
    let input_lines: Vec<&str> = content.lines().collect();

    let num_commands = input_lines[0].parse::<usize>().expect("First input line not a number");
    let bit_vector = BitVector::load_from_string(input_lines[1]);
    let commands: Vec<Command> = input_lines[2..].iter().map(|s| *s).map(Command::from_string).collect();
    if num_commands != commands.len() {
        panic!("N and the number of commands differ");
    }
    (bit_vector, commands)
}
