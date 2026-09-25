### Changed — a filmstrip also lands in a Script block, so chinese, japanese, russian and urdu print theirs (HL-C443)

The third rollout left chinese, russian and urdu printing no filmstrip at all,
and japanese and arabic only one each. The cause was not their headwords, which
are single letters (尔, ち, ч, س). Those tracks present a letter under a
`## Script —` heading and have no `## Writing:` block, and derivation required
one.

`filmstripBlockIndex` now picks the first Writing block, or else the first
Script block. That block both qualifies a lesson and receives its figure.
Writing still wins when a lesson has both.

| | before | after |
|---|---|---|
| lessons that print a filmstrip | 207 | 369 |
| letters in the ledger | 130 | 254 |

Per track, the lessons that now print a filmstrip:

| track | lessons |
|---|---|
| chinese | 58 |
| hindi | 42 |
| marathi | 42 |
| sanskrit | 39 |
| gujarati | 33 |
| marwadi | 32 |
| japanese | 23 |
| tamil | 21 |
| russian | 18 |
| kannada | 13 |
| malayalam | 13 |
| urdu | 12 |
| persian | 11 |
| telugu | 9 |
| arabic | 3 |

Two new unit tests cover the fallback and the Writing-first precedence.
