## HL-C447-3cff1a11 — Track-specific integration ratchets now live in independent evidence modules

**Status: CLOSED — implemented by #16096.** After the filmstrip aggregates
were sharded, `tests/integration.test.ts` remained the next shared hotspot at
seven recent touches. Spanish level pins, Persian/Urdu chapter chains, and the
Japanese script runway all lived beside corpus-wide contracts, so extending
one track edited the shared integration module.

Those ratchets now live in three independently owned evidence modules. The
integration gate discovers them eagerly, rejects duplicate evidence identities,
and supplies the same once-loaded corpus context, preserving every assertion
without reparsing the curriculum for each owner.

The same audit now has no cross-track aggregate above two recent touches.
The highest remaining track-local test hotspot is
`tests/corpus/arabic.test.ts` at three touches; its next useful concurrency pass
is to separate unrelated Arabic curriculum ratchets only if independent Arabic
tranches begin colliding there.
