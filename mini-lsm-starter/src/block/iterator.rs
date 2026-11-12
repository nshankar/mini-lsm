// Copyright (c) 2022-2026 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use crate::key::{KeySlice, KeyVec};

use super::Block;

/// Iterates on a block.
pub struct BlockIterator {
    /// The internal `Block`, wrapped by an `Arc`
    block: Arc<Block>,
    /// The current key, empty represents the iterator is invalid
    key: KeyVec,
    /// the current value range in the block.data, corresponds to the current key
    value_range: (usize, usize),
    /// Current index of the key-value pair, should be in range of [0, num_of_elements)
    idx: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        let mut iter = Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        };
        // Position on entry 0 to capture the block's first key; stays empty for an empty block.
        iter.seek_to_first();
        iter.first_key = iter.key.clone();
        iter
    }

    /// Creates a block iterator and seek to the first entry.
    pub fn create_and_seek_to_first(block: Arc<Block>) -> Self {
        let mut iter = Self::new(block);
        iter.seek_to_first();
        iter
    }

    /// Creates a block iterator and seek to the first key that >= `key`.
    pub fn create_and_seek_to_key(block: Arc<Block>, key: KeySlice) -> Self {
        let mut iter = Self::new(block);
        iter.seek_to_key(key);
        iter
    }

    /// Returns the key of the current entry.
    pub fn key(&self) -> KeySlice<'_> {
        self.key.as_key_slice()
    }

    /// Returns the value of the current entry.
    pub fn value(&self) -> &[u8] {
        &self.block.data[self.value_range.0..self.value_range.1]
    }

    /// Returns true if the iterator is valid.
    /// Note: You may want to make use of `key`
    pub fn is_valid(&self) -> bool {
        !self.key.is_empty()
    }

    fn _seek_to_key_idx(&mut self, idx: usize) {
        let Some(&offset) = self.block.offsets.get(idx) else {
            // mark invalid
            self.key = KeyVec::new();
            return;
        };
        let offset = offset as usize;
        const U16_SIZE: usize = std::mem::size_of::<u16>();

        // Entry layout: overlap (u16) | rest_len (u16) | rest | value_len (u16) | value
        let overlap_end = offset + U16_SIZE;
        let rest_start = overlap_end + U16_SIZE;
        let overlap =
            u16::from_be_bytes(self.block.data[offset..overlap_end].try_into().unwrap()) as usize;
        let rest_len =
            u16::from_be_bytes(self.block.data[overlap_end..rest_start].try_into().unwrap())
                as usize;
        let rest_end = rest_start + rest_len;

        let mut key_bytes = Vec::with_capacity(overlap + rest_len);
        key_bytes.extend_from_slice(&self.first_key.raw_ref()[..overlap]);
        key_bytes.extend_from_slice(&self.block.data[rest_start..rest_end]);
        let key = KeyVec::from_vec(key_bytes);

        let value_start = rest_end + U16_SIZE;
        let value_len =
            u16::from_be_bytes(self.block.data[rest_end..value_start].try_into().unwrap()) as usize;
        let value_range = (value_start, value_start + value_len);

        (self.idx, self.key, self.value_range) = (idx, key, value_range);
    }

    /// Seeks to the first key in the block.
    pub fn seek_to_first(&mut self) {
        self._seek_to_key_idx(0);
    }

    pub fn next(&mut self) {
        self._seek_to_key_idx(self.idx + 1);
    }

    /// Seek to the first key that >= `key`.
    /// Note: You should assume the key-value pairs in the block are sorted when being added by
    /// callers.
    pub fn seek_to_key(&mut self, key: KeySlice) {
        let mut left = 0;
        let mut right = self.block.offsets.len();

        // assumes no duplicates
        while left < right {
            let idx = left + (right - left) / 2;
            self._seek_to_key_idx(idx);
            if self.key() < key {
                left = idx + 1;
            } else {
                right = idx;
            }
        }
        self._seek_to_key_idx(left)
    }
}
