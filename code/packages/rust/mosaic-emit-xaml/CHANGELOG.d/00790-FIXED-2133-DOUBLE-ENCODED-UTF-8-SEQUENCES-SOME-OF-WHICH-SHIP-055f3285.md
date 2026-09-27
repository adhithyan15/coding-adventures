### Fixed — 2133 double-encoded UTF-8 sequences, some of which shipped (#14885)

Repo-wide repair plus a CI guard. The corruption comes from copying a non-ASCII
character out of terminal output: the terminal renders the character's UTF-8
bytes as cp1252, and re-encoding those glyphs produces a longer sequence that
still looks like the original in a rendered diff. That invisibility is why 2133
of them accumulated.

| file | occurrences |
| --- | --- |
| `mosaic-emit-xaml/src/pipeline.rs` | 1706 |
| `semantic-ir-to-go/CHANGELOG.md` | 245 |
| `python/lexer/.../grammar_lexer.py` | 61 |
| `python/haskell-lexer/.../lexer.py` | 58 |
| `python/haskell-parser/.../parser.py` | 54 |
| six others | 9 |

**#14885 said this was comment-only. That was wrong, and it was my claim.**
Classifying by line rather than assuming found 10 non-comment occurrences in
`pipeline.rs`, and they reach generated artifacts:

- four in the emitted WinUI host **README** text
- one in an emitted **XAML comment** (`HostDialog dismiss-on-backdrop`)
- one in a string that becomes a comment in generated code
- two in **user-facing error messages** (`unmatched LParen`, and the
  HostTable child diagnostic)
- two in test assertion messages, which are harmless

So this was not cosmetic: mojibake was being written into shipped output.

The repair is byte-level and the mapping is *derived* rather than hand-typed —
each corrupt sequence is computed as
`ch.encode("utf-8").decode("cp1252").encode("utf-8")`, and the script asserts
that round-trip before touching a file, so the table cannot drift from the
characters it protects.

`code/scripts/check_mojibake.py` now runs in CI. It was falsified by planting a
corrupt em-dash in a scratch file: the guard reported it and exited 1.

