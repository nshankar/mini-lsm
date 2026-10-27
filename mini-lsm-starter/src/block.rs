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

mod builder;
mod iterator;

pub use builder::BlockBuilder;
use bytes::{BufMut, Bytes, BytesMut};
pub use iterator::BlockIterator;

/// A block is the smallest unit of read and caching in LSM tree. It is a collection of sorted key-value pairs.
pub struct Block {
    pub(crate) data: Vec<u8>,
    pub(crate) offsets: Vec<u16>,
}

impl Block {
    pub fn new(data: Vec<u8>, offsets: Vec<u16>) -> Self {
        assert!(
            offsets.len() <= u16::MAX as usize,
            "too many entries: {}",
            offsets.len()
        );
        if let Some(&first) = offsets.first() {
            assert!(first == 0, "first offset is {first}, expected 0");
        }
        for (i, w) in offsets.windows(2).enumerate() {
            assert!(
                w[0] <= w[1],
                "offset {} ({}) > offset {} ({})",
                i,
                w[0],
                i + 1,
                w[1]
            );
        }
        if let Some(&last) = offsets.last() {
            assert!(
                last as usize <= data.len(),
                "last offset {last} past data ({} bytes)",
                data.len()
            );
        }
        Block { data, offsets }
    }
    /// Encode the internal data to the data layout illustrated in the course
    /// Note: You may want to recheck if any of the expected field is missing from your output
    pub fn encode(&self) -> Bytes {
        // The 3 blocks: data, offsets, extra
        let total_size = self.data.len()
            + self.offsets.len() * std::mem::size_of::<u16>()
            + std::mem::size_of::<u16>();
        let mut buf = BytesMut::with_capacity(total_size);

        // all writes below are big endian
        buf.put_slice(&self.data);
        for &offset in &self.offsets {
            buf.put_u16(offset)
        }
        buf.put_u16(self.offsets.len() as u16);
        buf.freeze()
    }

    /// Decode from the data layout, transform the input `data` to a single `Block`
    pub fn decode(data: &[u8]) -> Self {
        const U16_SIZE: usize = std::mem::size_of::<u16>();
        let Some(n_elements_bytes) = data.last_chunk::<U16_SIZE>() else {
            panic!(
                "corrupt block: {} bytes, too short for element count",
                data.len()
            );
        };
        let n_elements = u16::from_be_bytes(*n_elements_bytes) as usize;

        let offsets_start = data.len() - (n_elements + 1) * U16_SIZE; // underflow?
        let offsets_end = data.len() - U16_SIZE;
        let offsets: Vec<u16> = data[offsets_start..offsets_end]
            .chunks_exact(U16_SIZE)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        let data = data[..offsets_start].to_vec();

        Block::new(data, offsets)
    }
}
