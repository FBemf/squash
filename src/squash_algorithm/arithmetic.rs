#![warn(clippy::all)]

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::convert::TryInto;

const BIGGEST_BIT_64: u64 = 1 << 63;

// the likelihood of a number in the arithmetic coding
// will never be considered less than padding / (padding * base + memory)
const FREQUENCY_MEMORY: u32 = 10_000;
const FREQUENCY_PADDING: u32 = 50;
const RECALCULATION_FREQUENCY: u32 = 50;

pub fn pack_arithmetic<T>(
    front_matter: Vec<u8>,
    plaintext: &[T],
    encode: fn(&T) -> u32,
    base: u32,
) -> Vec<u8> {
    let mut out = Packer::from_vec(front_matter);
    let mut queue: VecDeque<u32> = VecDeque::with_capacity(FREQUENCY_MEMORY.try_into().unwrap());
    let mut freqs: Vec<u32> = Vec::with_capacity(base as usize);
    let mut freq_map: HashMap<u32, u64> = HashMap::new();
    let mut bottom: u64 = 0;
    let mut top: u64 = !0;
    let mut time_till_recalc = 0;
    let mut total = 0;
    let total_padding = base * FREQUENCY_PADDING;
    for _ in 0..base {
        freqs.push(FREQUENCY_PADDING);
    }
    for item in plaintext {
        if time_till_recalc == 0 {
            time_till_recalc = RECALCULATION_FREQUENCY;
            let mut total_so_far: u64 = 0;
            for (i, freq) in freqs.iter().enumerate() {
                freq_map.insert(u32::try_from(i).unwrap(), total_so_far);
                total_so_far += u64::from(*freq);
            }
            freq_map.insert(base, total_so_far);
            total = u64::from(total_padding) + u64::try_from(queue.len()).unwrap();
        } else {
            time_till_recalc -= 1;
        }
        let code = encode(item);
        let lower = freq_map[&code];
        let upper = freq_map[&(code + 1)];
        let diff = top - bottom;
        top = bottom + (diff / total) * upper;
        bottom += (diff / total) * lower;
        while bottom & BIGGEST_BIT_64 == top & BIGGEST_BIT_64 {
            if bottom & BIGGEST_BIT_64 == 0 {
                out.push(0, 1);
            } else {
                out.push(1, 1);
            }
            bottom <<= 1;
            top <<= 1;
        }
        queue.push_back(code);
        freqs[code as usize] += 1;
        if queue.len() > FREQUENCY_MEMORY.try_into().unwrap() {
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
    let mut queue: VecDeque<u32> = VecDeque::with_capacity(FREQUENCY_MEMORY.try_into().unwrap());
    let mut freqs: Vec<u32> = Vec::with_capacity(base as usize);
    let mut freq_map: HashMap<u32, u64> = HashMap::new();
    let mut freq_map_reverse: BTreeMap<u64, u32> = BTreeMap::new();
    let mut time_till_recalc = 0;
    let mut bottom: u64 = 0;
    let mut top: u64 = !0;
    let mut unpacked: u64 = 0;
    let mut operating_bit = BIGGEST_BIT_64;
    let mut total = 0;
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
        if time_till_recalc == 0 {
            time_till_recalc = RECALCULATION_FREQUENCY;
            freq_map_reverse.clear();
            let mut total_so_far: u64 = 0;
            for (i, freq) in freqs.iter().enumerate() {
                freq_map.insert(u32::try_from(i).unwrap(), total_so_far);
                freq_map_reverse.insert(total_so_far, u32::try_from(i).unwrap());
                total_so_far += u64::from(*freq);
            }
            freq_map.insert(base, total_so_far);
            total = u64::from(total_padding) + u64::try_from(queue.len()).unwrap();
        } else {
            time_till_recalc -= 1;
        }
        let diff = top - bottom;
        let cap =
            u64::try_from((u128::from(unpacked - bottom) * u128::from(total)) / u128::from(diff))
                .unwrap();
        let mut code = match freq_map_reverse.range(0..cap).rev().next() {
            Some((_, code)) => *code,
            None => 0,
        };
        while code + 1 < base && bottom + (diff / total) * freq_map[&(code + 1)] < unpacked {
            code += 1;
        }
        let lower = freq_map[&code];
        let upper = freq_map[&(code + 1)];
        out.push(decode(code));
        top = bottom + (diff / total) * upper;
        bottom += (diff / total) * lower;
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
        if queue.len() > FREQUENCY_MEMORY.try_into().unwrap() {
            freqs[queue.pop_front().unwrap() as usize] -= 1;
        }
    }
    out
}

pub struct Packer {
    result: Vec<u8>,
    working_byte: u8,
    next_bit: u8,
}

pub struct Unpacker<'a> {
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

#[cfg(test)]
mod test {
    use super::super::test_text::*;
    use super::*;

    #[test]
    fn arithmetic_test() {
        let test = b"ddabdaddabccda";
        let alphabet_size = 4;
        let enc = pack_arithmetic(vec![], test, |a| u32::from(a - b"a"[0]), alphabet_size);
        let dec = unpack_arithmetic(
            &enc,
            |b| u8::try_from(b).unwrap() + b"a"[0],
            alphabet_size,
            test.len(),
        );
        assert_eq!(&test[..], &dec[..]);

        let test = b"qwertyqweyrtqwyeeewteyyrqwwerttqywetrtrrrrrrrrrrwert";
        let alphabet_size = 26;
        let enc = pack_arithmetic(vec![], test, |a| u32::from(a - b"a"[0]), alphabet_size);
        let dec = unpack_arithmetic(
            &enc,
            |b| u8::try_from(b).unwrap() + b"a"[0],
            alphabet_size,
            test.len(),
        );
        assert_eq!(&test[..], &dec[..]);

        let packed_text = pack_arithmetic(vec![], TEXT.as_bytes(), |a| u32::from(*a), 256);
        assert_eq!(
            String::from_utf8_lossy(&unpack_arithmetic(
                &packed_text,
                |b| u8::try_from(b).unwrap(),
                256,
                TEXT.len()
            )),
            TEXT
        );
    }

    #[test]
    fn packers_test() {
        let mut p = Packer::from_vec(vec![]);
        p.push(5, 5);
        p.push(7, 8);
        p.push(2, 2);
        let f = p.finish();
        let mut u = Unpacker::from_vec(&f);
        assert_eq!(u.pop(5), Some(5));
        assert_eq!(u.pop(8), Some(7));
        assert_eq!(u.pop(2), Some(2));
    }
}
