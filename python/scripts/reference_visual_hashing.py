# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Small Python reference implementation used only to regenerate vectors."""

from __future__ import annotations

from typing import Final

from blake3 import blake3

_EMOJI_DIGITS: Final[tuple[tuple[str, str], ...]] = (
    ("🐵", "monkey"),
    ("🐶", "dog"),
    ("🐺", "wolf"),
    ("🦊", "fox"),
    ("🐱", "cat"),
    ("🦁", "lion"),
    ("🐯", "tiger"),
    ("🐴", "horse"),
    ("🦄", "unicorn"),
    ("🦓", "zebra"),
    ("🦌", "deer"),
    ("🐮", "cow"),
    ("🐷", "pig"),
    ("🐗", "boar"),
    ("🐭", "mouse"),
    ("🐹", "hamster"),
    ("🐰", "rabbit"),
    ("🐻", "bear"),
    ("🐼", "panda"),
    ("🐨", "koala"),
    ("🐸", "frog"),
    ("🐲", "dragon"),
    ("🐔", "chicken"),
    ("🐧", "penguin"),
    ("🦆", "duck"),
    ("🦅", "eagle"),
    ("🦉", "owl"),
    ("🦇", "bat"),
    ("🐢", "turtle"),
    ("🐍", "snake"),
    ("🦎", "lizard"),
    ("🐊", "crocodile"),
    ("🐳", "whale"),
    ("🐬", "dolphin"),
    ("🐟", "fish"),
    ("🐠", "tropical-fish"),
    ("🐡", "blowfish"),
    ("🦈", "shark"),
    ("🐙", "octopus"),
    ("🦑", "squid"),
    ("🦀", "crab"),
    ("🦞", "lobster"),
    ("🦐", "shrimp"),
    ("🦋", "butterfly"),
    ("🐌", "snail"),
    ("🐞", "lady-beetle"),
    ("🐝", "bee"),
    ("🐜", "ant"),
    ("🦂", "scorpion"),
    ("🍎", "apple"),
    ("🍐", "pear"),
    ("🍊", "orange"),
    ("🍋", "lemon"),
    ("🍌", "banana"),
    ("🍉", "watermelon"),
    ("🍇", "grapes"),
    ("🍓", "strawberry"),
    ("🍒", "cherries"),
    ("🍍", "pineapple"),
    ("🥝", "kiwi"),
    ("🍑", "peach"),
    ("🥥", "coconut"),
    ("🥕", "carrot"),
    ("🌽", "corn"),
)
_RANDOMART_VALUES: Final[str] = " .o+=*BOX@%&#/^"


def emoji_indices(data: bytes, length: int) -> list[int]:
    wanted = max(1, length)
    digest = blake3(data).digest(length=((wanted * 6) + 7) // 8)
    indices: list[int] = []
    acc = 0
    bits = 0
    for byte in digest:
        acc = (acc << 8) | byte
        bits += 8
        while bits >= 6 and len(indices) < wanted:
            bits -= 6
            indices.append((acc >> bits) & 0x3F)
    return indices[:wanted]


def emojihash(data: bytes, length: int = 11) -> str:
    return " ".join(_EMOJI_DIGITS[idx][0] for idx in emoji_indices(data, length))


def emojihash_labels(data: bytes, length: int = 11) -> str:
    return " ".join(_EMOJI_DIGITS[idx][1] for idx in emoji_indices(data, length))


def randomart(data: bytes, label: str = "") -> str:
    width, height = 17, 9
    start_x, start_y = width // 2, height // 2
    grid: list[list[int]] = [[0] * width for _ in range(height)]
    x, y = start_x, start_y

    for byte in data:
        for shift in range(0, 8, 2):
            move = (byte >> shift) & 0x3
            if move == 0:
                y = max(0, y - 1)
                x = max(0, x - 1)
            elif move == 1:
                y = max(0, y - 1)
                x = min(width - 1, x + 1)
            elif move == 2:
                y = min(height - 1, y + 1)
                x = max(0, x - 1)
            else:
                y = min(height - 1, y + 1)
                x = min(width - 1, x + 1)
            grid[y][x] += 1

    end_x, end_y = x, y
    grid[start_y][start_x] = 0
    grid[end_y][end_x] = len(_RANDOMART_VALUES) - 1

    header = f"+--[{label:14s}]+" if label else "+----------------+"
    lines = [header]
    for row_idx, row in enumerate(grid):
        line_chars = ["|"]
        for col_idx, count in enumerate(row):
            if row_idx == start_y and col_idx == start_x:
                line_chars.append("S")
            elif row_idx == end_y and col_idx == end_x:
                line_chars.append("E")
            else:
                line_chars.append(
                    _RANDOMART_VALUES[min(count, len(_RANDOMART_VALUES) - 1)]
                )
        line_chars.append("|")
        lines.append("".join(line_chars))
    lines.append("+----------------+")
    return "\n".join(lines)
