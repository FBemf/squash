#![warn(clippy::all)]

static TEXT: &str =
    "When you create a closure, Rust infers which \
     trait to use based on how the closure uses the values from the environment. All \
     closures implement FnOnce because they can all be called at least once. Closures \
     that don't move the captured variables also implement FnMut, and closures that \
     don't need mutable access to the captured variables also implement Fn. In Listing \
     13-12, the equal_to_x closure borrows x immutably (so equal_to_x has the Fn trait\
     ) because the body of the closure only needs to read the value in x.\
     If you want to force the closure to take ownership of the values it uses in the \
     environment, you can use the move keyword before the parameter list. This technique \
     is mostly useful when passing a closure to a new thread to move the data so it'\
     s owned by the new thread.\
     We'll have more examples of move closures in Chapter 16 when we talk about concurrency.\
     For now, here's the code from Listing 13-12 with the move keyword added to the \
     closure definition and using vectors instead of integers, because integers can \
     be copied rather than moved; note that this code will not yet compile.";

fn main() {
    let plaintext = TEXT.as_bytes();
    let bwt = bw_transform(plaintext);
    let encoded = run_length_encode(&mtf_transform(&bwt.block));
    let squashed: Vec<u8> = encoded
        .iter()
        .flat_map(|a| vec![a.byte, a.length])
        .collect();
    println!("{:?}", &squashed);
    let mut unsquashed = vec![];
    for index in 0..squashed.len() / 2 {
        unsquashed.push(Run {
            byte: squashed[index * 2],
            length: squashed[index * 2 + 1],
        });
    }
    let decoded = bw_untransform(BwVec {
        block: mtf_untransform(&run_length_decode(&unsquashed)),
        end_index: bwt.end_index,
    });
    println!("{:?}", String::from_utf8_lossy(&decoded));
    println!(
        "Compression ratio of {}",
        squashed.len() as f32 / plaintext.len() as f32
    );
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
        /*let last_shift = [
            &plaintext[0..plaintext.len() - 1],
            &plaintext[plaintext.len() - 1..plaintext.len()],
        ]
        .concat();*/
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
        //out.push(cyphertext.block[next_index]);
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
    //out.reverse();
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

#[derive(PartialEq, Debug, Clone)]
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
        &bw_untransform(BwVec {
            block: vec![104, 100, 97, 97, 98, 98, 99, 99, 104, 100, 101, 101, 102, 102, 103, 103],
            end_index: 0,
        })[..]
    );
    assert_eq!(
        b"toblerone bars",
        &bw_untransform(BwVec {
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
