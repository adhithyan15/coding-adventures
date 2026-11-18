# omit-example-v2: the `omit_example` agreement fixture (CV2-S10-1)

`omit.warrant` + `corpus.profile` omit three examples, one per way the oracle decides a reason
(VER04r 8.4, [D34]); `corpus/` is what the oracle derives from them. It is written by
`make_fixture.py` (the oracle has no corpus writer of its own; the generator writes the other
fixtures), and `BUILD` runs `make_fixture.py --check` and `verify-v2` on it.

| Example | Why it is omitted | `reason` |
|---|---|---|
| `echo_big_value` | evaluates fine; its argument is 8,192 bytes (> 4,096) | `value-over-limit` |
| `echo_size_error` | the argument passes `MAX_IR_VALUE` while it is evaluated (first error is a size error) | `value-over-limit` |
| `echo_two_sided` | `echo(..) != echo(..)` is not `op(args) == pure` | `not-op-eq-pure` |

`echo_small` is kept, so `cases.tfv` has one record. The recorded generator is
`warrant-corpus-gen@0.5.0`; the generator's S10-1 PR is checked against this directory and
regenerates it from its own output (the bytes must not move).

**[D13] note.** The recorded generator is pinned by `BUILD`. A generator version bump must
regenerate this fixture (`make_fixture.py`, or the generator's own writer once it has one) and move
the pin. The generator's S10-1 PR adds this fixture to the ct-compare README checklist and to the
section 16.8 count of places a bump touches; this PR does not edit that README.
