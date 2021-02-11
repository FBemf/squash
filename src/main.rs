use squash::squash_algorithm::*;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Bad args! {:?}", args);
        return;
    }
    if args[1] == "enc" {
        let mut read = match fs::File::open(&args[2]) {
            Ok(x) => x,
            Err(x) => panic!(x),
        };
        let mut write = match fs::File::create(&args[3]) {
            Ok(x) => x,
            Err(x) => panic!(x),
        };
        match squash(&mut read, &mut write) {
            Ok(()) => (),
            Err(x) => eprintln!("Error: {}", x),
        }
    } else if args[1] == "dec" {
        let mut read = match fs::File::open(&args[2]) {
            Ok(x) => x,
            Err(x) => panic!(x),
        };
        let mut write = match fs::File::create(&args[3]) {
            Ok(x) => x,
            Err(x) => panic!(x),
        };
        match unsquash(&mut read, &mut write) {
            Ok(()) => (),
            Err(x) => panic!(x),
        }
    } else {
        eprintln!("Bad args! {:?}", args)
    }
}
