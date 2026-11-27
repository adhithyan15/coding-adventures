# RV32I D-MEM ready/wait input data, version 1

`input.json` is the canonical, portable input data for the three cases in
[`../../HOS-D06-go-dmem-fixture-contract.md`](../../HOS-D06-go-dmem-fixture-contract.md).
`input.schema.json` defines its strict JSON shape. `expected_oracle.json` and
`expected_oracle.schema.json` contain the independently derived timing events
and architectural end states. The oracle file is not a pipeline trace: it
contains no per-cycle IF/ID/EX/MEM/WB projection. `expected_trace.json`, its
schema, and a runtime adapter remain future work. This checkpoint adds no
route and makes no cross-language execution claim.

The `d05-precise-latency-one` case repeats the exact program and initial memory
from `../go-riscv-d05-v1/input.json`. Its `source` field names that input and
stores its SHA-256 so consumers can verify provenance. The two new latency-3
guests are encoded as canonical lowercase little-endian bytes.

Content identity:

- `input.json`: `800486ce328d06263a6e8626964c45ec2b50aff1edbdb48c6a09b401c1213356`
- `input.schema.json`: `67115aad438079b448d0c467a29a7b20197ad5fb36541fbfb0389a088c0d3149`
- `expected_oracle.json`: `3b49a95bc30a02a95cebd64f811419229c6e69c23f314d751a1298d69751d700`
- `expected_oracle.schema.json`: `b607659809f7edc9b36ca92614ab8776bab1e54c8b30f8db9a7bc7236e2e371b`

Validation for this data-only checkpoint used a small independent RV32I
decoder/interpreter for the supported ADDI, LW, SW, BEQ, and ECALL instructions.
It confirmed each instruction word, successful retirement prefixes, final
registers and word at address 128, and the out-of-range load at address 256.
The D-MEM issue/wait/ready/release tuples were checked separately using the
arithmetic cycle convention in the contract. No pipeline implementation
produced these checks. The input and oracle data validate against their Draft
2020-12 schemas. Cross-field properties such as unique initial addresses,
program fit, event ordering, and exact latency formulas remain driver
validation duties.
