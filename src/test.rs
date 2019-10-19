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
fn run_encode_test() {
    assert_eq!(to_bijective(6), &[RunEncode::RunB, RunEncode::RunB]);
    assert_eq!(
        to_bijective(7),
        &[RunEncode::RunA, RunEncode::RunA, RunEncode::RunA]
    );
    assert_eq!(to_bijective(1), &[RunEncode::RunA]);
    assert_eq!(
        to_bijective(23),
        &[
            RunEncode::RunA,
            RunEncode::RunA,
            RunEncode::RunA,
            RunEncode::RunB
        ]
    );

    assert_eq!(6, from_bijective(&[RunEncode::RunB, RunEncode::RunB]));
    assert_eq!(
        7,
        from_bijective(&[RunEncode::RunA, RunEncode::RunA, RunEncode::RunA])
    );
    assert_eq!(1, from_bijective(&[RunEncode::RunA]));
    assert_eq!(
        23,
        from_bijective(&[
            RunEncode::RunA,
            RunEncode::RunA,
            RunEncode::RunA,
            RunEncode::RunB
        ])
    );
}

#[test]
fn squash_exploded_test() {
    let plaintext = TEXT.as_bytes();
    let bwt_encoded = bw_transform(plaintext);
    let mtf_encoded = mtf_transform(&bwt_encoded.block);
    let rle_encoded = run_length_encode(&mtf_encoded);
    let packed = bit_pack(&rle_encoded, bwt_encoded.end_index);
    match bit_unpack(&packed) {
        Ok((unpacked, end_index)) => {
            assert_eq!(end_index, bwt_encoded.end_index);
            assert_eq!(unpacked, rle_encoded);
            let rle_decoded = run_length_decode(&unpacked);
            assert_eq!(rle_decoded, mtf_encoded);
            let mtf_decoded = mtf_untransform(&rle_decoded);
            assert_eq!(mtf_decoded, bwt_encoded.block);
            let plaintext2 = bw_untransform(&BwVec {
                block: mtf_decoded,
                end_index: end_index,
            });
            assert_eq!(plaintext, &plaintext2[..]);
            println!(
                "Compression ratio of {}",
                packed.len() as f32 / plaintext.len() as f32
            );
        }
        Err(s) => {
            panic!(s);
        }
    };
    return ();
}

#[test]
fn packers_test() {
    let mut p = Packer::from_vec(vec![]);
    p.push(5, 5);
    p.push(7, 8);
    p.push(2, 2);
    let f = p.finish();
    println!("product: {:?}", f);
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
    assert_eq!(plaintext, &unsquashed[..]);
}

#[test]
fn bwt_test() {
    assert_eq!(
        bw_transform(b"abcdabcdefghefgh"),
        BwVec {
            block: vec![104, 100, 97, 97, 98, 98, 99, 99, 104, 100, 101, 101, 102, 102, 103, 103],
            end_index: 0,
        }
    );
    assert_eq!(
        bw_transform(b"toblerone bars"),
        BwVec {
            block: Vec::from(&b"eb onlbotrears"[..]),
            end_index: 13,
        }
    );
    assert_eq!(
        b"abcdabcdefghefgh",
        &bw_untransform(&BwVec {
            block: vec![104, 100, 97, 97, 98, 98, 99, 99, 104, 100, 101, 101, 102, 102, 103, 103],
            end_index: 0,
        })[..]
    );
    assert_eq!(
        b"toblerone bars",
        &bw_untransform(&BwVec {
            block: Vec::from(&b"eb onlbotrears"[..]),
            end_index: 13,
        })[..]
    );
}

#[test]
fn mtf_test() {
    assert_eq!(
        mtf_transform(b"aaaaabbbbbcccccddddd"),
        [97, 0, 0, 0, 0, 98, 0, 0, 0, 0, 99, 0, 0, 0, 0, 100, 0, 0, 0, 0]
    );
    assert_eq!(
        mtf_transform(b"syllogism"),
        [115, 121, 110, 0, 113, 107, 109, 5, 112]
    );
    assert_eq!(
        mtf_untransform(&[97, 0, 0, 0, 0, 98, 0, 0, 0, 0, 99, 0, 0, 0, 0, 100, 0, 0, 0, 0]),
        b"aaaaabbbbbcccccddddd"
    );
    assert_eq!(
        mtf_untransform(&[115, 121, 110, 0, 113, 107, 109, 5, 112]),
        b"syllogism"
    );
    assert_eq!(mtf_transform(&[]), &[]);
    assert_eq!(mtf_untransform(&[]), &[]);
}

#[test]
fn rle_test() {
    assert_eq!(
        run_length_encode(b"bbfdddeejreewwwer"),
        [
            Run {
                byte: 98,
                length: 1
            },
            Run {
                byte: 98,
                length: 1
            },
            Run {
                byte: 102,
                length: 1
            },
            Run {
                byte: 100,
                length: 1
            },
            Run {
                byte: 100,
                length: 1
            },
            Run {
                byte: 100,
                length: 1
            },
            Run {
                byte: 101,
                length: 1
            },
            Run {
                byte: 101,
                length: 1
            },
            Run {
                byte: 106,
                length: 1
            },
            Run {
                byte: 114,
                length: 1
            },
            Run {
                byte: 101,
                length: 1
            },
            Run {
                byte: 101,
                length: 1
            },
            Run {
                byte: 119,
                length: 1
            },
            Run {
                byte: 119,
                length: 1
            },
            Run {
                byte: 119,
                length: 1
            },
            Run {
                byte: 101,
                length: 1
            },
            Run {
                byte: 114,
                length: 1
            }
        ]
    );
    assert_eq!(
        run_length_decode(&vec![
            Run {
                byte: 98,
                length: 2
            },
            Run {
                byte: 102,
                length: 1
            },
            Run {
                byte: 100,
                length: 3
            },
            Run {
                byte: 101,
                length: 2
            },
            Run {
                byte: 106,
                length: 1
            },
            Run {
                byte: 114,
                length: 1
            },
            Run {
                byte: 101,
                length: 2
            },
            Run {
                byte: 119,
                length: 3
            },
            Run {
                byte: 101,
                length: 1
            },
            Run {
                byte: 114,
                length: 1
            }
        ]),
        b"bbfdddeejreewwwer"
    );
    assert_eq!(run_length_encode(b""), []);
}
