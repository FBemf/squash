#![warn(clippy::all)]

extern crate squash;
use squash::*;
use std::env;
use std::fs;
use std::time::Instant;

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
            Ok(x) => eprintln!("Wrote {}.", x),
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
            Ok(x) => eprintln!("Wrote {}.", x),
            Err(x) => panic!(x),
        }
    } else {
        eprintln!("Bad args! {:?}", args)
    }
}

fn _main_old() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Bad args! {:?}", args);
        return;
    }
    let plaintext = fs::read(&args[1]).expect("File not found");
    let t1 = Instant::now();
    let squashed = squash_block(&plaintext);
    let t1 = t1.elapsed();
    let t2 = Instant::now();
    let unsquashed = unsquash_block(&squashed).unwrap();
    let t2 = t2.elapsed();
    println!("{:?} to compress, {:?} to decompress", t1, t2);
    assert_eq!(&plaintext, &unsquashed);
    println!(
        "Ratio is {}",
        squashed.len() as f32 / plaintext.len() as f32
    );
}
