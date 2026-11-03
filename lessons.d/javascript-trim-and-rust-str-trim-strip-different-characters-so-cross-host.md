---
category: TypeScript / JavaScript
---

# JavaScript trim and Rust str::trim strip different characters, so cross-host string matching must trim with an explicit White_Space set

**What went wrong.** Backlog 6e2 made TaskApp's note-to-task name match
identical on the native host (Rust) and the web host (TypeScript). The
lowercasing turned out to agree (`str::to_lowercase` and `toLowerCase()` give
the same code points even for final sigma, U+0130, sharp s and the DŽ
digraphs), but the *trim* in front of it did not. A scan of every code point
showed `String.prototype.trim` strips U+FEFF and keeps U+0085, while Rust's
`str::trim` (the Unicode `White_Space` property) does the opposite. So
"Groceries" followed by NEXT LINE attached natively and was refused on the web.

**Fix.** The web host trims with `/^\p{White_Space}+|\p{White_Space}+$/gu`,
which is exactly Rust's set; shared vectors pin both characters on both hosts.

**Next time.** When two hosts must agree on a string rule, compare *every*
stdlib step, not just the obvious one. Brute-force the full code-point range
in both languages (a few lines each) and diff the outputs before writing the
rule down. The same mismatch still exists in Add Task and the Sheet's Labels
column, which trim with each platform's own `trim`.
