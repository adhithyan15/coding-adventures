---
category: Python
---

# A domain rule that reads the claim body is blind to shifts behind a model_call, so a fixture of model-call claims never exercises it

VER04r R10 gives the 73-value `nat_shift` leaf to a `Nat` binder that *is* the count operand of `shl shr sar
rotl rotr bit` in the claim body, and to nothing that reaches a count through a `model_call` argument or an
`admit`. The first draft of the `fixed-eval-v1` fixture wrote every claim as `op_sh_u8(a, n) == sh_u8(a, n)`
with the shifts inside the model `sh_u8`. The rule was working as specified, and the fixture's shift
suites still had the 65-value `nat_0_64` for `n`: the one domain the slice exists to add was never
exercised, and nothing failed. A dump that is meant to pin a rule must contain a case the rule *fires* on.

Fix: the shift claims write the built-ins in the claim body itself, and the document has an explicit
selection matrix (selected: each of the six tags, inside a tuple literal, inside a `match` arm, a count
that is also in an `admit`; not selected: a count that is an expression of the binder, a `model_call`
argument, a count only in an `admit`, a projection of a tuple binder). The test over the committed dump
reads the descriptor of every binder in that matrix. For any rule keyed on a syntactic position, list the
positions it must *not* fire on beside the ones it must, and check both in the committed output.
