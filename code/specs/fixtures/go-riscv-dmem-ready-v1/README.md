# RV32I D-MEM ready/wait input data, version 1

`input.json` is the canonical, portable input data for the three cases in
[`../../HOS-D06-go-dmem-fixture-contract.md`](../../HOS-D06-go-dmem-fixture-contract.md).
`input.schema.json` defines its strict JSON shape. `expected_oracle.json` and
`expected_oracle.schema.json` contain the independently derived timing events
and architectural end states. The oracle file is not a pipeline trace: it
contains no per-cycle IF/ID/EX/MEM/WB projection. `expected_trace.json` holds
the actual Go pipeline's normalized per-cycle snapshots for the two latency-3
cases and a SHA reference to the unchanged legacy D05 projection;
`expected_trace.schema.json` defines this format. The external Go runner
independently checks the sequential architectural state and timing arithmetic
before comparing the snapshots. Its explicit `-record` option can update only
`expected_trace.json` after those checks pass. The runner is not a package
test and adds no package filesystem capability or repository-input route.

The `d05-precise-latency-one` case repeats the exact program and initial memory
from `../go-riscv-d05-v1/input.json`. Its `source` field names that input and
stores its SHA-256 so consumers can verify provenance. The two new latency-3
guests are encoded as canonical lowercase little-endian bytes.

Content identity:

- `input.json`: `800486ce328d06263a6e8626964c45ec2b50aff1edbdb48c6a09b401c1213356`
- `input.schema.json`: `67115aad438079b448d0c467a29a7b20197ad5fb36541fbfb0389a088c0d3149`
- `expected_oracle.json`: `3b49a95bc30a02a95cebd64f811419229c6e69c23f314d751a1298d69751d700`
- `expected_oracle.schema.json`: `b607659809f7edc9b36ca92614ab8776bab1e54c8b30f8db9a7bc7236e2e371b`
- `expected_trace.json`: `dc9ef0278603b9b17273df5b3aa236a8c7f3460b20660b8a551366965c075bc4`
- `expected_trace.schema.json`: `191837c7ccd4c6d2084705c169583f29abdaba991adcacc74555cdc005547175`

Before recording the pipeline trace, the adapter validates the pinned input
and oracle content identities and the strict case bounds. A native sequential
`RiscVSimulator` independently executes each successful guest; for the fault
case it executes only the successful prefix and restores the full program
image before comparing all registers and RAM. An independent access walk
checks valid load/store PCs, addresses, values, and that the faulting load
issues no request. A separate arithmetic checker verifies the wait-cycle and
ready-cycle equations and store WB timing. The adapter then runs the precise
Go pipeline, compares its state, events, counters and fault against that
oracle, and only then compares or records the full trace. The existing D05
trace and input files are never rewritten.

The original input/oracle data checks used a small independent RV32I
decoder/interpreter for the supported ADDI, LW, SW, BEQ, and ECALL instructions.
It confirmed each instruction word, successful retirement prefixes, final
registers and word at address 128, and the out-of-range load at address 256.
The D-MEM issue/wait/ready/release tuples were checked separately using the
arithmetic cycle convention in the contract. No pipeline implementation
produced these checks. The input, oracle, and captured trace validate against
their Draft 2020-12 schemas; unknown root properties were rejected in a
negative check. Cross-field properties such as unique initial addresses,
program fit, event ordering, and exact latency formulas are also checked by
the external Go runner.

From `code/packages/go/riscv-simulator`, the standard BUILD fronts run:

```sh
go run ../../../specs/fixtures/go-riscv-dmem-ready-v1/test-adapters/go/main.go
```

Use `-record` explicitly only after reviewing the independent oracle; normal
replay compares against the committed trace.
