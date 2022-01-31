// MIT License
//
// Copyright (c) 2020 Mathias Hall-Andersen
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Source: https://github.com/WireGuard/wireguard-rs/blob/7d84ef9064559a29b23ab86036f7ef62b450f90c/src/wireguard/router/anti_replay.rs

use core::mem;

// Implementation of RFC 6479.
// https://tools.ietf.org/html/rfc6479

#[cfg(target_pointer_width = "64")]
type Word = u64;
#[cfg(target_pointer_width = "64")]
const REDUNDANT_BIT_SHIFTS: usize = 6;

#[cfg(target_pointer_width = "32")]
type Word = u32;
#[cfg(target_pointer_width = "32")]
const REDUNDANT_BIT_SHIFTS: usize = 5;

const SIZE_OF_WORD: usize = mem::size_of::<Word>() * 8;

const BITMAP_BITLEN: usize = 512;
const BITMAP_WORDLEN: usize = BITMAP_BITLEN / SIZE_OF_WORD;
const BITMAP_INDEX_MASK: u64 = BITMAP_WORDLEN as u64 - 1;
const BITMAP_LOC_MASK: u64 = (SIZE_OF_WORD - 1) as u64;
const WINDOW_SIZE: u64 = (BITMAP_BITLEN - SIZE_OF_WORD) as u64;

pub struct AntiReplay {
    bitmap: [Word; BITMAP_WORDLEN],
    last: u64,
}

impl Default for AntiReplay {
    fn default() -> Self {
        AntiReplay::new()
    }
}

impl AntiReplay {
    pub fn new() -> Self {
        debug_assert_eq!(1 << REDUNDANT_BIT_SHIFTS, SIZE_OF_WORD);
        debug_assert_eq!(BITMAP_BITLEN % SIZE_OF_WORD, 0);

        AntiReplay {
            last: 0,
            bitmap: [0; BITMAP_WORDLEN],
        }
    }

    // Returns true if check is passed, i.e., not a replay or too old.
    //
    // Unlike RFC 6479, zero is allowed.
    fn is_valid(&self, seq: u64) -> bool {
        // Larger is always good.
        if seq > self.last {
            return true;
        }

        if self.last - seq > WINDOW_SIZE {
            return false;
        }

        let bit_location = seq & BITMAP_LOC_MASK;
        let index = (seq >> REDUNDANT_BIT_SHIFTS) & BITMAP_INDEX_MASK;

        self.bitmap[index as usize] & (1 << bit_location) == 0
    }

    // Should only be called if is_valid returns true.
    fn update_store(&mut self, seq: u64) {
        debug_assert!(self.is_valid(seq));

        let index = seq >> REDUNDANT_BIT_SHIFTS;

        if seq > self.last {
            let index_cur = self.last >> REDUNDANT_BIT_SHIFTS;
            let diff = index - index_cur;

            if diff >= BITMAP_WORDLEN as u64 {
                self.bitmap = [0; BITMAP_WORDLEN];
            } else {
                for i in 0 .. diff {
                    let real_index = (index_cur + i + 1) & BITMAP_INDEX_MASK;
                    self.bitmap[real_index as usize] = 0;
                }
            }

            self.last = seq;
        }

        let index = index & BITMAP_INDEX_MASK;
        let bit_location = seq & BITMAP_LOC_MASK;
        self.bitmap[index as usize] |= 1 << bit_location;
    }

    /// Checks and marks a sequence number in the replay filter
    ///
    /// # Arguments
    ///
    /// - seq: Sequence number check for replay and add to filter
    ///
    /// # Returns
    ///
    /// Ok(()) if sequence number is valid (not marked and not behind the moving window).
    /// Err if the sequence number is invalid (already marked or "too old").
    #[inline]
    pub fn update(&mut self, seq: u64) -> bool {
        if self.is_valid(seq) {
            self.update_store(seq);
            true
        } else {
            false
        }
    }
}

// TODO: add tests
