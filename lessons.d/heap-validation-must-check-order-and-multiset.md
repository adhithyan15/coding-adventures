# Heap validation must check order and multiset

### What happened

A heap's parent-child invariant does not specify one unique backing-slice
layout, and checking only the root can miss lost, duplicated, or incorrectly
ordered later values.

### What to do

After each operation, compare length and root with an independent multiset
model, and check every parent-child relation. For a full construction example,
drain a separate copy and compare its complete ordered output with an
independently sorted input. Keep model checks outside benchmark timing.

When checking rejected operations, compare the observable sequence and length.
Do not treat nil versus allocated-but-empty slices as different states unless
that distinction is part of the public contract; a fuzz run exposed that
`reflect.DeepEqual` can report a false failure for an empty queue.
