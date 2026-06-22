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
cargo package --locked --list
cargo publish --locked --dry-run
```

The vector scripts live under `python/scripts/`. Regenerate vectors only when
the public rendering contract intentionally changes, then review the JSON diff.

Contributions are accepted under **Apache-2.0 OR MIT**.
