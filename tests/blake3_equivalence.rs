// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Proof that the in-tree BLAKE3 agrees with the upstream implementation.
//!
//! `visual-hashing` carries its own one-shot BLAKE3 so that it ships with no
//! runtime dependencies (see `src/blake3.rs`). The `blake3` crate is kept as a
//! dev-dependency purely as the oracle for this file — it is never built by
//! consumers of the library.
//!
//! The comparison runs through the public API rather than the private hash, so
//! it covers the 6-bit slicing in `emoji_indices` as well as the digest.

use visual_hashing::emoji_indices;

/// SplitMix64, so the sweep is wide but exactly reproducible on every run and
/// every platform — a failing case can be re-examined without hunting a seed.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            out.extend_from_slice(&self.next_u64().to_le_bytes());
        }
        out.truncate(len);
        out
    }
}

/// What `emoji_indices` must produce, computed from the upstream digest.
fn oracle_indices(data: &[u8], length: usize) -> Vec<usize> {
    let wanted = length.max(1);
    let mut digest = vec![0u8; (wanted * 6).div_ceil(8)];
    blake3::Hasher::new()
        .update(data)
        .finalize_xof()
        .fill(&mut digest);

    let mut out = Vec::with_capacity(wanted);
    let mut acc: u64 = 0;
    let mut bits: u32 = 0;
    for byte in digest {
        acc = (acc << 8) | u64::from(byte);
        bits += 8;
        while bits >= 6 && out.len() < wanted {
            bits -= 6;
            out.push(((acc >> bits) & 0x3f) as usize);
        }
        acc &= (1u64 << bits) - 1;
    }
    out
}

/// Input lengths at every structural boundary: block (64), chunk (1024), and
/// the subtree splits where the recursive descent in `src/blake3.rs` divides.
const INPUT_LENS: &[usize] = &[
    0, 1, 2, 3, 31, 32, 63, 64, 65, 127, 128, 129, 1023, 1024, 1025, 2047, 2048, 2049, 3072, 4095,
    4096, 4097, 6144, 8192, 8193, 16384, 16385, 31337,
];

/// Output lengths chosen so the digest crosses the 64-byte root-output block:
/// a `length` of 85 needs exactly 64 bytes, 86 needs 65, 170 needs 128.
const EMOJI_LENS: &[usize] = &[1, 2, 3, 11, 41, 42, 43, 84, 85, 86, 87, 169, 170, 171, 256];

#[test]
fn matches_upstream_across_every_boundary() {
    let mut rng = SplitMix64(0x5653_4841_5348_0001);

    for &input_len in INPUT_LENS {
        let data = rng.bytes(input_len);
        for &emoji_len in EMOJI_LENS {
            assert_eq!(
                emoji_indices(&data, emoji_len),
                oracle_indices(&data, emoji_len),
                "input_len {input_len}, emoji_len {emoji_len}"
            );
        }
    }
}

/// Structured inputs can expose carry and padding mistakes that random bytes
/// mask, so repeat the sweep over degenerate patterns.
#[test]
fn matches_upstream_for_degenerate_inputs() {
    for &input_len in INPUT_LENS {
        for fill in [0x00u8, 0xff, 0x55] {
            let data = vec![fill; input_len];
            for &emoji_len in [1usize, 11, 85, 86, 171].iter() {
                assert_eq!(
                    emoji_indices(&data, emoji_len),
                    oracle_indices(&data, emoji_len),
                    "fill {fill:#04x}, input_len {input_len}, emoji_len {emoji_len}"
                );
            }
        }
    }
}
