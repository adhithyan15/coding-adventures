---
category: Security boundaries
---

# Names a compiler invents from user names multiply their length, so an output budget needs an input name-length cap and a charge taken before formatting

**What went wrong.** #18474 gave the Twig compiler budgets on source
bytes, tree items, registers, and instructions per function and per
module, and the PR said they stopped the quadratic record expansion. The
security review found a way around them. A record makes the compiler *and*
the type checker invent a predicate and one `name-field` accessor per
field. Every accessor repeats the record's name, and `NAME` had no length
limit. A 512 KB record name with about 58,000 fields fits in 1 MiB of
source and 460K tree items, inside every budget, and the pre-pass
(`fn_globals.insert(format!("{prefix}-{f}"))`) and `register_record`
each formatted about 30 GB of names. That happened before lowering, so the
instruction budgets never had a chance to count anything. Even
`emit_record_def` built about 750 MB of names before the module budget
tripped. The budgets counted *instructions*, and the cost was in *names*.

**Fix.** Two budgets, in `twig-parser`, the one crate every consumer goes
through:

- `MAX_NAME_BYTES` = 256, checked on every `NAME` token before parsing.
  The compiler's tree walk checks it too, for hand-built trees.
- `MAX_GENERATED_NAME_BYTES` = 1 MiB, applied to the formula
  `(|name|+1) + Σ(|name|+1+|field|)`. The formula is computed by
  arithmetic in `extract_program`, in the type checker before Pass 1, and
  in the compiler's pre-pass, each time before a single name is formatted.

**Next time.**

- **Find every invented name.** When a frontend invents names from user
  names (accessors, mangled symbols, `__r{n}_{name}` registers), list
  every place that does it, in every consumer. A per-element string
  repeated per element is a multiplication. A name cap alone gives only a
  constant factor (513 bytes per field here), so also charge the product
  against a budget, before allocating.
- **Put input caps upstream.** Put them in the crate everything parses
  through, not in one consumer.
- **Don't stop at the work you were asked to bound.** Measure what an
  attacker can make each phase *allocate*, not just the instructions it
  emits.

**Same trap, one layer over (re-review).** The fix above still missed one
case. The type checker's *diagnostics* had the same shape: a
non-exhaustive `match` listed every unmatched variant, so each one-arm
match on a 60,000-variant union cost ~600 KB of message, and 50,000
matches cost ~33 GB. Every message that describes a *type* (a list of
variants, fields or overloads) costs the size of that type, once per
error. Bound three things:
- each list (`type_checker_protocol::bounded::name_list`: 8 names, then
  "… and K more");
- each message (4 KiB);
- the number and total bytes of diagnostics (256, 1 MiB, then one note).

Then skip building a diagnostic that would be dropped.
