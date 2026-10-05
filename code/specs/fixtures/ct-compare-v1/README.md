# CT01 neutral conformance fixtures v1

This is a process-free behavior oracle for the pure `ct-compare` package in
every established implementation language. The governing contract is
[`CT01-constant-time-compare.md`](../../CT01-constant-time-compare.md).

`schema.json` closes the transport shape. `cases.json` contains 21 cases for
byte equality, fixed-width equality, byte selection, and unsigned-64 equality.
All bytes and unsigned integers are canonical lowercase hexadecimal; the
expected result and logical public-work counts are checked by an independent
validator rather than captured from an existing package.

Run from the repository root:

```sh
python code/scripts/ct_compare_conformance.py
python -m unittest discover -s code/scripts/tests -p test_ct_compare_conformance.py -v
```

Adapters should decode a case, call the real package function, and compare the
result or normalized semantic error. A successful selection must return fresh
storage: mutate a mutable input after the call and confirm the result is
unchanged. For `expected_work`, compare the public loop shape and inspect the
production implementation; a passing functional case alone does not prove
data-independent control flow. In particular, this suite neither measures
wall-clock timing nor guarantees compiler, VM, cache, or hardware behavior.

The sole runtime error ID is `CT_SELECT_LENGTH_MISMATCH`. Malformed JSON,
duplicate keys, noncanonical hex, invalid fixed widths, and wrong expectations
are fixture-authoring failures that the validator rejects before any adapter
receives them.
