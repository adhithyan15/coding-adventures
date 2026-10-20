---
category: Testing & coverage
---

# Named negative fixtures must preserve their specific failure witnesses

The initial BF03 validator pinned case names and required each payload to fail.
An independent review replaced all named malformed text with empty strings and
all malformed programs with null: the validator still passed while the specific
version, opcode, ID and domain evidence disappeared. A schema-invalid envelope
could also masquerade as an expected parser rejection. Pin typed/text witnesses
independently, preserve outer types before dispatch, and add degeneration tests.
Positive boundary/empty examples and generator starts need the same protection.
