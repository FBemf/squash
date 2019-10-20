#![warn(clippy::all)]

use std::cmp::Ordering;

pub struct SuffixArray<'a> {
    _text: &'a [u8],
    array: Vec<usize>,
}

#[derive(Clone)]
struct Suffix {
    index: usize,
    rank: (i64, i64),
}

fn suffix_compare(a: &Suffix, b: &Suffix) -> Ordering {
    if a.rank.0 == b.rank.0 {
        a.rank.1.cmp(&b.rank.1)
    } else {
        a.rank.0.cmp(&b.rank.0)
    }
}

impl<'a> SuffixArray<'a> {
    pub fn from_array(body: &'a [u8]) -> SuffixArray {
        // special thanks to https://www.geeksforgeeks.org/suffix-array-set-2-a-nlognlogn-algorithm/
        let mut array: Vec<Suffix> = vec![
            Suffix {
                index: 0,
                rank: (0, 0)
            };
            body.len()
        ];
        for (i, v) in array.iter_mut().enumerate() {
            v.index = i;
            v.rank.0 = i64::from(body[i]);
            v.rank.1 = if i == body.len() - 1 {
                -1
            } else {
                i64::from(body[i + 1])
            }
        }
        array.push(Suffix {
            index: body.len(),
            rank: (-1, -2),
        });

        array.sort_by(suffix_compare);

        let mut indices: Vec<usize> = vec![0; body.len() + 1];
        let mut k = 4;
        loop {
            if k >= 2 * array.len() {
                break;
            }

            let mut rank = 0;
            let mut prev_rank = array[0].rank.0;
            array[0].rank.0 = rank;
            indices[array[0].index] = 0;

            for i in 0..array.len() {
                if array[i].rank.0 == prev_rank && array[i].rank.1 == array[i - 1].rank.1 {
                    prev_rank = array[i].rank.0;
                    array[i].rank.0 = rank;
                } else {
                    prev_rank = array[i].rank.0;
                    rank += 1;
                    array[i].rank.0 = rank;
                }
                indices[array[i].index] = i;
            }

            for i in 0..array.len() {
                let next_index = array[i].index + k / 2;
                array[i].rank.1 = if next_index < body.len() {
                    array[indices[next_index]].rank.0
                } else {
                    -1
                }
            }

            array.sort_by(suffix_compare);

            k *= 2;
        }

        SuffixArray {
            _text: body,
            array: array.iter().map(|a| a.index).collect(),
        }
    }
    pub fn raw(self) -> Vec<usize> {
        self.array
    }
    pub fn _fmt(&self) -> String {
        let mut out = String::new();
        out += &format!(
            "SUFFIX ARRAY FOR {}:\n",
            String::from_utf8_lossy(self._text)
        );
        for (i, line) in self.array.iter().enumerate() {
            out += &format!(
                "{}:\t'{}'\n",
                i,
                String::from_utf8_lossy(&self._text[*line..])
            );
        }
        out
    }
}
