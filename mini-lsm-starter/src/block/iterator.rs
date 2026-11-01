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
    first_key: KeyVec, // TODO: why is this unused?
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        }
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
        let key_start = offset + std::mem::size_of::<u16>();

        let key_len =
            u16::from_be_bytes(self.block.data[offset..key_start].try_into().unwrap()) as usize;
        let key_end = key_start + key_len;
        let key = KeyVec::from_vec(self.block.data[key_start..key_end].to_vec());

        let value_start = key_end + std::mem::size_of::<u16>() as usize;
        let value_len = u16::from_be_bytes(
            self.block.data[key_start + key_len..value_start]
                .try_into()
                .unwrap(),
        ) as usize;
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
