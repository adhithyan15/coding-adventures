# off-by-one-consumed-v1

The oracle's committed proof of the fault `off-by-one-consumed` (VER04r 8.2, 16.10 [D36], slice S10o-1).

The fault turns the **second** component `n` of a tuple result (or of the tuple inside an `Ok`) into `n + 1`
when `n` is even and `n - 1` when it is odd, the rule of the Rust harness. Section 8.2 gives its exact JSON
(`requires` `["ir.fixed", "ir.tuple"]`), so unlike the composite faults of `tuple-forms-v1` (R11) the spec, not
the dump, is normative: the dump here must reproduce it.

## Files

| File | What it is |
|---|---|
| `make_off_by_one_consumed.py` | writes (or `--check`s) the four files below; `BUILD` runs `--check` |
| `off_by_one_consumed_v1.warrant` | generated synthetic document: bare tuples with a `Nat`, `U8` and `I8` second component, `Result<(U64,Nat),E>` and `Result<(I64,Nat),E>` (the wasm-leb128 decoder shapes), `Result<(Bool,U8),E>`, a 3-tuple; `dec_u64` also carries a second claim over its `Err` region (`n == 7`), blind for this fault |
| `off_by_one_consumed_v1.profile` | the `corpus.profile`: one `fault <op> off-by-one-consumed` line per op |
| `off_by_one_consumed_v1.oracle-faults.json` | `parity_dumps.faults_view` under the open gate: every fault row, and the controls under the explicit closed gate (`controls_shipped`/`gate_shipped`, a historical key name shared with `fixed-eval-v1`; `GATE_CLOSED`) and the open gate (`controls_gate_on`/`gate_on`, `GATE_OPEN`, what both tools ship since the S10c flip) |
| `off_by_one_consumed_v1.oracle-refusals.json` | every signature the fault refuses (scalar, `Bytes`, `Result<Bytes,E>`, non-numeric second component, `ok_payload` clash, and both defects at once, where the second component is reported first), with the oracle's code and message |

`.gitattributes` (`* -text`) keeps the bytes exact on every platform.

## Why a directory of its own

The generator's `tuple_parity.rs` reproduces the dumps of `tuple-forms-v1` byte for byte; a new fault row there
would turn the generator red until its mirror landed. Here nothing exists to break.

## The contract with the generator

Slice S10o-2 (`rust/warrant-corpus-gen`) must reproduce `oracle-faults.json` byte for byte (the fault rows and the
controls) and refuse the signatures of `oracle-refusals.json` with the code `fault`.
The one exception is the `Option` row (`an-option-result`): a real generator refuses an `Option` in a signature earlier, at lowering, with the level `ir.data` ([D23]; `tests/off_by_one_consumed_parity.rs` asserts it); only the oracle's bare `fault_row` reaches the `fault` code for it. The dumps carry no generator
name or version. Regenerate with `python3 make_off_by_one_consumed.py`; never edit a file by hand.
