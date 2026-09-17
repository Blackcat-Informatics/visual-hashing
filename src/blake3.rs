// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A one-shot BLAKE3 extendable-output function.
//!
//! This crate needs exactly one thing from BLAKE3: the XOF stream of a byte
//! slice it already holds in full — in practice nine bytes of it, derived from
//! a 32-byte public key. That is a small enough slice of the function to carry
//! in-tree, and carrying it keeps `visual-hashing` at zero runtime
//! dependencies.
//!
//! What this is: unkeyed hashing of a whole input, with extendable output,
//! portable and allocation-free. What it deliberately is not: the keyed and
//! `derive_key` modes, an incremental `update` API, SIMD, or multithreading.
//!
//! Dropping the incremental API is what makes it small. Upstream carries a
//! chunk state, a 54-deep chaining-value stack and block bookkeeping because
//! `update` may be handed any prefix of the input; a caller that supplies the
//! whole slice at once lets all of that collapse into one recursive descent
//! over the subtree structure.
//!
//! Correctness is checked, not assumed. The unit tests below pin published
//! known-answer vectors, and `tests/blake3_equivalence.rs` diffs this
//! implementation against the upstream `blake3` crate across every block,
//! chunk and subtree boundary.

const BLOCK_LEN: usize = 64;
const CHUNK_LEN: usize = 1024;

const CHUNK_START: u32 = 1 << 0;
const CHUNK_END: u32 = 1 << 1;
const PARENT: u32 = 1 << 2;
const ROOT: u32 = 1 << 3;

const IV: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

const MSG_PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

