---
category: Testing & coverage
---

# Curriculum membership direct owners must use the canonical pretty-printed byte form

The curriculum loader does more than parse each
`curriculum-membership.d/<lesson>.json` file: it serializes the parsed owner and
requires the source bytes to match that canonical representation exactly. A
hand-authored owner with a compact one-line `extensions` array is valid JSON but
fails every consumer with `curriculum membership owner ... is not canonical`.

Copy the layout of an existing owner or let the shard writer format it. In
particular, write each extension object on its own indented lines, retain the
final newline, and run a track corpus test before broader validation. The rule
is useful: byte-canonical direct owners make generation deterministic and keep
independent lesson shards reviewable.
