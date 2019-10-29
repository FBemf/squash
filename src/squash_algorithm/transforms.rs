#![warn(clippy::all)]

use crate::suffixarray::SuffixArray;
use std::convert::TryInto;

const BIGGEST_BIT_32: u32 = 1 << 31;

#[derive(PartialEq, Debug)]
pub struct BwVec {
    pub block: Vec<u8>,
    pub end_index: u32,
}

pub fn bw_transform(plaintext: &[u8]) -> BwVec {
    if plaintext.is_empty() {
        return BwVec {
            block: vec![],
            end_index: 0,
        };
    }
    let suffix_array = SuffixArray::from_array(plaintext);
    let mut out = Vec::with_capacity(plaintext.len());
    let mut end = 0;
    for (s_index, s_val) in suffix_array.raw().iter().enumerate() {
        let p_index = *s_val;
        if p_index == 0 {
            out.push(36);
            end = s_index;
        } else {
            out.push(plaintext[p_index - 1]);
        }
    }
    BwVec {
        block: out,
        end_index: end.try_into().unwrap(),
    }
}

pub fn bw_untransform(cyphertext: &BwVec) -> Vec<u8> {
    let mut out = vec![0; cyphertext.block.len() - 1];

    // counts stores the number of items of each
    // kind in the cyphertext
    let mut counts = vec![0; 256];

    // position stores, for each entry in cyphetext, n
    // where the entry is the nth instance of its kind
    // starting at zero
    let mut position = vec![0; cyphertext.block.len()];
    for (index, val) in cyphertext.block.iter().enumerate() {
        if index != cyphertext.end_index.try_into().unwrap() {
            position[index] = counts[*val as usize];
            counts[*val as usize] += 1;
        }
    }

    // now counts become summed to be the index of
    // the beginning of each section of the sorted array
    let mut sections = counts;
    for i in (0..sections.len() - 1).rev() {
        sections[i + 1] = sections[i]
    }
    sections[0] = 1;
    for i in 1..sections.len() {
        sections[i] += sections[i - 1];
    }

    let mut next_index = 0;
    for out_index in (0..cyphertext.block.len() - 1).rev() {
        let next_item = cyphertext.block[next_index];
        out[out_index] = next_item;

        let char_position = position[next_index];

        next_index = sections[next_item as usize] + char_position;
    }
    out
}

pub fn mtf_transform(plaintext: &[u8]) -> Vec<u8> {
    let mut dict = Vec::with_capacity(256);
    let mut out = Vec::with_capacity(plaintext.len());
    for i in (0..256).rev() {
        dict.push(i as u8);
    }
    for item in plaintext {
        for i in (0..256).rev() {
            if dict[i] == *item {
                dict.remove(i);
                dict.push(*item);
                out.push(i as u8);
            }
        }
    }
    out
}

pub fn mtf_untransform(cyphertext: &[u8]) -> Vec<u8> {
    let mut dict = Vec::with_capacity(256);
    let mut out = Vec::with_capacity(cyphertext.len());
    for i in (0..256).rev() {
        dict.push(i as u8);
    }
    for item in cyphertext {
        let i = dict[*item as usize];
        out.push(i);
        dict.remove(*item as usize);
        dict.push(i);
    }
    out
}

#[derive(PartialEq, Debug)]
pub enum RunEncoded {
    Byte(u8),
    ZeroRun(Bijective),
}

pub fn run_length_encode(plaintext: &[u8]) -> Vec<RunEncoded> {
    let mut out = Vec::with_capacity(plaintext.len());
    let mut index = 0;
    loop {
        if index >= plaintext.len() {
            return out;
        }
        if plaintext[index] != 0 {
            out.push(RunEncoded::Byte(plaintext[index]));
            index += 1;
        } else {
            let mut zeros = 0;
            while index < plaintext.len() && plaintext[index] == 0 {
                zeros += 1;
                index += 1;
            }
            for item in to_bijective(zeros) {
                out.push(RunEncoded::ZeroRun(item));
            }
        }
    }
}

pub fn run_length_decode(cyphertext: &[RunEncoded]) -> Vec<u8> {
    let mut out = Vec::with_capacity(cyphertext.len());
    let mut index = 0;
    loop {
        if index >= cyphertext.len() {
            break;
        }
        if let RunEncoded::Byte(b) = cyphertext[index] {
            out.push(b);
            index += 1;
        } else {
            let mut zeros = vec![];
            while index < cyphertext.len() {
                if let RunEncoded::ZeroRun(z) = cyphertext[index] {
                    zeros.push(z);
                    index += 1
                } else {
                    break;
                }
            }
            for _ in 0..from_bijective(&zeros) {
                out.push(0);
            }
        }
    }
    out
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Bijective {
    A,
    B,
}

pub fn to_bijective(num: u32) -> Vec<Bijective> {
    let mut out = vec![];
    if num == 0 {
        panic!("can't use zero");
    }
    let mut sieve = 0;
    let mut sieve_increment = 2;
    loop {
        let next_sieve = sieve | sieve_increment;
        if next_sieve >= num || sieve_increment == BIGGEST_BIT_32 {
            let num2 = num - sieve - 1;
            let mut pushing_bit = 1;
            while pushing_bit < sieve_increment {
                if num2 & pushing_bit == 0 {
                    out.push(Bijective::A);
                } else {
                    out.push(Bijective::B);
                }
                pushing_bit <<= 1;
            }
            break;
        }
        sieve_increment <<= 1;
        sieve = next_sieve;
    }
    out
}

pub fn from_bijective(num: &[Bijective]) -> u32 {
    if num.is_empty() {
        return 0;
    }
    if num.len() == 32 {
        return 0xffff_ffff;
    }
    if num.len() > 32 {
        panic!("too long to be valid");
    }
    let mut out = 0;
    let mut bit = 1;
    for item in num {
        if let Bijective::B = item {
            out |= bit;
        }
        bit <<= 1;
    }
    let base = (1 << num.len()) - 1;
    out + base
}

#[test]
fn bwt_test() {
    let test = b"banana_banana";
    let enc = bw_transform(test);
    assert_eq!(bw_untransform(&enc), test);

    let test = b"banana_banana$";
    let enc = bw_transform(test);
    assert_eq!(bw_untransform(&enc), test);

    let test = b"blooby blabby blam. man manam malamla. blom blooby blop.";
    let enc = bw_transform(test);
    assert_eq!(
        String::from_utf8_lossy(&bw_untransform(&enc)),
        String::from_utf8_lossy(test)
    );

    let test = b"abcdabcdefghefgh";
    let enc = bw_transform(test);
    assert_eq!(bw_untransform(&enc), test);

    let test = b"toblerone bars";
    let enc = bw_transform(test);
    assert_eq!(bw_untransform(&enc), test);
}

#[test]
fn mtf_test() {
    let test = b"aaaaabbbbbcccccddddd";
    let enc = mtf_transform(test);
    assert_eq!(mtf_untransform(&enc), test);

    let test = b"syllogism";
    let enc = mtf_transform(test);
    assert_eq!(mtf_untransform(&enc), test);

    assert_eq!(mtf_transform(&[]), &[]);
    assert_eq!(mtf_untransform(&[]), &[]);
}

#[test]
fn rle_test() {
    let test = b"bbfdddeejreewwwer";
    let enc = run_length_encode(test);
    assert_eq!(run_length_decode(&enc), test);
    assert_eq!(run_length_encode(b""), []);
}