/// The quarter-round mixing function.
#[allow(clippy::too_many_arguments)]
fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
    state[d] = (state[d] ^ state[a]).rotate_right(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(12);
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
    state[d] = (state[d] ^ state[a]).rotate_right(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(7);
}

/// One round: mix the four columns, then the four diagonals.
fn round(state: &mut [u32; 16], m: &[u32; 16]) {
    g(state, 0, 4, 8, 12, m[0], m[1]);
    g(state, 1, 5, 9, 13, m[2], m[3]);
    g(state, 2, 6, 10, 14, m[4], m[5]);
    g(state, 3, 7, 11, 15, m[6], m[7]);
    g(state, 0, 5, 10, 15, m[8], m[9]);
    g(state, 1, 6, 11, 12, m[10], m[11]);
    g(state, 2, 7, 8, 13, m[12], m[13]);
    g(state, 3, 4, 9, 14, m[14], m[15]);
}

fn permute(m: &mut [u32; 16]) {
    let mut permuted = [0u32; 16];
    for (dst, &src) in permuted.iter_mut().zip(MSG_PERMUTATION.iter()) {
        *dst = m[src];
    }
    *m = permuted;
}

/// The compression function: seven rounds over a 16-word state, then feed-forward.
fn compress(
    cv: &[u32; 8],
    block: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let mut state = [
        cv[0],
        cv[1],
        cv[2],
        cv[3],
        cv[4],
        cv[5],
        cv[6],
        cv[7],
        IV[0],
        IV[1],
        IV[2],
        IV[3],
        counter as u32,
        (counter >> 32) as u32,
        block_len,
        flags,
    ];
    let mut block = *block;

    round(&mut state, &block);
    for _ in 0..6 {
        permute(&mut block);
        round(&mut state, &block);
    }

    for i in 0..8 {
        state[i] ^= state[i + 8];
        state[i + 8] ^= cv[i];
    }
    state
}

fn words_from_le_bytes(block: &[u8; BLOCK_LEN]) -> [u32; 16] {
    let mut words = [0u32; 16];
    for (word, bytes) in words.iter_mut().zip(block.chunks_exact(4)) {
        *word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    }
    words
}

/// The final compression of a chunk or parent node, held un-run.
///
/// A node's last compression has two possible readings — a chaining value fed
/// to its parent, or the root of the whole tree — and which one applies is not
/// known until the caller decides. Keeping the inputs lets both be taken.
struct Output {
    cv: [u32; 8],
    block: [u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
}

impl Output {
    fn chaining_value(&self) -> [u32; 8] {
        let state = compress(
            &self.cv,
            &self.block,
            self.counter,
            self.block_len,
            self.flags,
        );
        let mut cv = [0u32; 8];
        cv.copy_from_slice(&state[..8]);
        cv
    }

    /// Fill `out` with the root extendable output.
    ///
    /// Each compression yields one 64-byte output block; the block index takes
    /// the place of the node counter, which is how BLAKE3 extends past 64
    /// bytes.
    fn root_bytes(&self, out: &mut [u8]) {
        for (index, slot) in out.chunks_mut(BLOCK_LEN).enumerate() {
            let words = compress(
                &self.cv,
                &self.block,
                index as u64,
                self.block_len,
                self.flags | ROOT,
            );
            for (dst, word) in slot.chunks_mut(4).zip(words.iter()) {
                let bytes = word.to_le_bytes();
                dst.copy_from_slice(&bytes[..dst.len()]);
            }
        }
    }
}

/// Compress one chunk (at most [`CHUNK_LEN`] bytes) into its final output.
fn chunk_output(chunk: &[u8], chunk_counter: u64) -> Output {
    debug_assert!(chunk.len() <= CHUNK_LEN);

    let mut cv = IV;
    let mut compressed = 0usize;
    let mut rest = chunk;

    // Every block but the last is absorbed here; the last is left to `Output`
    // because only the caller knows whether this chunk is the root.
    while rest.len() > BLOCK_LEN {
        let (block, tail) = rest.split_at(BLOCK_LEN);
        let mut bytes = [0u8; BLOCK_LEN];
        bytes.copy_from_slice(block);
        let flags = if compressed == 0 { CHUNK_START } else { 0 };
        let state = compress(
            &cv,
            &words_from_le_bytes(&bytes),
            chunk_counter,
            BLOCK_LEN as u32,
            flags,
        );
        cv.copy_from_slice(&state[..8]);
        compressed += 1;
        rest = tail;
    }

    let mut bytes = [0u8; BLOCK_LEN];
    bytes[..rest.len()].copy_from_slice(rest);
    Output {
        cv,
        block: words_from_le_bytes(&bytes),
        counter: chunk_counter,
        block_len: rest.len() as u32,
        flags: CHUNK_END | if compressed == 0 { CHUNK_START } else { 0 },
    }
}

fn parent_output(left: &[u32; 8], right: &[u32; 8]) -> Output {
    let mut block = [0u32; 16];
    block[..8].copy_from_slice(left);
    block[8..].copy_from_slice(right);
    Output {
        cv: IV,
        block,
        counter: 0,
        block_len: BLOCK_LEN as u32,
        flags: PARENT,
    }
}

/// The number of bytes belonging to the left subtree of a multi-chunk input.
///
/// BLAKE3's tree is left-full: the left side takes the largest power-of-two
/// number of chunks that still leaves at least one byte on the right.
fn left_len(content_len: usize) -> usize {
    let full_chunks = (content_len - 1) / CHUNK_LEN;
    (((full_chunks / 2) + 1).next_power_of_two()) * CHUNK_LEN
}

/// Reduce `input` to the output of the subtree rooted at `chunk_counter`.
///
/// Depth is `log2(len / CHUNK_LEN)` — at most 54 frames for a 2^64-byte input.
fn subtree_output(input: &[u8], chunk_counter: u64) -> Output {
    if input.len() <= CHUNK_LEN {
        return chunk_output(input, chunk_counter);
    }
    let split = left_len(input.len());
    let (left, right) = input.split_at(split);
    let left_cv = subtree_output(left, chunk_counter).chaining_value();
    let right_cv =
        subtree_output(right, chunk_counter + (split / CHUNK_LEN) as u64).chaining_value();
    parent_output(&left_cv, &right_cv)
}

/// Fill `out` with the unkeyed BLAKE3 extendable output of `data`.
pub(crate) fn hash_xof(data: &[u8], out: &mut [u8]) {
    subtree_output(data, 0).root_bytes(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The input pattern used by the published BLAKE3 test vectors: the bytes
    /// 0, 1, 2, … 250, repeating.
    fn pattern(len: usize) -> alloc::vec::Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    fn hex(bytes: &[u8]) -> alloc::string::String {
        use core::fmt::Write as _;
        let mut s = alloc::string::String::new();
        for b in bytes {
            let _ = write!(s, "{b:02x}");
        }
        s
    }

    /// Known answers from the BLAKE3 reference test vectors, chosen to cross
    /// every structural boundary: empty, sub-block, exact block, sub-chunk,
    /// exact chunk, and the first three subtree splits.
    ///
    /// These are independent of the `blake3` dev-dependency on purpose — if
    /// both this code and the oracle were wrong in the same way, only a fixed
    /// external answer would notice.
    #[test]
    fn published_known_answers() {
        const CASES: &[(usize, &str)] = &[
            (
                0,
                "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
            ),
            (
                1,
                "2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213",
            ),
            (
                63,
                "e9bc37a594daad83be9470df7f7b3798297c3d834ce80ba85d6e207627b7db7b",
            ),
            (
                64,
                "4eed7141ea4a5cd4b788606bd23f46e212af9cacebacdc7d1f4c6dc7f2511b98",
            ),
            (
                1023,
                "10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11",
            ),
            (
                1024,
                "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7",
            ),
            (
                1025,
                "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444",
            ),
            (
                2048,
                "e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a",
            ),
            (
                2049,
                "5f4d72f40d7a5f82b15ca2b2e44b1de3c2ef86c426c95c1af0b6879522563030",
            ),
            (
                3072,
                "b98cb0ff3623be03326b373de6b9095218513e64f1ee2edd2525c7ad1e5cffd2",
            ),
        ];

        for &(len, want) in CASES {
            let mut got = [0u8; 32];
            hash_xof(&pattern(len), &mut got);
            assert_eq!(hex(&got), want, "input_len {len}");
        }
    }

    /// 131 bytes of output spans three root blocks, so this pins the output
    /// counter as well as the digest — a 64-byte-only test would not.
    #[test]
    fn published_extended_output() {
        const CASES: &[(usize, &str)] = &[
            (
                0,
                "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262\
                 e00f03e7b69af26b7faaf09fcd333050338ddfe085b8cc869ca98b206c08243a\
                 26f5487789e8f660afe6c99ef9e0c52b92e7393024a80459cf91f476f9ffdbda\
                 7001c22e159b402631f277ca96f2defdf1078282314e763699a31c5363165421\
                 cce14d",
            ),
            (
                1025,
                "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444\
                 f4c4a22b4b399155358a994e52bf255de60035742ec71bd08ac275a1b51cc6bf\
                 e332b0ef84b409108cda080e6269ed4b3e2c3f7d722aa4cdc98d16deb554e562\
                 7be8f955c98e1d5f9565a9194cad0c4285f93700062d9595adb992ae68ff1280\
                 0ab67a",
            ),
        ];

        for &(len, want) in CASES {
            let mut got = [0u8; 131];
            hash_xof(&pattern(len), &mut got);
            assert_eq!(hex(&got), want, "input_len {len}");
        }
    }

    #[test]
    fn left_len_is_left_full() {
        assert_eq!(left_len(CHUNK_LEN + 1), CHUNK_LEN);
        assert_eq!(left_len(2 * CHUNK_LEN), CHUNK_LEN);
        assert_eq!(left_len(2 * CHUNK_LEN + 1), 2 * CHUNK_LEN);
        assert_eq!(left_len(4 * CHUNK_LEN), 2 * CHUNK_LEN);
        assert_eq!(left_len(4 * CHUNK_LEN + 1), 4 * CHUNK_LEN);
    }

    /// The output length must not change the bytes that shorter reads saw.
    #[test]
    fn output_is_a_prefix_stream() {
        let data = pattern(5000);
        let mut long = [0u8; 200];
        hash_xof(&data, &mut long);
        for n in [1usize, 7, 31, 32, 63, 64, 65, 127, 128, 129, 200] {
            let mut short = alloc::vec![0u8; n];
            hash_xof(&data, &mut short);
            assert_eq!(short[..], long[..n], "output_len {n}");
        }
    }
}
