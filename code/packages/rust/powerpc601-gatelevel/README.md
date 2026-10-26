# PowerPC 601 gate-level simulator

This crate is the Rust gate-level partner for the PowerPC 601 educational
machine in Spec 07u. It implements the complete functional integer surface
while clocking every persistent architectural bit through the repository's
master/slave D-flip-flop primitive.

The exact topology is 525,473 DFFs: 524,288 memory bits, 1,024 GPR bits, 160
bits for LR/CTR/XER/CR/CIA, and one halt bit. Installed-program bounds are
validated lifecycle metadata rather than persistent circuit state.

Decode is fixed-field combinational logic. Arithmetic, carries, comparisons,
Boolean operations, shifts, count-leading-zero, effective addresses, branch
updates, 32-round multiplication, and fixed-round signed/unsigned division use
repository gate and ripple-carry networks. Successful transitions clock the
DFF state; typed instruction faults leave every DFF unchanged.

Conditional branches use an explicit BO-controlled gate network: a muxed
gate-ALU CTR decrement/nonzero path and a selected CR-bit/polarity path feed
independent bypass muxes and a final AND wire. An exhaustive truth-table test
covers all 32 BO values, all 32 CR selectors, both selected-bit polarities, and
CTR zero/nonzero boundaries.

CR and XER updates are likewise gate-authoritative. Subtractor Z/N/V/C wires
produce one-hot signed and unsigned relation bundles. A three-to-eight decoder
and per-bit muxes isolate compare fields, a write-enable mux controls CR0, and
32 muxes implement `mtcrf`. Split signed/unsigned compare lines gate-select the
relation bundle, and decoded compare/`mtcrf` lines gate-qualify the final CR
commit; host opcode and XO tests do not authorize status writes. XER[CA] is the
only bit replaced by ordinary
carry-producing instructions; SO, OV, and all opaque XER bits remain stable.
`adde` consumes CA from stable DFF Q and CR nibbles copy stable XER[SO].

The modeled XO arithmetic profile admits only `OE=0,Rc=0`. Every other
modifier pair returns the exact typed unknown-instruction error before a GPR,
CR, XER, CIA, memory byte, halt bit, or installed-program bound can change.
X-form record operations remain supported. The `sraw` CA network consumes all
six count wires, including the source-sign result for counts 32 through 63;
the shift movement, leading-zero counter, and divider remain separate cells.

The checked API shares `PowerPcState`, `PowerPcError`, `StepTrace`, and
`ExecutionResult` with `powerpc601-simulator`, including exact snapshots,
validated restore, origin-aware loading, checked direct access, atomic steps,
and transactional bounded runs.

```bash
cargo test -p coding-adventures-powerpc601-gatelevel
cargo clippy -p coding-adventures-powerpc601-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-powerpc601-gatelevel --no-deps
```

The aggregate differential replays all 239 reproducible Python full-state
vectors from the functional package. One frozen legacy `sraw` vector was
source-corrected from clamp-to-31 CA behavior; the remaining 238 hashes are
unchanged. Primary-source edge tests independently cover relation, CR/CR0,
CA, `mtcrf`, modifier, producer-to-branch, snapshot/trace/rollback, and hostile
AST anti-bypass behavior. The normative completion contract is Spec 07u2.
The completed status cell has 95.38% package-source line coverage
(1,052/1,103), including 100% of `status.rs` (125/125).
