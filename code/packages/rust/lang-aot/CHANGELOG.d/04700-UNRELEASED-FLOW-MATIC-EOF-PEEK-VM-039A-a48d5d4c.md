## Unreleased — FLOW-MATIC EOF peek (VM-039a)

Support the zero-argument `input_more` builtin through the shared C runtime.
It returns 1 when input remains and 0 at EOF without consuming the next field.
Native/LLVM matrix proofs cover finite streams, empty input, partial records
and field preservation after EOF. Other backend adapters remain follow-up work.


