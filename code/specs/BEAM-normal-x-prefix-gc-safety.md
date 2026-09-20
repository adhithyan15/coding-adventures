# BEAM normal X-prefix GC safety repair

Preimplementation contract for the persistent PR15780 Linux CI failure.

The compact putchar root repair did not resolve the OTP 27.3.4.11 crash in
job 106080216851. The log does not identify the exact collecting instruction.
Tape index arithmetic and ordinary arithmetic still scan a contiguous normal
X-register prefix containing future locals and registers clobbered by calls.
Initializing high scratch operands alone does not make that prefix safe.

Initialize every preallocated non-parameter normal X register to the immediate
nil term at function entry, after allocating and initializing any Y frame.
Preserve incoming parameter registers. At each existing imported-call restoration
boundary, preserve the operation destination and restore the existing live-across
Y values, then overwrite every other normal X register with nil. Apply the same
normal-prefix cleanup after local-call restoration. No allocation may intervene
between result capture, restoration, and cleanup. Do not clear scratch registers
while an operation expansion still needs them. Keep the existing liveness
analysis, register bounds, byte masking, input/EOF behavior and call ABI.

This establishes valid normal roots at IIR boundaries for the byte-tape and
scalar operations. Tape load/store must continue initializing their entire
additional scratch prefix before GC-capable index adjustment or byte masking.
Putchar and getchar retain compact internal root sets with Y preservation.
This contract does not claim a complete audit of other multi-call expansions,
closure lowering, or arbitrary malformed IIR. CLR13 behavior remains unchanged.

Add deterministic emitted-code root tracking regressions that invalidate X
registers across calls and beyond each declared GC live count, then reject any
later GC scan of an invalid register. Cover future variables at entry, allocation,
getchar, putchar, tape load/store, scalar arithmetic, live tape references and
counters, and an aliased destination. Verify parameter preservation and a local
call followed by arithmetic. The regression must fail on the previous emission.
Run the complete BEAM suite and real Erlang byte/EOF and output-loop tests, plus
Clippy and documentation checks. Local Windows success is not Linux validation;
all latest exact-head Linux and other required CI must pass before merge.
