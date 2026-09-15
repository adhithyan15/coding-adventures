## 0.202.0 - 2026-07-11 (FLOW-MATIC frontend — PL09 codegen)

Adds `Language::FlowMatic`, wiring the new `flow-matic-iir-compiler` into the
driver so FLOW-MATIC compiles to `interpreter_ir::IIRModule` and runs on every
execution backend. This slice covers FLOW-MATIC's control flow + scalar-field
moves (operations→labels, `COMPARE`/`IF`/`OTHERWISE`/`GO TO`/`JUMP`/`STOP`,
`MOVE`); record/file I/O is a later rung. First step of the PL09 arc pivoting
FLOW-MATIC and COBOL onto the IIR (execution) and SIR (transpile) pipelines.

