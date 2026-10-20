---
category: Rust
---

# Rust's {:e} shortest float formatting rounds an exact tie up, so it is not an oracle for Python repr or JavaScript toString

Python's `repr`, ECMAScript `Number::toString`, Java 19+ and Ryu all print the
shortest round-trip digits, choose the closest on a length tie, and on an
EXACT tie choose the even last digit. Rust's `format!("{:e}", x)` agrees on
the digit length, but on an exact tie it rounds UP: `2^-25` =
`2.98023223876953125e-08` prints as `…313e-8` in Rust and `…312e-08` in
Python. The same goes for any double near `1e15` with a `.25`/`.75` fraction
(`1072012167405248.25`).

LANG83 PR6a's spec first named Rust's `{:e}` as the oracle for every backend's
float printer. The first unit test against host `python3` caught it. Across
535 432 values, Rust differed on 94, and every one was such a tie.

What to do: use host `python3`/`node` as the oracle (and re-run them in a test
whenever they are installed), never Rust's formatting. When Rust's digits feed
an output that must match Python/JS, detect the tie with Rust's exact mode
(`{:.1100e}` prints every digit of a double): if the exact expansion is
exactly one digit longer than the shortest and ends in `5`, take the even
neighbour, provided it reads back as the same bits (`vm_core::float_format`).
