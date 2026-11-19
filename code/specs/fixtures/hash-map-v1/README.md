# DT18 Hash Map v1 Fixture

This fixture pins the DT18 Dart hash-map profile and supplies a shared case
corpus for the real public Swift consumer. It does not claim Dart conformance
or a Warrant/SAT proof. Dart package and BUILD routing remain a later gate.

## Contract and oracle

`schema.json` defines the JSON shape. `hash_map_oracle.py` contains two
separate reference models. `AssociationListModel` stores normalized
`(type-tag, raw-payload-bytes)` identities and ordered values. It does not call
or compare package `HashMapKey` equality, encoding getters, or lookup methods.
`PhysicalLayoutModel` separately models bucket/slot placement, growth,
tombstones, rehashing, and physical iteration order. Both use the independent
exact-integer hash primitives from the existing
`code/specs/fixtures/hash-functions-v1/oracle.py`.

The hash-answer section includes literal DT17/Warrant vectors for FNV-1a-32,
seeded Murmur3, and caller-key SipHash-2-4. Encoded-key hash rows are generated
by the independent oracle. `operation_cases` record every public operation's
expected typed error, count, capacity, exact load-factor ratio, query result,
and physical entry order. Null values remain distinct from absent keys by
recording the `has` observation.

## Exhaustive histories

`history_recipe` defines nine ordered operations: six set combinations over
three colliding byte keys and two values, then removal of each key. Histories
have lengths zero through five in length-then-lexicographic operation order,
for 66,430 cases per collision strategy. Every sequence is represented once
by its ordinal, `sum(9^j for j < length) + base9_rank(operation_indices)`.
For each sequence, the semantic stream stores one character
encoding the final value state (`absent`, `v0`, or `v1` for each key in
`k0,k1,k2` order), using `d0 + 3*d1 + 9*d2`. Each consumer replays every
sequence, checks every prefix against that prefix's own ordinal, and verifies
all earlier persistent versions after the sequence completes.

Separate physical-order streams point into the 16-entry table of ordered
subsets/permutations. Capacity streams pin capacity four for every history;
growth and non-power-of-two transitions are covered by directed operation
cases. Validators re-encode operation histories and decode every ordinal, so
matching stream length alone cannot hide missing or reordered histories.

## Cohorts and limits

The common Swift/Dart cohort contains byte values in `0..255`, valid Unicode
strings, and UInt32 Murmur seeds. Dart-only cases preserve raw invalid byte
integers, unpaired UTF-16 code units, out-of-range seeds, and lazy iterable
behaviors. Swift classifies those inputs as unrepresentable or skipped without
claiming rejection. The later Dart consumer must pass them through its real
public API without adapter prevalidation.

The public full-cycle case inserts and removes four FNV-colliding keys at
capacity four, creating four tombstones at live count zero. It then exercises
full-cycle lookup, earliest-tombstone reuse, and growth-before-tombstone-reuse.
The Swift consumer must execute those operations through the public API and
compare every recorded state. The production maximum-capacity map recipe is
recorded as a Dart gate3 requirement only; this Swift phase does not claim a
million-entry execution.

## Validation

Regenerate and validate the fixture with Python 3.13:

```powershell
py -3.13 generate.py
py -3.13 validate.py
py -3.13 -m unittest test_validate.py
```

`validate.py` checks the formal schema, key framing, literal and generated
hash answers, stream alphabet/lengths, history count/order/bijection, and all
semantic and physical outputs against the independent models. Its negative
controls cover truncation, malformed alphabets, reordered histories,
well-formed one-output mutations, and an expected hash value that is changed
without breaking the JSON schema. The Swift consumer must prove that the
well-formed wrong-answer control reaches and fails the actual public
comparator.
