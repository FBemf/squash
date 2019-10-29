#![warn(clippy::all)]

use arithmetic::*;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::io;
use transforms::*;

mod arithmetic;
mod test_text;
mod transforms;

// the likelihood of a number in the arithmetic coding
// will never be considered less than padding / (padding * base + memory)
const FREQUENCY_MEMORY: u32 = 10_000;
const FREQUENCY_PADDING: u32 = 50;
const RECALCULATION_FREQUENCY: u32 = 50;

const BLOCK_SIZE: usize = 1 << 18;

const MAGIC_NUMBER: u32 = 0xca55_e77e;
const FILETYPE_VERSION: u8 = 1;

pub fn squash(reader: &mut dyn io::Read, writer: &mut dyn io::Write) -> Result<usize, io::Error> {
    let mut bytes_read = 0;
    let mut bytes_written = 0;

    let mut four_bytes = MAGIC_NUMBER.to_le_bytes();
    bytes_written += writer.write(&four_bytes)?;

    let one_byte = FILETYPE_VERSION.to_le_bytes();
    bytes_written += writer.write(&one_byte)?;

    four_bytes = FREQUENCY_MEMORY.to_le_bytes();
    bytes_written += writer.write(&four_bytes)?;
    let frequency_memory = u32::from_le_bytes(four_bytes);

    four_bytes = FREQUENCY_PADDING.to_le_bytes();
    bytes_written += writer.write(&four_bytes)?;
    let frequency_padding = u32::from_le_bytes(four_bytes);

    four_bytes = RECALCULATION_FREQUENCY.to_le_bytes();
    bytes_written += writer.write(&four_bytes)?;
    let recalculation_frequency = u32::from_le_bytes(four_bytes);

    let _ = frequency_memory;
    let _ = frequency_padding;
    let _ = recalculation_frequency;

    loop {
        let mut block = vec![0; BLOCK_SIZE];
        let bytes = reader.read(&mut block)?;
        match bytes {
            0 => {
                break;
            }
            n => {
                bytes_read += n;
                let squashed = squash_block(&block[0..n]);
                let squashed_len = u32::try_from(squashed.len()).unwrap().to_le_bytes();
                bytes_written += writer.write(&squashed_len)?;
                bytes_written += writer.write(&squashed)?;
            }
        };
    }
    let _ = bytes_read;
    Ok(bytes_written)
}

pub fn unsquash(reader: &mut dyn io::Read, writer: &mut dyn io::Write) -> Result<usize, io::Error> {
    let mut one_byte: [u8; 1] = [0; 1];
    let mut four_bytes: [u8; 4] = [0; 4];
    let mut bytes_read: usize = 0;
    let mut bytes_written: usize = 0;

    bytes_read += reader.read(&mut four_bytes)?;
    let magic_number = u32::from_le_bytes(four_bytes);
    assert_eq!(magic_number, MAGIC_NUMBER);

    bytes_read += reader.read(&mut one_byte)?;
    let version_number = u8::from_le_bytes(one_byte);
    assert_eq!(version_number, FILETYPE_VERSION);

    bytes_read += reader.read(&mut four_bytes)?;
    let frequency_memory = u32::from_le_bytes(four_bytes);
    assert_eq!(frequency_memory, FREQUENCY_MEMORY);

    bytes_read += reader.read(&mut four_bytes)?;
    let frequency_padding = u32::from_le_bytes(four_bytes);
    assert_eq!(frequency_padding, FREQUENCY_PADDING);

    bytes_read += reader.read(&mut four_bytes)?;
    let recalculation_frequency = u32::from_le_bytes(four_bytes);
    assert_eq!(recalculation_frequency, RECALCULATION_FREQUENCY);

    loop {
        let block_len = match reader.read(&mut four_bytes)? {
            0 => {
                break;
            }
            4 => {
                bytes_read += 4;
                u32::from_le_bytes(four_bytes)
            }
            _ => {
                panic!("expected something");
            }
        };
        let mut block = vec![0; block_len.try_into().unwrap()];
        match reader.read(&mut block)? {
            0 => {
                panic!("expected something");
            }
            n => {
                let _ = n;
                bytes_read += usize::try_from(block_len).unwrap();
                match unsquash_block(&block) {
                    Ok(x) => {
                        bytes_written += writer.write(&x)?;
                    }
                    Err(s) => {
                        return Err(io::Error::new(io::ErrorKind::Other, s));
                    }
                }
            }
        };
    }
    let _ = bytes_read;
    Ok(bytes_written)
}

