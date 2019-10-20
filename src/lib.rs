#![warn(clippy::all)]

use std::collections::VecDeque;
use std::convert::TryFrom;

#[cfg(test)]
mod test;

static BYTE_MASK: u8 = !0;

static BIGGEST_BIT_32: u32 = 1 << 31;

static BIGGEST_BIT_64: u64 = 1 << 63;

// the likelihood of a number in the arithmetic coding
// will never be considered less than padding / (padding * base + memory)
static FREQUENCY_MEMORY: usize = 1000;
static FREQUENCY_PADDING: u32 = 100;

pub fn squash(plaintext: &[u8]) -> Vec<u8> {
    let bwt_encoded = bw_transform(plaintext);
    let mtf_encoded = mtf_transform(&bwt_encoded.block);
    let rle_encoded = run_length_encode(&mtf_encoded);
    let arith_encoded = pack_arithmetic(
        &rle_encoded,
        |x| match x {
            RunEncoded::Byte(n) => u32::from(*n),
            RunEncoded::ZeroRun(Bijective::A) => 0,
            RunEncoded::ZeroRun(Bijective::B) => 256,
        },
        257,
    );
    add_front_matter(&arith_encoded, rle_encoded.len(), bwt_encoded.end_index)
}

pub fn unsquash(cyphertext: &[u8]) -> Result<Vec<u8>, &'static str> {
    let (body, front_matter) = get_front_matter(cyphertext)?;
    let arith_decoded = unpack_arithmetic(
        body,
        |x| match x {
            0 => RunEncoded::ZeroRun(Bijective::A),
            256 => RunEncoded::ZeroRun(Bijective::B),
            n => RunEncoded::Byte(u8::try_from(n).unwrap()),
        },
        257,
        front_matter.length,
    );
    let rle_decoded = run_length_decode(&arith_decoded);
    let mtf_decoded = mtf_untransform(&rle_decoded);
    let bw_decoded = bw_untransform(&BwVec {
        block: mtf_decoded,
        end_index: front_matter.end_index,
    });
    Ok(bw_decoded)
}

struct FrontMatter {
    length: usize,
    end_index: usize,
}

fn add_front_matter(body: &[u8], length: usize, end_index: usize) -> Vec<u8> {
    let mut front_matter: Vec<u8> = Vec::with_capacity(16 + body.len());
    for shift in 0..8 {
        // only works on 64 bit
        front_matter.push(u8::try_from(end_index >> (shift * 8) & BYTE_MASK as usize).unwrap());
    }
    for shift in 0..8 {
        // only works on 64 bit
        front_matter.push(u8::try_from(length >> (shift * 8) & BYTE_MASK as usize).unwrap());
    }
    for i in body {
        front_matter.push(*i);
    }
    front_matter
}

fn get_front_matter(body: &[u8]) -> Result<(&[u8], FrontMatter), &'static str> {
    if body.len() < 16 {
        return Err("too short");
    }
    let mut end_index: usize = 0;
    let mut length: usize = 0;
    for shift in 0..8 {
        // only works on 64 bit
        end_index |= (body[shift] as usize) << (shift * 8);
    }
    for shift in 0..8 {
        // only works on 64 bit
        length |= (body[shift + 8] as usize) << (shift * 8);
    }
    Ok((&body[16..], FrontMatter { length, end_index }))
}

struct Packer {
    result: Vec<u8>,
    working_byte: u8,
    next_bit: u8,
}

struct Unpacker<'a> {
    reserve: &'a [u8],
    current_byte: usize,
    next_bit: u8,
}

