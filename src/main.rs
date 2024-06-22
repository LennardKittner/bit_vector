use std::env::args;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Instant;
use bit_vector::BitVector;
use crate::Command::{Access, Rank, Select};

const NAME :&str = "Lennard_Kittner";

/// A command parsed from the input file
#[derive(Debug)]
enum Command {
    Access{index: usize},
    Rank{bit: bool, index: usize},
    Select{bit: bool, index: usize},
}

impl Command {
    
    /// Creates a command form a string
    /// * `string` input string
    fn from_string(string: &str) -> Self {
        let input: Vec<&str> = string.split_whitespace().collect();
        match input.as_slice() {
            ["access", i] => Access {index: i.parse().expect("Invalid access command parameter")},
            ["rank", b, i] => Rank {bit: b == &"1", index: i.parse().expect("Invalid rank command parameter")},
            ["select", b, i] => Select {bit: b == &"1", index: i.parse().expect("Invalid select command parameter")},
            _ => panic!("Invalid command or parameter: {}", input.join(" "))
        }
    }
}

/// Main executable takes an input and output path as commandline arguments.
fn main() {
    // read input
    let args :Vec<String> = args().collect();
    if args.len() < 3 {
        eprintln!("Please provide an input and output path.");
        return;
    }
    let path_in = &args[1];
    let path_out = &args[2];
    
    let (mut bit_vector, commands) = parse_input(path_in);
    
    // start the timer
    let start_time = Instant::now();
    // generate the acceleration structures
    bit_vector.init();

    let mut results = Vec::new();
    
    // execute the commands
    for command in commands {
        results.push(match command {
            Access {index} => bit_vector.access(index),
            Rank {bit , index} => bit_vector.rank(bit, index),
            Select {bit, index} => bit_vector.select(bit, index),
        });
    }
    let end_time = Instant::now();

    println!("RESULT name={NAME} time={} space={}", (end_time - start_time).as_millis(), bit_vector.get_size());
    let mut file_out = File::create(path_out).unwrap();
    let out = results.iter().map(|n| n.to_string()).collect::<Vec<String>>().join("\n");
    file_out.write_all(out.as_bytes()).expect("Failed to write output file");
}

/// Parses the file at `path_in` to generate a bit vector and a list of commands
fn parse_input(path_in: &str) -> (BitVector, Vec<Command>) {
    let mut file_in = File::open(path_in).unwrap();
    let mut content = String::new();
    file_in.read_to_string(&mut content).expect("Failed to read input file");
    let input_lines: Vec<&str> = content.lines().collect();

    let num_commands = input_lines[0].parse::<usize>().expect("First input line not a number");
    let bit_vector = BitVector::load_from_string(input_lines[1]);
    let commands: Vec<Command> = input_lines[2..].iter().copied().map(Command::from_string).collect();
    if num_commands != commands.len() {
        panic!("N and the number of commands differ");
    }
    (bit_vector, commands)
}