pub fn squash_block(plaintext: &[u8]) -> Vec<u8> {
    let bwt_encoded = bw_transform(plaintext);
    let mtf_encoded = mtf_transform(&bwt_encoded.block);
    let rle_encoded = run_length_encode(&mtf_encoded);
    let front_matter = create_front_matter(
        rle_encoded.len().try_into().unwrap(),
        bwt_encoded.end_index.try_into().unwrap(),
    );
    pack_arithmetic(
        front_matter,
        &rle_encoded,
        |x| match x {
            RunEncoded::Byte(n) => u32::from(*n),
            RunEncoded::ZeroRun(Bijective::A) => 0,
            RunEncoded::ZeroRun(Bijective::B) => 256,
        },
        257,
    )
}

pub fn unsquash_block(cyphertext: &[u8]) -> Result<Vec<u8>, &'static str> {
    let (body, front_matter) = get_front_matter(cyphertext)?;
    let arith_decoded = unpack_arithmetic(
        body,
        |x| match x {
            0 => RunEncoded::ZeroRun(Bijective::A),
            256 => RunEncoded::ZeroRun(Bijective::B),
            n => RunEncoded::Byte(u8::try_from(n).unwrap()),
        },
        257,
        front_matter.length.try_into().unwrap(),
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
    length: u32,
    end_index: u32,
}

fn create_front_matter(length: u32, end_index: u32) -> Vec<u8> {
    let mut front_matter: Vec<u8> = Vec::with_capacity(8);
    front_matter.extend_from_slice(&end_index.to_le_bytes()[..]);
    front_matter.extend_from_slice(&length.to_le_bytes()[..]);
    front_matter
}

fn get_front_matter(body: &[u8]) -> Result<(&[u8], FrontMatter), &'static str> {
    if body.len() < 8 {
        return Err("too short");
    }
    let mut staging = [0; 4];
    staging.copy_from_slice(&body[0..4]);
    let end_index = u32::from_le_bytes(staging).try_into().unwrap();
    staging.copy_from_slice(&body[4..8]);
    let length = u32::from_le_bytes(staging).try_into().unwrap();
    Ok((&body[8..], FrontMatter { length, end_index }))
}

#[cfg(test)]
mod test {
    use super::test_text::*;
    use super::*;

    #[test]
    fn e2e_test() {
        let plaintext = TEXT.as_bytes();
        let squashed = squash_block(plaintext);
        let unsquashed = unsquash_block(&squashed).unwrap();
        assert_eq!(
            String::from_utf8_lossy(plaintext),
            String::from_utf8_lossy(&unsquashed[..])
        );
    }

    #[test]
    fn exploded_squash_test() {
        let bwt_encoded = bw_transform(TEXT.as_bytes());
        let mtf_encoded = mtf_transform(&bwt_encoded.block);
        let rle_encoded = run_length_encode(&mtf_encoded);
        let front_matter = create_front_matter(
            rle_encoded.len().try_into().unwrap(),
            bwt_encoded.end_index.try_into().unwrap(),
        );
        let arith_encoded = pack_arithmetic(
            front_matter,
            &rle_encoded,
            |x| match x {
                RunEncoded::Byte(n) => u32::from(*n),
                RunEncoded::ZeroRun(Bijective::A) => 0,
                RunEncoded::ZeroRun(Bijective::B) => 256,
            },
            257,
        );

        let (body, front_matter) = get_front_matter(&arith_encoded).unwrap();
        let arith_decoded = unpack_arithmetic(
            body,
            |x| match x {
                0 => RunEncoded::ZeroRun(Bijective::A),
                256 => RunEncoded::ZeroRun(Bijective::B),
                n => RunEncoded::Byte(u8::try_from(n).unwrap()),
            },
            257,
            front_matter.length.try_into().unwrap(),
        );
        let rle_decoded = run_length_decode(&arith_decoded);
        let mtf_decoded = mtf_untransform(&rle_decoded);
        assert_eq!(arith_decoded, rle_encoded);
        assert_eq!(rle_decoded.len(), mtf_encoded.len());
        assert_eq!(rle_decoded, mtf_encoded);
        assert_eq!(mtf_decoded, bwt_encoded.block);
        let bw_decoded = bw_untransform(&BwVec {
            block: mtf_decoded,
            end_index: front_matter.end_index,
        });
        assert_eq!(bwt_encoded.end_index, front_matter.end_index);
        assert_eq!(rle_encoded.len(), front_matter.length.try_into().unwrap());
        assert_eq!(String::from_utf8_lossy(&bw_decoded), TEXT);
    }

    #[test]
    fn front_matter() {
        let len = 352_354_634;
        let e_i = 1_112_323_534;
        let mut block = create_front_matter(len, e_i);
        block.append(&mut vec![1, 2, 3]);
        let (body, f_m) = get_front_matter(&block).unwrap();
        assert_eq!(f_m.length, len);
        assert_eq!(f_m.end_index, e_i);
        assert_eq!(&body, &[1, 2, 3]);
    }
}
