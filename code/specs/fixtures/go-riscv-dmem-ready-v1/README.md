# RV32I D-MEM ready/wait input data, version 1

`input.json` is the canonical, portable input data for the three cases in
[`../../HOS-D06-go-dmem-fixture-contract.md`](../../HOS-D06-go-dmem-fixture-contract.md).
It records the guest bytes, initial words, flat-RAM size, cycle budget, and
configured memory latency. It contains no captured expected trace or runtime
adapter yet; this checkpoint adds no route and makes no cross-language
execution claim.

The `d05-precise-latency-one` case repeats the exact program and initial memory
from `../go-riscv-d05-v1/input.json`. Its `source` field names that input and
stores its SHA-256 so consumers can verify provenance. The two new latency-3
guests are encoded as canonical lowercase little-endian bytes.

Content identity:

- `input.json`: `800486ce328d06263a6e8626964c45ec2b50aff1edbdb48c6a09b401c1213356`

Validation for this data-only checkpoint used a small independent RV32I
decoder/interpreter for the supported ADDI, LW, SW, BEQ, and ECALL instructions.
It confirmed each instruction word, successful retirement prefixes, final
registers and word at address 128, and the out-of-range load at address 256.
The D-MEM issue/wait/ready/release tuples were checked separately using the
arithmetic cycle convention in the contract. No pipeline implementation
produced these checks.
