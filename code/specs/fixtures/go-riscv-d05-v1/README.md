# Go RV32I classic pipeline case, version 1

This test-only fixture captures one bounded D05 guest on the Go core's classic
IF/ID/EX/MEM/WB pipeline. `input.json` supplies the guest and initial RAM;
`expected_trace.json` is the fixed actual-engine per-cycle capture. The replay
test independently runs `RiscVSimulator` and compares retired PCs, all 32
registers, and all 64 aligned RAM words.

The case checks two-source EX/MEM forwarding into a store, a load-use hold and
MEM forward into a taken branch, one redirect flush, and wrong-path suppression.
It uses flat RAM and has no precise fault or store-at-retirement claim. The
store takes effect at MEM, before the store's WB retirement.

The expected output embeds the SHA-256 of the exact input file. Current content
identities:

- `input.json`: `fcf69bf2c614f0119c10c2b5d8a1b742c2bf9c1e7d847650ee6c09e736930744`
- `expected_trace.json`: `6183d3477816862d3dd504a12310b2e91ded137519485d740e0b5a57e68a93a7`

To recapture from the actual Go pipeline after reviewing a fixture change, run
from `code/packages/go/riscv-simulator`:

```bash
UPDATE_GO_D05_TRACE=1 go test ./... -run '^TestGoD05FixtureMatchesCapturedFiveStageTraceAndSequentialOracle$' -count=1
```

The replay test also runs its independent oracle and semantic assertions during
recapture. Update these digests after an intentional input or trace change.
This capture is Go-specific evidence and is not a shared cross-language golden.
