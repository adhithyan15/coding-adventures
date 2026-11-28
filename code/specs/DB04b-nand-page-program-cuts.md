# DB04b — Interrupted programming of one NAND page

Introduce explicit power-loss cuts to one ideal SLC page program. This slice
uses the page contents and program marker from [DB01](DB01-nand-flash.md).
It follows the completion and acknowledgement distinction introduced by the
HDD write cuts in [DB04a](DB04a-hdd-write-crash-cuts.md).

## State, inputs, and outcomes

The input is a page-size value, a page state, one complete payload, and one
cut. Page sizes are integers in `1..=65536`. A valid state owns exactly that
many bytes and a program marker. A clear marker requires every byte to be
erased at `0xff`. A set marker admits the byte contents of a completed or
interrupted program. The distinct states with all-ones bytes can therefore
have different retry behavior.

The cut specifies how far a program progressed. It selects precisely one
state and acknowledgement outcome.

| Cut | Media bytes | Program marker | Acknowledged |
|---|---|---|---|
| `before_accept` | The erased page | Clear | No |
| `after_prefix(n)` | First `n` payload bytes, with an erased suffix | Set | No |
| `after_media` | Full payload | Set | No |
| `complete` | Full payload | Set | Yes |

`after_prefix(n)` admits `0 <= n < page_size`. Prefix zero models a command
that consumed the marker before changing any byte. The exact-length payload
can include erased bytes at any position. Its bytes and the original page
state remain caller-owned. The outcome provides detached bytes and a marker,
and an acknowledgement flag. Use a separate result kind for a refusal.

Validate size, state shape and consistency, payload length, cut shape and
prefix range, then the existing marker. The respective domain errors are
`invalid_page_size`, `invalid_page_state`, `wrong_payload_length`,
`invalid_cut`, and `already_programmed`. Every refusal preserves the input
state. A call on one page influences no other page or wear count.

`before_accept`, `after_prefix`, and `after_media` all report absent
acknowledgement. A subsequent program can proceed only if the preserved
marker is clear. For example, an all-ones payload has identical byte contents
before acceptance and after all media work. The marker and the completion
receipt distinguish those histories.

## Manual evidence

- Enumerate every supported cut and every prefix for small page sizes one
  through four. Generate asymmetric payloads that exercise independent bit
  positions. Check before/after bytes, the acknowledgement flag, and the
  next program's result against a separate prefix oracle.
- Verify all-ones, all-zero, exact maximum size, minimum and invalid sizes,
  short and long payloads, inconsistent clear-marker states, invalid cut
  payloads, precedence, and detached buffers.
- Add a bounded shared vector and check its declared outcomes against the
  prefix oracle before replay. Include a completed page lost before
  acknowledgement, a cut at prefix zero, and attempts to reprogram both
  zero-prefix and all-ones programmed pages.
- Fuzz the fixture and bounded command histories. In disposable copies check
  that an extra programmed suffix byte, a cleared marker after a partial
  program, and acknowledgement before media completion fail named tests.
- Run a separate adapter check once the pure model is integrated with a
  block medium. Populate adjacent pages and at least one other block and
  verify by readback that their bytes and erase counts are preserved.

This model chooses an exact byte prefix and an explicit command marker for
replay. A block-erase interruption, variable physical disturb, and controller
recovery each require their own media guarantees.
