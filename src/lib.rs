#![warn(clippy::all)]

#[cfg(test)]
mod test;

use std::convert::TryInto;

pub fn squash(plaintext: &[u8]) -> (Vec<u8>, usize) {
    let bwt = bw_transform(plaintext);
    let encoded = run_length_encode(&mtf_transform(&bwt.block));
    let squashed: Vec<u8> = encoded
        .iter()
        .flat_map(|a| vec![a.byte, a.length])
        .collect();
    (squashed, bwt.end_index)
}

pub fn unsquash(cyphertext: &[u8], end_index: usize) -> Vec<u8> {
    let mut unsquashed = vec![];
    for index in 0..cyphertext.len() / 2 {
        unsquashed.push(Run {
            byte: cyphertext[index * 2],
            length: cyphertext[index * 2 + 1],
        });
    }
    let plaintext = bw_untransform(BwVec {
        block: mtf_untransform(&run_length_decode(&unsquashed)),
        end_index: end_index,
    });
    plaintext
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

struct Unpacker {
    reserve: Vec<u8>,
    working_byte: u8,
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
    fn append(&mut self, bits: u8, length: u8) {
        let mut bit_mask = 1;
        for _ in 0..length {
            self.working_byte = self.working_byte | (bit_mask & bits);
            bit_mask = bit_mask << 1;
            if self.next_bit == 128 {
                self.next_bit = 1;
                self.result.push(self.working_byte);
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

impl Unpacker {
    fn from_vec(mut base: Vec<u8>) -> Self {
        base.reverse();
        Unpacker {
            reserve: base,
            working_byte: 0,
            next_bit: 1,
        }
    }
    fn append(&mut self, length: u8) -> Option<u8> {
        let mut out = 0;
        for _ in 0..length {
            if self.reserve.is_empty() {
                return None;
            }
            out = out | (self.next_bit & self.reserve[self.reserve.len()]);
            if self.next_bit == 128 {
                self.next_bit = 1;
                self.reserve.pop();
            } else {
                self.next_bit = self.next_bit << 1;
            }
        }
        Some(out)
    }
}

fn bit_pack(cyphertext: &[Run], end_index: usize) -> Vec<u8> {
    let byte_mask: u8 = 255;
    let mut end_index_mut = end_index;
    let mut packed: Vec<u8> = vec![];
    for _ in 0..8 {
        // only works on x64
        packed.push((end_index_mut & byte_mask as usize).try_into().unwrap());
        end_index_mut = end_index_mut >> 8;
    }

    let mut packing_state = Packer::from_vec(packed);
    for item in cyphertext {
        if item.byte == 0 {
            packing_state.append(0, 1);
            packing_state.append(item.length - 1, 5);
        } else {
            for _ in 0..item.length {
                packing_state.append(1, 1);
                packing_state.append(item.byte, 8);
            }
        }
    }
    packing_state.finish()
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

fn bw_untransform(cyphertext: BwVec) -> Vec<u8> {
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
    println!("{:?}", out);
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
            if item == &out.last().unwrap().byte && last_length < MAX_RUN_LENGTH {
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

fn run_length_decode(cyphertext: &Vec<Run>) -> Vec<u8> {
    let mut out = Vec::with_capacity(cyphertext.len());
    if !cyphertext.is_empty() {
        for item in cyphertext {
            for _ in 0..item.length {
                out.push(item.byte);
            }
        }
    }
    out
}
