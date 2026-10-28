# Overlap suppression can still have register side effects

An instruction manual saying that an operation is suppressed does not imply
that every architectural output is unchanged. IBM System/370 `MVCL` detects a
destructive overlap before moving storage and reports condition code 3, yet its
completion rule still clears the ignored high byte of both address registers.

When translating a source rule into an atomic simulator transition, separate
the effects explicitly:

- storage movement;
- operand progress;
- completion-time register normalization; and
- status reporting.

A regression that initializes ignored or reserved fields to nonzero values is
often the only test that distinguishes “no useful work happened” from “the
architectural completion path made no changes.”
