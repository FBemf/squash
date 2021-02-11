use std::convert::TryFrom;
use std::convert::TryInto;
use std::io;

use super::arithmetic::*;
use super::transforms::*;

const BLOCK_SIZE: usize = 1 << 18;
const MAGIC_NUMBER: u32 = 0xca55_e77e;
const FILETYPE_VERSION: u8 = 1;

pub fn squash(reader: &mut dyn io::Read, writer: &mut dyn io::Write) -> Result<(), io::Error> {
    writer.write_all(&MAGIC_NUMBER.to_le_bytes())?;
    writer.write_all(&FILETYPE_VERSION.to_le_bytes())?;

    let arithmetic_encoder = ArithmeticEncoder::default_encoder();
    arithmetic_encoder.write_config(writer)?;

    loop {
        let mut block = vec![0; BLOCK_SIZE];
        let bytes = reader.read(&mut block)?;
        match bytes {
            0 => {
                break;
            }
            n => {
                let squashed = squash_block(&block[0..n], &arithmetic_encoder);
                let squashed_len = u32::try_from(squashed.len()).unwrap().to_le_bytes();
                writer.write_all(&squashed_len)?;
                writer.write_all(&squashed)?;
            }
        };
    }
    Ok(())
}

pub fn unsquash(reader: &mut dyn io::Read, writer: &mut dyn io::Write) -> Result<(), io::Error> {
    let mut one_byte: [u8; 1] = [0; 1];
    let mut four_bytes: [u8; 4] = [0; 4];

    reader.read_exact(&mut four_bytes)?;
    let magic_number = u32::from_le_bytes(four_bytes);
    assert_eq!(magic_number, MAGIC_NUMBER);

    reader.read_exact(&mut one_byte)?;
    let version_number = u8::from_le_bytes(one_byte);
    assert_eq!(version_number, FILETYPE_VERSION);

    let arithmetic_encoder = ArithmeticEncoder::read_config(reader)?;

    loop {
        let block_len = match reader.read(&mut four_bytes)? {
            0 => {
                break;
            }
            4 => u32::from_le_bytes(four_bytes),
            _ => {
                panic!("expected something");
            }
        };
        let mut block = vec![0; block_len.try_into().unwrap()];
        match reader.read(&mut block)? {
            0 => {
                panic!("expected something");
            }
            _ => match unsquash_block(&block, &arithmetic_encoder) {
                Ok(x) => {
                    writer.write_all(&x)?;
                }
                Err(s) => {
                    return Err(io::Error::new(io::ErrorKind::Other, s));
                }
            },
        };
    }
    Ok(())
}

pub fn squash_block(plaintext: &[u8], arithmetic_encoder: &ArithmeticEncoder) -> Vec<u8> {
    let bwt_encoded = bw_transform(plaintext);
    let mtf_encoded = mtf_transform(&bwt_encoded.block);
    let rle_encoded = run_length_encode(&mtf_encoded);
    let front_matter =
        create_front_matter(rle_encoded.len().try_into().unwrap(), bwt_encoded.end_index);
    arithmetic_encoder.pack(
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

pub fn unsquash_block(
    ciphertext: &[u8],
    arithmetic_encoder: &ArithmeticEncoder,
) -> Result<Vec<u8>, &'static str> {
    let (body, front_matter) = get_front_matter(ciphertext)?;
    let arithmetic_decoded = arithmetic_encoder.unpack(
        body,
        |x| match x {
            0 => RunEncoded::ZeroRun(Bijective::A),
            256 => RunEncoded::ZeroRun(Bijective::B),
            n => RunEncoded::Byte(u8::try_from(n).unwrap()),
        },
        257,
        front_matter.length.try_into().unwrap(),
    );
    let rle_decoded = run_length_decode(&arithmetic_decoded);
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
    let end_index = u32::from_le_bytes(staging);
    staging.copy_from_slice(&body[4..8]);
    let length = u32::from_le_bytes(staging);
    Ok((&body[8..], FrontMatter { length, end_index }))
}

#[cfg(test)]
mod test {
    use super::*;

    const TEXT: &str = "When you create a closure, Rust infers which \
        trait to use based on how the closure uses the values from the environment. All \
        closures implement FnOnce because they can all be called at least once. Closures \
        that don't move the captured variables also implement FnMut, and closures that \
        don't need mutable access to the captured variables also implement Fn. In Listing \
        13-12, the equal_to_x closure borrows x immutably (so equal_to_x has the Fn trait \
        ) because the body of the closure only needs to read the value in x.\n\
        If you want to force the closure to take ownership of the values it uses in the \
        environment, you can use the move keyword before the parameter list. This technique \
        is mostly useful when passing a closure to a new thread to move the data so it's \
        owned by the new thread.\n";

    #[test]
    fn e2e_test() {
        let plaintext = TEXT.as_bytes();
        let arithmetic_encoder = ArithmeticEncoder::default_encoder();
        let squashed = squash_block(plaintext, &arithmetic_encoder);
        let unsquashed = unsquash_block(&squashed, &arithmetic_encoder).unwrap();
        assert_eq!(
            String::from_utf8_lossy(plaintext),
            String::from_utf8_lossy(&unsquashed[..])
        );
    }

    #[test]
    fn exploded_squash_test() {
        let arithmetic_encoder = ArithmeticEncoder::default_encoder();
        let bwt_encoded = bw_transform(TEXT.as_bytes());
        let mtf_encoded = mtf_transform(&bwt_encoded.block);
        let rle_encoded = run_length_encode(&mtf_encoded);
        let front_matter =
            create_front_matter(rle_encoded.len().try_into().unwrap(), bwt_encoded.end_index);
        let arith_encoded = arithmetic_encoder.pack(
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
        let arith_decoded = arithmetic_encoder.unpack(
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
