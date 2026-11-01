// SPDX-License-Identifier: AGPL-3.0-or-later
//! The index searches of the converted .rgr code (indexOf and friends) in
//! byte offsets, as Rust slices them, and the UTF-8 bytes of a string once
//! as a `Vec<int>`, cheap to index on every target.

use ranger::prelude::*;

pub struct PptxText {}

impl PptxText {
    pub fn utf8(text: &str) -> Vec<int> {
        let mut out: Vec<int> = Vec::new();
        for b in text.bytes() {
            out.push(b as int);
        }
        out
    }

    /// byte index of the first `key` in `text`, -1 when there is none
    pub fn index_of(text: &str, key: &str) -> int {
        if let Some(p) = text.find(key) {
            return p as int;
        }
        -1
    }

    /// byte index of the first `key` at or after `from`, -1 when none
    pub fn index_of_from(text: &str, key: &str, from: int) -> int {
        if from < 0 || from as usize > text.as_bytes().len() {
            return -1;
        }
        if let Some(p) = text[from as usize..].find(key) {
            return p as int + from;
        }
        -1
    }

    /// byte index of the last `key` in `text`, -1 when there is none
    pub fn last_index_of(text: &str, key: &str) -> int {
        let mut at: int = -1;
        let mut from: int = 0;
        loop {
            let p = PptxText::index_of_from(text, key, from);
            if p < 0 {
                return at;
            }
            at = p;
            from = p + 1;
        }
    }
}
