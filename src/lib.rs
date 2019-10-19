#![warn(clippy::all)]

use std::collections::VecDeque;
use std::convert::TryFrom;

#[cfg(test)]
mod test;

pub fn squash(plaintext: &[u8]) -> Vec<u8> {
    let bwt_encoded = bw_transform(plaintext);
    let mtf_encoded = mtf_transform(&bwt_encoded.block);
    let rle_encoded = run_length_encode(&mtf_encoded);
    bit_pack(&rle_encoded, bwt_encoded.end_index)
}

pub fn unsquash(cyphertext: &[u8]) -> Result<Vec<u8>, &'static str> {
    let (unpacked, end_index) = bit_unpack(&cyphertext)?;
    let rle_decoded = run_length_decode(&unpacked);
    let mtf_decoded = mtf_untransform(&rle_decoded);
    let bw_decoded = bw_untransform(&BwVec {
        block: mtf_decoded,
        end_index,
    });
    Ok(bw_decoded)
}

static MAX_RUN_LENGTH: u8 = 32;

#[derive(PartialEq, Debug)]
struct BwVec {
    block: Vec<u8>,
    end_index: usize,
}

#[derive(PartialEq, Debug, Clone)]
struct Run {
    byte: u8,
    length: u8,
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
                self.working_byte = self.working_byte | self.next_bit;
            } else {
            }
            if self.next_bit == 128 {
                self.next_bit = 1;
                self.result.push(self.working_byte);
                self.working_byte = 0;
            } else {
                self.next_bit = self.next_bit << 1;
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

fn bit_pack(cyphertext: &[Run], end_index: usize) -> Vec<u8> {
    let byte_mask: u8 = 255;
    let mut end_index_mut = end_index;
    let mut packed: Vec<u8> = Vec::with_capacity(cyphertext.len());
    for _ in 0..8 {
        // only works on x64
        packed.push(u8::try_from(end_index_mut & byte_mask as usize).unwrap());
        end_index_mut >>= 8;
    }

    let mut packing_state = Packer::from_vec(packed);
    for item in cyphertext {
        if item.byte == 0 {
            packing_state.push(0, 1);
            packing_state.push(item.length - 1, 5);
        } else {
            for _ in 0..item.length {
                packing_state.push(1, 1);
                packing_state.push(item.byte, 8);
            }
        }
    }
    packing_state.push(1, 1);
    packing_state.finish()
}

fn bit_unpack<'a>(packed: &'a [u8]) -> Result<(Vec<Run>, usize), &'static str> {
    let mut shift_amount = 0;
    let mut unpacked: Vec<Run> = Vec::with_capacity(packed.len());
    let mut end_index: usize = 0;
    for byte in &packed[0..8] {
        // only works on x64
        end_index |= (*byte as usize) << shift_amount;
        shift_amount += 8;
    }
    let mut unpacker = Unpacker::from_vec(&packed[8..]);

    loop {
        match unpacker.pop(1) {
            Some(0) => {
                // zero case
                match unpacker.pop(5) {
                    Some(n) => {
                        unpacked.push(Run {
                            byte: 0,
                            length: n + 1,
                        });
                    }
                    None => {
                        return Err("bad format");
                    }
                }
            }
            Some(1) => {
                // non-zero case
                match unpacker.pop(8) {
                    Some(n) => {
                        unpacked.push(Run { byte: n, length: 1 });
                    }
                    None => {
                        return Ok((unpacked, end_index));
                    }
                }
            }
            Some(_) => panic!("impossible bit"),
            None => {
                return Err("bad format");
            }
        }
    }
}

fn bw_transform(plaintext: &[u8]) -> BwVec {
    if plaintext.is_empty() {
        return BwVec {
            block: vec![],
            end_index: 0,
        };
    }
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

fn run_length_encode(plaintext: &[u8]) -> Vec<Run> {
    let mut out = Vec::with_capacity(plaintext.len());
    if !plaintext.is_empty() {
        out.push(Run {
            byte: plaintext[0],
            length: 1,
        });
        for item in &plaintext[1..] {
            let last_length = out.last().unwrap().length;
            if *item == 0 && *item == out.last().unwrap().byte && last_length < MAX_RUN_LENGTH {
                out.pop();
                out.push(Run {
                    byte: *item,
                    length: last_length + 1,
                });
            } else {
                out.push(Run {
                    byte: *item,
                    length: 1,
                });
            }
        }
    }
    out
}

fn run_length_decode(cyphertext: &[Run]) -> Vec<u8> {
    let mut out = Vec::with_capacity(cyphertext.len());
    for item in cyphertext {
        for _ in 0..item.length {
            out.push(item.byte);
        }
    }
    out
}

#[derive(PartialEq, Debug)]
enum RunEncode {
    RunA,
    RunB,
}

static BIGGEST_BIT_32: u32 = 1 << 31;

fn to_bijective(num: u32) -> Vec<RunEncode> {
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
                    out.push(RunEncode::RunA);
                } else {
                    out.push(RunEncode::RunB);
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

fn from_bijective(num: &[RunEncode]) -> u32 {
    if num.len() == 32 {
        return 0xffff_ffff;
    }
    if num.len() > 32 {
        panic!("too long to be valid");
    }
    let mut out = 0;
    let mut bit = 1;
    for item in num {
        if let RunEncode::RunB = item {
            out |= bit;
        }
        bit <<= 1;
    }
    let base = (1 << num.len()) - 1;
    out + base
}

static BIGGEST_BIT_64: u64 = 1 << 63;

// the likelihood of a number in the arithmetic coding
// will never be considered less than padding / (padding * base + memory)
static FREQUENCY_MEMORY: usize = 1000;
static FREQUENCY_PADDING: u32 = 100;

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
