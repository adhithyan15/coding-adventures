# DB-E118 — Preserve exact Unicode in the Go HDD fixture

The DB02b adapter at SHA-256
`2b9b77b7a1f96d59a004ca49940ddf3fb772a6f7e4fd80b0592791526259ae91`
accepts the canonical timing vector and ordinary Unicode case names.
It also accepted complete vector variants with an isolated high
surrogate, an isolated low surrogate and a high surrogate followed
by a non-surrogate escape. Go's JSON decoder replaces those escaped
code units during parsing, which makes the decoded case name differ
from the supplied text.
[Go timing profile](DB02b-hdd-simulator-go.md)

Apply the same Unicode scalar boundary as the qualified Go NAND
reader before JSON decoding. Keep the byte and depth limits,
strict decoded key checks and exact operation shape. Valid UTF-8,
paired surrogate escapes and literal escaped backslashes continue
to decode normally. Keep this change in the test-only adapter and
its tests. The canonical fixture and public timing runtime need no
changes to support these checks.

Replay the three malformed variants and the three affirmative controls
through the real adapter. Add a decoded duplicate-name case and retain
invalid UTF-8 and trailing-data coverage. Run both command lists,
coverage, race, vet, fixed parser and history fuzz, and architecture
checks. Verify the exact HDD-only route before publishing DB02b.

## Local verification

The Unicode repair passed all six independent controls. Three
invalid escaped sequences received a refusal. Literal Unicode,
paired surrogates and the escaped backslash replayed the full
fixture. The repaired adapter SHA-256 before DB-E119 is
`a8c714212b7da74a05888db881d8eb1ebeb3ddb0f86de8f3326a282169f7332e`.
Both native command lists ran on Linux, with race and vet.
