# BEAM putchar compact GC root repair

Preimplementation contract for the PR15780 CI repair.

Linux push job106074353057 crashes the Brainfuck raw-byte sweep with
size_object: bad tag for 0x0. The existing putchar heap reservation scans all
preallocated X registers, including values not initialized or clobbered by
imported calls. The local Windows sweep passes; it does not disprove the hazard.

After saving values live across putchar into initialized Y slots, move the
character operand into x0 before test_heap. Reserve two words with live=1,
construct the one-character list from x0 into x0, call io:put_chars/1, and restore
the existing saved values. Preserve the existing liveness computation, byte
semantics, return convention and every other lowering operation.

A structural regression must place future variables and a value live across
output around putchar and assert the single initialized X root, list operand,
and save/restore order. It must fail on the previous wide-root emission.
Run the full BEAM backend suite with real Erlang, the exact Brainfuck 256-byte
and EOF probe, and the existing loop-state output test. Linux CI must pass at
the new head before merge; local success is not a claim that Linux was tested.
Keep CLR13 contract and implementation unchanged.