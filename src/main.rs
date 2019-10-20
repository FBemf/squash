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
    let plaintext = fs::read_to_string(&args[1]).expect("File not found");
    let squashed = squash(plaintext.as_bytes());
    let unsquashed = unsquash(&squashed).unwrap();
    assert_eq!(plaintext, String::from_utf8_lossy(&unsquashed));
    println!(
        "Ratio is {}",
        squashed.len() as f32 / plaintext.len() as f32
    );
}
