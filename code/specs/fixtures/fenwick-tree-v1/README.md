# Fenwick tree conformance fixtures v1

These files define the language-neutral DT06 observable contract. They are
test data, not a runtime dependency. Consumers should run each case against an
independent plain-array model, and should include the bounded exhaustive domain
described in `contract.md`.

Indices are one-based except constructor input arrays. Non-integer input type
behavior is outside v1 conformance as documented in `contract.md`. `prefix_sum(0)` is valid
and returns zero. Invalid operations must leave the tree unchanged.


Run `python3 code/specs/fixtures/fenwick-tree-v1/validate.py` to check fixture structure and expected values against the independent flat-array oracle. That command does not execute package implementations; each language adapter must feed these cases to its implementation and compare results/errors/state to the fixture. The script also exhausts its bounded oracle domain locally.
