#![warn(clippy::all)]

extern crate squash;
use squash::*;
use std::convert::TryFrom;

static TEXT: &str =
    "Let’s break down the match in the value_in_cents function. \
     First, we list the match keyword followed by an expression, which in this case is \
     the value coin. This seems very similar to an expression used with if, but there\
     ’s a big difference: with if, the expression needs to return a Boolean value, but \
     here, it can be any type. The type of coin in this example is the Coin enum that \
     we defined on line 1.\n\
     Next are the match arms. An arm has two parts: a pattern and some code. The first \
     arm here has a pattern that is the value Coin::Penny and then the => operator that \
     separates the pattern and the code to run. The code in this case is just the value \
     1. Each arm is separated from the next with a comma.\n\
     When the match expression executes, it compares the resulting value against the \
     pattern of each arm, in order. If a pattern matches the value, the code associated \
     with that pattern is executed. If that pattern doesn’t match the value, execution \
     continues to the next arm, much as in a coin-sorting machine. We can have as many \
     arms as we need: in Listing 6-3, our match has four arms.\n\
     The code associated with each arm is an expression, and the resulting value of the \
     expression in the matching arm is the value that gets returned for the entire match \
     expression.\n\
     Curly brackets typically aren’t used if the match arm code is short, as it is in \
     Listing 6-3 where each arm just returns a value. If you want to run multiple lines \
     of code in a match arm, you can use curly brackets. For example, the following code \
     would print “Lucky penny!” every time the method was called with a Coin::Penny but \
     would still return the last value of the block, 1:\n";

fn main() {
    let _ = unpack_arithmetic(
        &[143, 13, 36, 1],
        |b| u8::try_from(b).unwrap() + b"a"[0],
        4,
        11,
    );
    let plaintext = TEXT.as_bytes();
    let squashed = squash(plaintext);
    let unsquashed = unsquash(&squashed).unwrap();
    assert_eq!(plaintext, &unsquashed[..]);
    println!(
        "Ratio is {}",
        squashed.len() as f32 / plaintext.len() as f32
    );
}
