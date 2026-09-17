<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# Contributing to visual-hashing

Thanks for your interest in `visual-hashing`.

This repository contains one Rust crate plus the conformance vectors that pin
its emojihash and randomart output. Behavior changes must update the generated
vectors and keep the Rust implementation byte-for-byte compatible with the
reference data.

Useful local checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build --lib --target wasm32-unknown-unknown
cargo build --lib --target thumbv7em-none-eabihf
cargo deny check
python3 scripts/check-licenses.py
cargo package --locked --list
cargo publish --locked --dry-run
```

The library must keep **zero runtime dependencies** — that claim is in the
README and is the reason the dependency licence surface stays small, so
`scripts/check-licenses.py` fails the build if one appears. `src/blake3.rs`
exists for the same reason; if you change it, `tests/blake3_equivalence.rs`
must still agree with the upstream `blake3` crate, and the frozen vectors must
still pass byte-for-byte.

The vector scripts live under `python/scripts/`. Regenerate vectors only when
the public rendering contract intentionally changes, then review the JSON diff.

Contributions are accepted under **Apache-2.0 OR MIT**.
