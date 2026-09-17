#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Enforce this repository's licensing claims instead of asserting them.

Four things are checked, each of which has quietly drifted in real projects:

1. Every tracked file declares the expected SPDX expression - either in its own
   header, or through a ``REUSE.toml`` annotation for formats that cannot carry
   a comment (JSON vectors, the lockfile).
2. The ``REUSE.toml`` annotations declare that same expression, so the two
   mechanisms cannot disagree.
3. The root ``LICENSE-*`` files are byte-identical to their ``LICENSES/*.txt``
   counterparts, which is the convention this repository already follows.
4. The library has **no runtime dependencies**. That is a headline claim in the
   README and the reason the dependency licence surface is as small as it is,
   so it is worth a gate rather than a habit.

Run from anywhere; paths are resolved against the repository root.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

EXPECTED_SPDX = "MIT OR Apache-2.0 OR MulanPSL-2.0"
"""The SPDX expression every first-party file must declare."""

LICENSE_PAIRS = {
    "LICENSE-MIT": "LICENSES/MIT.txt",
    "LICENSE-APACHE": "LICENSES/Apache-2.0.txt",
    "LICENSE-MULAN": "LICENSES/MulanPSL-2.0.txt",
}

MULAN_SHA256 = "eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c"
"""SHA-256 of the official bilingual Mulan PSL v2 text.

Pinned from the SPDX license-list-data mirror because the canonical host
(license.coscl.org.cn) was unreachable when this was adopted:
https://raw.githubusercontent.com/spdx/license-list-data/main/text/MulanPSL-2.0.txt
"""


def tracked_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True
    )
    return [line for line in out.stdout.splitlines() if line]


def reuse_annotations() -> dict[str, str]:
    """Map each annotated path pattern to the SPDX expression it declares."""
    with (ROOT / "REUSE.toml").open("rb") as handle:
        config = tomllib.load(handle)

    patterns: dict[str, str] = {}
    for block in config.get("annotations", []):
        paths = block["path"]
        if isinstance(paths, str):
            paths = [paths]
        for path in paths:
            patterns[path] = block["SPDX-License-Identifier"]
    return patterns


def matches(pattern: str, path: str) -> bool:
    if pattern.endswith("/**"):
        return path.startswith(pattern[:-2])
    return pattern == path


def check_headers(failures: list[str]) -> None:
    annotations = reuse_annotations()
    expected_line = f"SPDX-License-Identifier: {EXPECTED_SPDX}"

    for pattern, declared in annotations.items():
        if declared != EXPECTED_SPDX:
            failures.append(
                f"REUSE.toml: {pattern} declares {declared!r}, expected {EXPECTED_SPDX!r}"
            )

    license_texts = set(LICENSE_PAIRS) | set(LICENSE_PAIRS.values())

    for path in tracked_files():
        if path in license_texts:
            continue
        if any(matches(pattern, path) for pattern in annotations):
            continue

        text = (ROOT / path).read_text(encoding="utf-8", errors="replace")
        if expected_line not in text:
            failures.append(f"{path}: missing '{expected_line}'")


def check_license_texts(failures: list[str]) -> None:
    for root_name, licenses_name in LICENSE_PAIRS.items():
        root_file = ROOT / root_name
        licenses_file = ROOT / licenses_name
        wanted = "MulanPSL-2.0" in EXPECTED_SPDX or root_name != "LICENSE-MULAN"

        if not root_file.exists() or not licenses_file.exists():
            if wanted:
                failures.append(f"{root_name} / {licenses_name}: missing licence text")
            continue

        if root_file.read_bytes() != licenses_file.read_bytes():
            failures.append(f"{root_name} and {licenses_name} differ")

    mulan = ROOT / "LICENSE-MULAN"
    if "MulanPSL-2.0" in EXPECTED_SPDX and mulan.exists():
        digest = hashlib.sha256(mulan.read_bytes()).hexdigest()
        if digest != MULAN_SHA256:
            failures.append(
                f"LICENSE-MULAN sha256 {digest} does not match the pinned {MULAN_SHA256}"
            )


def check_no_runtime_dependencies(failures: list[str]) -> None:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    package = json.loads(out.stdout)["packages"][0]

    # `kind` is null for a normal dependency, "dev" or "build" otherwise.
    runtime = [
        dep["name"] for dep in package["dependencies"] if dep.get("kind") is None
    ]
    build = [
        dep["name"] for dep in package["dependencies"] if dep.get("kind") == "build"
    ]

    if runtime:
        failures.append(
            "the library must have no runtime dependencies, found: "
            + ", ".join(sorted(runtime))
        )
    if build:
        failures.append(
            "the library must have no build dependencies, found: "
            + ", ".join(sorted(build))
        )


def main() -> int:
    failures: list[str] = []
    check_headers(failures)
    check_license_texts(failures)
    check_no_runtime_dependencies(failures)

    if failures:
        print("license check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(f"license check passed ({EXPECTED_SPDX}, no runtime dependencies)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
