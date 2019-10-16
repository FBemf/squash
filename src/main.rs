#![warn(clippy::all)]

fn main() {
    let plaintext = b"busy busy boys play all busy day";
    let encoded = run_length_encode(&mtf_transform(&bw_transform(plaintext)));
    println!("{:?}", encoded);
    let decooded = mtf_untransform(&run_length_decode(&c));
}

fn bw_transform(plaintext: &[u8]) -> Vec<u8> {
    let mut arr = Vec::with_capacity(plaintext.len());
    let mut arr2 = Vec::with_capacity(plaintext.len());
    for i in 0..plaintext.len() {
        let a = &plaintext[0..i];
        let b = &plaintext[i..plaintext.len()];
        arr.push([b, a].concat());
    }
    arr.sort();
    let end = 0;
    for item in &arr {
        arr2.push(item[plaintext.len() - 1]);
        if item == &plaintext {
            end = arr2.len() - 1;
        }
    }
    arr2
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

#[derive(PartialEq, Debug)]
struct Run {
    byte: u8,
    length: u8,
}

fn run_length_encode(plaintext: &[u8]) -> Vec<Run> {
    let mut out = Vec::with_capacity(plaintext.len());
    if !plaintext.is_empty() {
        out.push(Run {
            byte: plaintext[0],
            length: 1,
        });
        for item in &plaintext[1..] {
            if item == &out.last().unwrap().byte {
                let last_length = out.last().unwrap().length;
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

fn run_length_decode(cyphertext: Vec<Run>) -> Vec<u8> {
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

#[test]
fn bwt_test() {
    assert_eq!(
        bw_transform(b"abcdabcdefghefgh"),
        [104, 100, 97, 97, 98, 98, 99, 99, 104, 100, 101, 101, 102, 102, 103, 103]
    );
    assert_eq!(&bw_transform(b"toblerone bars"), b"eb onlbotrears");
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
        ]
    );
    assert_eq!(
        run_length_decode(vec![
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
