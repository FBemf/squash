#![warn(clippy::all)]

use super::*;

static TEXT: &str =
    "When you create a closure, Rust infers which \
     trait to use based on how the closure uses the values from the environment. All \
     closures implement FnOnce because they can all be called at least once. Closures \
     that don't move the captured variables also implement FnMut, and closures that \
     don't need mutable access to the captured variables also implement Fn. In Listing \
     13-12, the equal_to_x closure borrows x immutably (so equal_to_x has the Fn trait \
     ) because the body of the closure only needs to read the value in x.\n\
     If you want to force the closure to take ownership of the values it uses in the \
     environment, you can use the move keyword before the parameter list. This technique \
     is mostly useful when passing a closure to a new thread to move the data so it's \
     owned by the new thread.\n\
     We'll have more examples of move closures in Chapter 16 when we talk about concurrency. \
     For now, here's the code from Listing 13-12 with the move keyword added to the \
     closure definition and using vectors instead of integers, because integers can \
     be copied rather than moved; note that this code will not yet compile.";

#[test]
fn exploded_squash_test() {
    let bwt_encoded = bw_transform(TEXT.as_bytes());
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
    let cyphertext = add_front_matter(&arith_encoded, rle_encoded.len(), bwt_encoded.end_index);

    let (body, front_matter) = get_front_matter(&cyphertext).unwrap();
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
    assert_eq!(body, &arith_encoded[..]);
    assert_eq!(arith_decoded, rle_encoded);
    assert_eq!(rle_decoded.len(), mtf_encoded.len());
    assert_eq!(rle_decoded, mtf_encoded);
    assert_eq!(mtf_decoded, bwt_encoded.block);
    let bw_decoded = bw_untransform(&BwVec {
        block: mtf_decoded,
        end_index: front_matter.end_index,
    });
    assert_eq!(bwt_encoded.end_index, front_matter.end_index);
    assert_eq!(rle_encoded.len(), front_matter.length);
    assert_eq!(String::from_utf8_lossy(&bw_decoded), TEXT);
}

#[test]
fn arithmetic_test() {
    assert_eq!(
        pack_arithmetic(b"ddabdaddabccda", |a| { u32::from(a - b"a"[0]) }, 4),
        &[143, 13, 36, 9]
    );
    assert_eq!(
        pack_arithmetic(b"abbadabbadc", |a| { u32::from(a - b"a"[0]) }, 4),
        &[40, 23, 40]
    );
    assert_eq!(
        &unpack_arithmetic(
            &[143, 13, 36, 9],
            |b| { u8::try_from(b).unwrap() + b"a"[0] },
            4,
            14
        )[..],
        &b"ddabdaddabccda"[..]
    );
    assert_eq!(
        &unpack_arithmetic(
            &[40, 23, 40],
            |b| { u8::try_from(b).unwrap() + b"a"[0] },
            4,
            11
        )[..],
        &b"abbadabbadc"[..]
    );
    let packed_text = pack_arithmetic(TEXT.as_bytes(), |a| u32::from(*a), 256);
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
fn run_encode_test() {
    let test = 6;
    let enc = to_bijective(test);
    assert_eq!(from_bijective(&enc), test);

    let test = 7;
    let enc = to_bijective(test);
    assert_eq!(from_bijective(&enc), test);

    let test = 23;
    let enc = to_bijective(test);
    assert_eq!(from_bijective(&enc), test);

    let test = 1;
    let enc = to_bijective(test);
    assert_eq!(from_bijective(&enc), test);
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

#[test]
fn e2e_test() {
    let plaintext = TEXT.as_bytes();
    let squashed = squash(plaintext);
    let unsquashed = unsquash(&squashed).unwrap();
    assert_eq!(
        String::from_utf8_lossy(plaintext),
        String::from_utf8_lossy(&unsquashed[..])
    );
}

#[test]
fn bwt_test() {
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
fn suffix_array_test() {
    let test = b"banana banana banana";
    let sa1 = SuffixArray::from_array(test);
    let sa2 = SuffixArray::from_array_naive(test);
    println!("{}\n{}", sa1.fmt(), sa2.fmt());
    assert_eq!(sa1.fmt(), sa2.fmt());
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
