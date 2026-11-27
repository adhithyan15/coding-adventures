# DT00 Go profile: checked edge mutations

The Go graph keeps the existing `AddEdge` and `SetEdgeProperty` methods and
their storage behavior. Opt-in `AddEdgeChecked` and
`SetEdgePropertyChecked` methods provide mutation-time validation for callers
that need the portable finite, non-negative edge-weight domain.

`AddEdgeChecked` validates only its positional `weight` argument. As in
`AddEdge`, zero means the default weight one. A `weight` key in an optional
property bag is ignored by the canonical edge-weight update; the positional
weight remains authoritative. Negative, NaN, and either infinity return
`ErrInvalidEdgeWeight` before nodes, edges, or properties are changed.

`SetEdgePropertyChecked` first verifies that the edge exists, preserving
missing-edge error precedence. For the canonical `weight` key, it applies the
same numeric conversion as `SetEdgeProperty`, normalizes zero to the existing
default one, then rejects negative or non-finite values with
`ErrInvalidEdgeWeight` before changing either the stored weight or property
bag. Non-weight properties retain existing behavior. These methods do not
validate other edges in the graph or change the behavior of legacy mutation
methods.

The legacy APIs continue to retain finite negative weights, so existing
minimum-spanning-tree callers can use signed finite weights. Existing zero to
one default behavior and the adjacency-matrix zero-means-no-edge representation
remain unchanged.

## Verification scope

Both representations are checked against a finite weight-class model for
negative, zero, positive, NaN, positive-infinity, negative-infinity, and
nonnumeric property inputs. Unit tests verify invalid operations leave graph
state unchanged and that legacy mutation/MST behavior remains available. A
native fuzz target explores the same bounded classes. A bounded SAT abstraction
using the repository's Go solver checked operation categories and
no-mutation/error precedence. It found no counterexample to those abstract
postconditions, produced a witness for zero-to-one with an ignored invalid
property-bag override, and a deliberately buggy override-validation control
produced a SAT witness. This scratch-only model does not execute the Go methods,
model arbitrary binary64 conversion or rounding, or constitute a general proof.
