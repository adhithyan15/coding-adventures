# DB-E176 — Publish the Go NAND block-erase cut model

Release the Go block-erase transition and shared vector. The
model records partial page prefixes, completed media state,
command acknowledgement and the erase counter.
[Erase model](DB04d-nand-block-erase-cuts.md) · [Prior route](DB-E41-go-nand-block-erase-fixture-routing.md) · [Scalar audit](DB-E176a-go-nand-block-erase-fixture-scalars.md)

Carry the package and original vector from `6f7aa2fd0a8` with
the admitted scalar repair and the linked routing and scalar
contracts. Port only the block-erase fixture
rule from `c99de3a7c81` onto the fetched release base. Preserve
the Go page-program cut route and all other exact inputs. Use
the base's registry tooling to regenerate input-boundary
projections and verify the resulting digest and census.

Compare ordinary and fixture-only selection, exact deletion,
neighboring nonmatch and cache key isolation for the Go reader.
Verify added package, fixture input, recipe and all unchanged
baseline graph rows and recipes. Run both package command lists,
guided inputs, clean controls, the Go, Python, C# and Swift
input checks, and repository metadata.

Confirm native CI and the two required gates at the reviewed
head. Verify all scoped paths on fetched main after merge.
