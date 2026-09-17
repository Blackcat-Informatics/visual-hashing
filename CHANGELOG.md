<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->
# Changelog

All notable changes to `visual-hashing` are recorded here.

## [Unreleased]

## [1.0.0] - 2026-09-17

### Added

- **A third licence option: `MulanPSL-2.0`.** The crate is now offered under
  `MIT OR Apache-2.0 OR MulanPSL-2.0` — pick any one. Note that MulanPSL-2.0 is
  bilingual and its §6 makes the Chinese text controlling where the versions
  diverge; `LICENSING.md` covers what else differs between the three. The
  pinned bilingual text ships as `LICENSE-MULAN`, digest-verified in CI.
- `no_std` support. The library needs only `alloc` and builds for bare-metal
  targets (CI proves it on `thumbv7em-none-eabihf`) as well as `wasm32`.
- `scripts/check-licenses.py` and `deny.toml`, which turn this project's
  licensing and dependency claims into CI gates rather than assertions.

### Removed

- **Every runtime dependency.** `blake3` was the last one, and with it went a
  build script, a C-compiler requirement and nine transitive crates. The crate
  now carries the part of BLAKE3 it needs in `src/blake3.rs` — unkeyed,
  one-shot, extendable output, `#![forbid(unsafe_code)]`.

  Output is unchanged: the frozen conformance vectors pass byte-for-byte, and
  `tests/blake3_equivalence.rs` diffs the implementation against the upstream
  `blake3` crate (now a dev-only oracle) across every block, chunk and subtree
  boundary on each CI run.

  Being portable rather than SIMD, it favours auditability over throughput. If
  you hash large inputs in bulk, hash them with `blake3` directly and pass the
  digest in.

- The `serde_json` dev-dependency, replaced by a strict reader for the
  generated vector format.

### Stability

`1.0` makes the wire contract binding. The 64-emoji alphabet, the randomart
character ramp and the public API will not change: a fingerprint already
printed in a log, a CLI banner or a user's notes has to keep meaning what it
meant, so a change to either rendering would be a new crate rather than a new
major version.

### Note for existing users

`0.1.3 → 1.0.0` is a semver-incompatible jump, so a `visual-hashing = "0.1.3"`
requirement will **not** pick this up — for a `0.x` crate the minor slot is the
breaking one, giving `>=0.1.3, <0.2.0`. Update the requirement to
`visual-hashing = "1"` deliberately.

Nothing about the output changed: the public API is identical and every
rendering is byte-for-byte the same, as the frozen vectors demonstrate. What
changed is the shape of the crate around it — no runtime dependencies, no build
script, `no_std`, and a third licence option.

## [0.1.3] - 2026-06-22

### Changed

- Moved the canonical source repository to
  `https://github.com/Blackcat-Informatics/visual-hashing`.
- Included the emojihash and randomart conformance vectors in the source
  repository and package tests, so fresh checkouts run strict corpus checks.
- Added standalone CI, release automation, citation metadata, and project
  support files for crate-only maintenance.

## [0.1.2] - 2026-06-19

### Changed

- Last monorepo-published release from `Blackcat-Informatics/gmeow-gts`.
