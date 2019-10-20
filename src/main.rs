#![warn(clippy::all)]

extern crate squash;
use squash::*;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Bad args! {:?}", args);
        return;
    }
    let plaintext = fs::read(&args[1]).expect("File not found");
    let squashed = squash(&plaintext);
    let unsquashed = unsquash(&squashed).unwrap();
    println!("done");
    assert_eq!(&plaintext, &unsquashed);
    println!(
        "Ratio is {}",
        squashed.len() as f32 / plaintext.len() as f32
    );
}
