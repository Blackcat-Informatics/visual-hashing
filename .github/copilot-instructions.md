<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->
# GitHub Copilot Instructions

This repository contains the `visual-hashing` Rust crate.

1. Preserve the public rendering contracts for emojihash and randomart unless a
   change intentionally updates the conformance vectors.
2. Do not hand-edit `vectors/**/*.json`; regenerate them from the scripts under
   `python/scripts/` and review the resulting diff.
3. Every source file must carry an SPDX `MIT OR Apache-2.0` license header.
4. Match the existing Rust style and keep `cargo fmt`, `cargo clippy`, tests,
   rustdoc, wasm build, and package dry-run green.
