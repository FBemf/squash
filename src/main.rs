use squash::squash_algorithm::*;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Bad args! {:?}", args);
        return;
    }

    let mut input_file = match fs::File::open(&args[2]) {
        Ok(x) => x,
        Err(x) => panic!("Unable to open input file {}", x),
    };
    let mut output_file = match fs::File::create(&args[3]) {
        Ok(x) => x,
        Err(x) => panic!("Unable to open output file {}", x),
    };

    if args[1] == "enc" {
        match squash(&mut input_file, &mut output_file) {
            Ok(()) => (),
            Err(x) => eprintln!("Error: {}", x),
        }
    } else if args[1] == "dec" {
        match unsquash(&mut input_file, &mut output_file) {
            Ok(()) => (),
            Err(x) => eprintln!("Error: {}", x),
        }
    } else {
        eprintln!("Bad args! {:?}", args)
    }
}
