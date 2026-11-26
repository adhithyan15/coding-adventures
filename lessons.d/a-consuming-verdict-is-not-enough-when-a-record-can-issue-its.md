---
category: Cryptography & security review
---

# A consuming verdict is not enough when a record can issue its proof again

The TLS 1.3 Finished callback can verify the same message twice and return
two valid, non-cloneable verdicts. A join that consumes one verdict but only
borrows the committed record can therefore issue more than one transition
proof from a single record. That is unsafe for downstream code which might
use each proof to start a new record epoch at sequence zero.

Spend the record result as well as the verdict on every attempt. Test reuse
with two independently issued verdicts and one committed record; checking
that the output type is not cloneable alone does not cover this case.
Inspect stable rustdoc's compile-fail output with `--show-output`: the
example can fail for the wrong reason even with an expected error code.