impl Packer {
    fn from_vec(base: Vec<u8>) -> Self {
        Packer {
            result: base,
            working_byte: 0,
            next_bit: 1,
        }
    }
    fn push(&mut self, bits: u8, length: u8) {
        assert!(length <= 8);
        for bit_offset in 0..length {
            if 0 != (1 << bit_offset) & bits {
                self.working_byte |= self.next_bit;
            } else {
            }
            if self.next_bit == 128 {
                self.next_bit = 1;
                self.result.push(self.working_byte);
                self.working_byte = 0;
            } else {
                self.next_bit <<= 1;
            }
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.next_bit != 1 {
            self.result.push(self.working_byte);
        }
        self.result
    }
}

impl<'a> Unpacker<'a> {
    fn from_vec(base: &'a [u8]) -> Self {
        Unpacker {
            reserve: base,
            current_byte: 0,
            next_bit: 1,
        }
    }
    fn pop(&mut self, length: u8) -> Option<u8> {
        assert!(length <= 8);
        let mut out = 0;
        for out_bit in 0..length {
            if self.current_byte >= self.reserve.len() {
                return None;
            }
            if 0 != (self.next_bit & self.reserve[self.current_byte]) {
                out |= 1 << out_bit;
            } else {
            }
            if self.next_bit == 128 {
                self.next_bit = 1;
                self.current_byte += 1;
            } else {
                self.next_bit <<= 1;
            }
        }
        Some(out)
    }
}

#[derive(PartialEq, Debug)]
struct BwVec {
    block: Vec<u8>,
    end_index: usize,
}

fn bw_transform(plaintext: &[u8]) -> BwVec {
    if plaintext.is_empty() {
        return BwVec {
            block: vec![],
            end_index: 0,
        };
    }
    // Gotta use a suffix array here
    let mut arr = Vec::with_capacity(plaintext.len());
    let mut arr2 = Vec::with_capacity(plaintext.len());
    for i in 0..plaintext.len() {
        let a = &plaintext[0..i];
        let b = &plaintext[i..plaintext.len()];
        arr.push([b, a].concat());
    }
    arr.sort();
    let mut end = 0;
    for item in &arr {
        arr2.push(item[plaintext.len() - 1]);
        if item == &plaintext {
            end = arr2.len() - 1;
        }
    }
    BwVec {
        block: arr2,
        end_index: end,
    }
}

fn bw_untransform(cyphertext: &BwVec) -> Vec<u8> {
    if cyphertext.block.is_empty() {
        return vec![];
    }
    let mut sorted = cyphertext.block.clone();
    sorted.sort();
    let mut out = Vec::with_capacity(cyphertext.block.len());
    let mut next_index = cyphertext.end_index;
    for _ in 0..cyphertext.block.len() {
        let next_item = sorted[next_index];
        let mut count = 0;
        // Can optimize by skipping the sort. Do the
        // preprocessing for the cyphertext, then
        // add up the counts incrementally to get a table
        // of the first position of each new digit in the sorted array
        // Like countsort
        for (index, val) in sorted.iter().enumerate() {
            if val == &next_item {
                count += 1;
            }
            if index == next_index {
                break;
            }
        }
        out.push(sorted[next_index]);
        let mut count2 = 0;
        // Can optimize by preprocessing
        // Iterate through once, annotating each one
        for (index, val) in cyphertext.block.iter().enumerate() {
            if val == &next_item {
                count2 += 1;
            }
            if count == count2 {
                next_index = index;
                break;
            }
        }
    }
    out
}

// Can turn into an iterator
fn mtf_transform(plaintext: &[u8]) -> Vec<u8> {
    let mut dict = Vec::with_capacity(256);
    let mut out = Vec::with_capacity(plaintext.len());
    for i in 0..255 {
        dict.push(i as u8);
    }
    for item in plaintext {
        for i in 0..255 {
            if dict[i] == *item {
                dict.remove(i);
                dict.insert(0, *item);
                out.push(i as u8);
            }
        }
    }
    out
}

// Can turn into an iterator
fn mtf_untransform(cyphertext: &[u8]) -> Vec<u8> {
    let mut dict = Vec::with_capacity(256);
    let mut out = Vec::with_capacity(cyphertext.len());
    for i in 0..255 {
        dict.push(i as u8);
    }
    for item in cyphertext {
        let i = dict[*item as usize];
        out.push(i);
        dict.remove(*item as usize);
        dict.insert(0, i);
    }
    out
}

#[derive(PartialEq, Debug)]
enum RunEncoded {
    Byte(u8),
    ZeroRun(Bijective),
}

fn run_length_encode(plaintext: &[u8]) -> Vec<RunEncoded> {
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

fn run_length_decode(cyphertext: &[RunEncoded]) -> Vec<u8> {
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
enum Bijective {
    A,
    B,
}

fn to_bijective(num: u32) -> Vec<Bijective> {
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

fn from_bijective(num: &[Bijective]) -> u32 {
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

fn pack_arithmetic<T>(plaintext: &[T], encode: fn(&T) -> u32, base: u32) -> Vec<u8> {
    let mut out = Packer::from_vec(vec![]);
    let mut queue: VecDeque<u32> = VecDeque::with_capacity(FREQUENCY_MEMORY);
    let mut freqs: Vec<u32> = Vec::with_capacity(base as usize);
    let mut bottom: u64 = 0;
    let mut top: u64 = !0;
    let total_padding = base * FREQUENCY_PADDING;
    for _ in 0..base {
        freqs.push(FREQUENCY_PADDING);
    }
    for item in plaintext {
        let mut lower = 0;
        let code = encode(item);
        for f in &freqs[0..code as usize] {
            lower += f
        }
        let upper = lower + freqs[code as usize];
        let diff = top - bottom;
        let total = total_padding + u32::try_from(queue.len()).unwrap();
        top = bottom + (diff / u64::from(total)) * u64::from(upper);
        bottom += (diff / u64::from(total)) * u64::from(lower);
        while bottom & BIGGEST_BIT_64 == top & BIGGEST_BIT_64 {
            if bottom & BIGGEST_BIT_64 == 0 {
                out.push(0, 1);
            } else {
                out.push(1, 1);
            }
            bottom <<= 1;
            top <<= 1
        }
        queue.push_back(code);
        freqs[code as usize] += 1;
        if queue.len() > FREQUENCY_MEMORY {
            freqs[queue.pop_front().unwrap() as usize] -= 1;
        }
    }
    out.push(1, 1);
    out.finish()
}

pub fn unpack_arithmetic<T>(
    cyphertext: &[u8],
    decode: fn(u32) -> T,
    base: u32,
    length: usize,
) -> Vec<T> {
    let mut unpacker = Unpacker::from_vec(cyphertext);
    let mut out: Vec<T> = Vec::with_capacity(cyphertext.len());
    let mut queue: VecDeque<u32> = VecDeque::with_capacity(FREQUENCY_MEMORY);
    let mut freqs: Vec<u32> = Vec::with_capacity(base as usize);
    let mut bottom: u64 = 0;
    let mut top: u64 = !0;
    let mut unpacked: u64 = 0;
    let mut operating_bit = BIGGEST_BIT_64;
    for _ in 0..64 {
        match unpacker.pop(1) {
            Some(1) => {
                unpacked |= operating_bit;
            }
            None => {
                break;
            }
            _ => (),
        }
        operating_bit >>= 1;
    }
    let total_padding = base * FREQUENCY_PADDING;
    for _ in 0..base {
        freqs.push(FREQUENCY_PADDING);
    }
    for _ in 0..length {
        let diff = top - bottom;
        let total = total_padding + u32::try_from(queue.len()).unwrap();
        let mut lower = 0;
        let mut upper = 0;
        let mut code = 0;
        for (i, f) in freqs.iter().enumerate() {
            if bottom + (diff / u64::from(total)) * u64::from(lower + f) < unpacked {
                lower += f;
            } else {
                upper = lower + f;
                code = u32::try_from(i).unwrap();
                out.push(decode(code));
                break;
            }
        }
        top = bottom + (diff / u64::from(total)) * u64::from(upper);
        bottom += (diff / u64::from(total)) * u64::from(lower);
        let mut counter = 0;
        let mut comparison_bit = BIGGEST_BIT_64;
        while bottom & comparison_bit == top & comparison_bit {
            counter += 1;
            comparison_bit >>= 1;
        }
        top <<= counter;
        bottom <<= counter;
        unpacked <<= counter;
        for new_bit in (0..counter).rev() {
            match unpacker.pop(1) {
                Some(1) => {
                    unpacked |= 1 << new_bit;
                }
                Some(0) => (),
                Some(_) => panic!("this should be impossible"),
                None => {
                    break;
                }
            }
        }
        queue.push_back(code);
        freqs[code as usize] += 1;
        if queue.len() > FREQUENCY_MEMORY {
            freqs[queue.pop_front().unwrap() as usize] -= 1;
        }
    }
    out
}
