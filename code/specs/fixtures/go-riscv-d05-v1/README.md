# Go RV32I classic pipeline case, version 1

This test-only fixture captures one bounded D05 guest on the Go core's classic
IF/ID/EX/MEM/WB pipeline. `input.json` supplies the guest and initial RAM;
`expected_trace.json` is the fixed actual-engine per-cycle capture. The
external harness independently runs `RiscVSimulator` and compares retired
PCs, all 32 registers, and all 64 aligned RAM words. The publishable Go
package's BUILD and tests do not read these fixture files or request host
capabilities.

The case checks two-source EX/MEM forwarding into a store, a load-use hold and
MEM forward into a taken branch, one redirect flush, and wrong-path suppression.
It uses flat RAM and has no precise fault or store-at-retirement claim. The
store takes effect at MEM, before the store's WB retirement.

The expected output embeds the SHA-256 of the exact input file. Current content
identities:

- `input.json`: `fcf69bf2c614f0119c10c2b5d8a1b742c2bf9c1e7d847650ee6c09e736930744`
- `expected_trace.json`: `6183d3477816862d3dd504a12310b2e91ded137519485d740e0b5a57e68a93a7`

To replay from the actual Go pipeline, run from `code/packages/go/riscv-simulator`:

```bash
go run ../../../specs/fixtures/go-riscv-d05-v1/test-adapters/go/main.go
```

Append `-update` to recapture after reviewing an intentional fixture change;
the harness runs its independent oracle and semantic assertions during
recapture. Update these digests after changing fixture contents. This capture
is Go-specific evidence and is not a shared cross-language golden.
