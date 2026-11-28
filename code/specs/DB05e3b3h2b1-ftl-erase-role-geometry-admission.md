# DB05e3b3h2b1 — Share erase-role geometry admission

A terminal image can have zero or more incomplete blocks. The
whole-set binder needs to validate geometry before matching selections,
including images where it never qualifies an individual target.
Expose the admitted check in `ftl-erase-role-evidence` as
`ValidateGeometry(geometry Geometry) error`.
[Existing qualifier](DB05e3b3d-ftl-interrupted-erase-role-evidence.md) · [Single target binding](DB05e3b3h2a-ftl-bind-protected-erase-intent.md)

Use the qualifier's current bounds. Require positive block and page
counts and a product from 2 through 128. Check bounds before the
product or an index conversion. Return `ErrInvalidGeometry` first if
those checks fail. Then refuse an erase ceiling above `2^53 - 1` with
`ErrInvalidEraseCount`. All admitted geometries return nil.

Use that function at the existing qualifier's geometry boundary.
Verify exact precedence against the original implementation before
checking target-specific conditions. Exercise signed coordinate
extremes, maximum product factorizations, invalid combinations, zero
erase budget, the portable ceiling and one beyond it. Retain the
existing qualifier's tests and run its native suite, race, vet and
existing dependent callers. The complete selected-set binder uses
this contract in a separate slice.

## Local evidence

Commit `3bcfab25039` matched 117,600 independent decisions against an
arbitrary-precision product model and checked the qualifier's error
precedence. The package's native suite adds 91,125 bounded geometry
cases. The emitted recipe passed at 100% coverage with race and vet.
All three existing mutation controls and all 13 package callers passed.
The graph's 5,998 rows and the recipe inventory's 5,997 rows matched
the earlier exports.
