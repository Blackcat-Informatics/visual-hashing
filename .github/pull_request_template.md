<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
## Summary

- what changed
- why it changed

## Validation

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- [ ] `cargo build --lib --target wasm32-unknown-unknown`
- [ ] `cargo publish --locked --dry-run`
