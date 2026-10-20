// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements. See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership. The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License. You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. See the License for the
// specific language governing permissions and limitations
// under the License.

use super::{Codec, ZSTDCodec, ZstdLevel};

fn compare(frame: &[u8], capacity: usize, prefix: usize) {
    let mut expected = vec![91; prefix];
    let decoded = zstd::bulk::decompress(frame, capacity);
    if let Ok(bytes) = &decoded {
        expected.extend_from_slice(bytes);
    }
    for spare in [0, 131200] {
        let mut actual = Vec::with_capacity(prefix + spare);
        actual.resize(prefix, 91);
        let result =
            ZSTDCodec::new(ZstdLevel::default()).decompress(frame, &mut actual, Some(capacity));
        assert_eq!(result.is_err(), decoded.is_err());
        assert_eq!(actual, expected);
        if let (Ok(count), Ok(bytes)) = (result, &decoded) {
            assert_eq!(count, bytes.len());
        }
    }
}

#[test]
fn preserves_prefix_and_bounded_decode_results() {
    for seed in 0..32u64 {
        let mut state = seed + 1;
        for len in [0usize, 1, 63, 4096, 131072] {
            let data = (0..len)
                .map(|i| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    match seed % 3 {
                        0 => (i / 8) as u8,
                        1 => (i % 7) as u8,
                        _ => state as u8,
                    }
                })
                .collect::<Vec<_>>();
            let mut compressor = zstd::bulk::Compressor::new(1).unwrap();
            compressor.include_checksum(seed & 1 != 0).unwrap();
            let mut frame = compressor.compress(&data).unwrap();
            for prefix in [0, 17] {
                for capacity in [len.saturating_sub(1), len, len + 128] {
                    compare(&frame, capacity, prefix);
                }
                for cut in [0, 1, frame.len() / 2, frame.len() - 1] {
                    compare(&frame[..cut], len, prefix);
                }
                for at in [0, frame.len() / 2, frame.len() - 1] {
                    frame[at] ^= 255;
                    compare(&frame, len, prefix);
                    frame[at] ^= 255;
                }
            }
        }
    }
}
