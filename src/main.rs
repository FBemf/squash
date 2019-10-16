#![warn(clippy::all)]

fn main() {
    let a = bw_transform(b"busy busy boys play all busy day");
    let b = mtf_transform(&a);
    println!("{:?}", b);
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
    for item in &arr {
        arr2.push(item[plaintext.len() - 1]);
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

#[test]
fn bwt_test() {
    assert_eq!(
        bw_transform(b"abcdabcdefghefgh"),
        [104, 100, 97, 97, 98, 98, 99, 99, 104, 100, 101, 101, 102, 102, 103, 103]
    );
    assert_eq!(
        String::from_utf8_lossy(&bw_transform(b"toblerone bars")),
        "eb onlbotrears"
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
}
