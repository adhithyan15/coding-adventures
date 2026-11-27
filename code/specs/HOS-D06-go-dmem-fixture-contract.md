# HOS-D06-GO2 — Portable observed D-MEM timing fixture contract

## Goal and boundary

Specify a future fixture-only corpus for the opt-in, one-outstanding precise
D-MEM behavior defined by HOS-D06-GO1 and implemented in the Go core. This
document defines the canonical guests, timing observations, architectural
references, JSON safety bounds, and independent-oracle rules. The canonical
input data is now materialized separately; this contract still does not add an
expected trace, JSON Schema, adapter, or language route.

The corpus must be language-neutral in its field names and values. Each
consumer must run the guest through its own native RV32I pipeline and emit the
same normalized observations. A captured trace is a comparison target, never
an execution input. The existing Go-only D05 fixture remains a separate
compatibility source and its files and hashes are not rewritten.

## Fixture location and shape

The fixture root is `code/specs/fixtures/go-riscv-dmem-ready-v1/`. It currently
contains canonical `input.json` data and a README. A later reviewed step may
add `expected_trace.json`, `schema.json`, and fixture-adjacent native adapters.
No package test imports the fixture or gains file access. A future adapter
must use the existing external fixture-driver convention.

`input.json` is a strict object with a versioned `schema` string and a `cases`
array. Each case has a stable `id`, precise flat-RAM mode, `latency_cycles`,
`ram_bytes`, `max_cycles`, a canonical RV32I program, and initial memory. The
compatibility case repeats the exact D05 program and initial RAM and carries a
tagged `source` reference to the D05 input path and SHA-256 below. The repeated
bytes make the corpus self-contained while the tag preserves provenance; a
consumer must verify they match the referenced input. Unknown properties are
rejected. `input.json` is canonical data, not yet backed by a published schema
or expected output. Its current SHA-256 is
`800486ce328d06263a6e8626964c45ec2b50aff1edbdb48c6a09b401c1213356`.

`expected_trace.json` is a strict object with a versioned `schema`, the SHA-256
of the exact `input.json` bytes, and one expected result per case. An expected
result contains:

- a full per-cycle normalized trace, or for the latency-one D05 case a
  path-and-SHA reference to the existing exact trace projection;
- ordered `request_events` with sequence, instruction PC, access kind,
  address, issue cycle, ordered wait-cycle list, completion status, and ready
  cycle;
- ordered precise `store_receipts` with PC, address, value, and WB commit
  cycle;
- `retired_pcs`, a typed `selected_fault` or null, and `fault_selection_cycle`
  / `terminal_cycle` when faulted;
- the full 32-entry `registers` array, the complete `ram_words` array, and
  aggregate cycles, retirement, forwarding, stall, flush, and D-MEM-wait
  counts.

Every trace item has the 1-based `cycle`, next-fetch `pc`, `stalled`,
`flushing`, and exactly the five keys IF/ID/EX/MEM/WB. A stage is null for an
empty slot or bubble; otherwise it records `pc` and `mnemonic`, with optional
typed `forwarded_operands`, a ready `load_result`, and a precise `fault` where
applicable. Stage PCs and next-fetch PC are unsigned RV32 values. A held
request's MEM stage retains its dynamic instruction PC across all wait
snapshots. The request event identifies which stalls are D-MEM waits, so an
ordinary load-use stall is not mislabeled.

## Canonical cases

### `d05-precise-latency-one`

This case reuses the exact existing input and its content identity:

- Source input: `code/specs/fixtures/go-riscv-d05-v1/input.json`
- Source SHA-256: `fcf69bf2c614f0119c10c2b5d8a1b742c2bf9c1e7d847650ee6c09e736930744`
- Program bytes: `13003006930000001301000823201100930110008321010063841100930230061382110073000000`
- RAM: 256 bytes; initial word at address 128 is 42; latency is 1.

The exact 16-cycle stage projection must equal the existing
`go-riscv-d05-v1/expected_trace.json` projection. Do not edit or recapture that
file. Precise mode adds this request/commit evidence:

| Sequence | PC | Access | Address | Issue | Wait cycles | Ready | WB commit |
| ---: | ---: | --- | ---: | ---: | --- | ---: | ---: |
| 1 | 12 | store | 128 | 7 | none | 7 | 8 |
| 2 | 20 | load | 128 | 9 | none | 9 | none |

The architecture oracle is the existing complete expected state: retired PCs
`[0,4,8,12,16,20,24,32,36]`; only nonzero registers are `x2=128`, `x4=1`; the
program words at addresses 0 through 36 are unchanged; every other RAM word,
including address 128, is zero at completion. The single precise store
receipt is PC 12, address 128, value 0, commit cycle 8. There are zero D-MEM
wait cycles; the existing one load-use stall remains. Expected aggregate
counts remain 16 cycles, 9 successful retirements, 2 forwards, 1 stall, and 1
redirect flush.

The existing latency-one pipeline trace is a compatibility comparison, not a
replacement for the independent sequential semantic oracle. The adapter also
executes the same bytes and initialization through the native sequential
`RiscVSimulator` and compares successful PCs, all registers, and all RAM
words.

### `delayed-store-load`

The guest is exactly five little-endian RV32I words:

| PC | Word | Instruction | Effect |
| ---: | ---: | --- | --- |
| 0 | `0x08000513` | `addi x10,x0,128` | data address |
| 4 | `0x03700093` | `addi x1,x0,55` | store value |
| 8 | `0x00152023` | `sw x1,0(x10)` | delayed older store |
| 12 | `0x00052103` | `lw x2,0(x10)` | ordered younger load |
| 16 | `0x00000073` | `ecall` | stop |

Canonical `program_bytes_hex` is
`1305000893007003232015000321050073000000`. Use RAM size 256 bytes, initial
word `{address:128,value:7}`, max cycles 100, and latency 3. Expected requests
and the store receipt are:

| Sequence | PC | Access | Address | Issue | Held cycles | Ready | WB commit |
| ---: | ---: | --- | ---: | ---: | --- | ---: | ---: |
| 1 | 8 | store | 128 | 6 | `[7,8]` | 8 | 9 |
| 2 | 12 | load | 128 | 9 | `[10,11]` | 11 | none |

The store releases to WB in cycle 9 while the load issues into MEM; it commits
once in that cycle. The load samples the newest older pending-store value 55
at issue and keeps the result unavailable through cycle 10. Value 55 is
attached as the load result at the end of its last wait cycle 11; release
arbitration is cycle 12. Wait-cycle snapshots 7 and 8 hold the IF/ID/EX/MEM
PCs with MEM=8, next-fetch PC 24, and WB empty. Wait snapshots 10 and 11 hold
those same stages with MEM=12, next-fetch PC 28, and WB empty. No second
request issues before the first is ready.

The sequential architecture reference retires `[0,4,8,12,16]`, ends with
`x1=55`, `x2=55`, `x10=128` and all other registers zero, and ends with
RAM[128]=55. Program bytes at addresses 0 through 16 stay unchanged; every
other data word is zero. The one store receipt is `{pc:8,address:128,value:55,
commit_cycle:9}`. Counts are 13 cycles, 5 successful retirements, 1 forward,
4 total stalls of which all 4 are D-MEM waits, and 0 flushes.

### `older-store-before-younger-fault`

The guest is exactly four little-endian RV32I words:

| PC | Word | Instruction | Effect |
| ---: | ---: | --- | --- |
| 0 | `0x02a00093` | `addi x1,x0,42` | older register effect |
| 4 | `0x08102023` | `sw x1,128(x0)` | older delayed store |
| 8 | `0x10002103` | `lw x2,256(x0)` | out-of-range younger fault |
| 12 | `0x06300f93` | `addi x31,x0,99` | younger effect to suppress |

Canonical `program_bytes_hex` is
`9300a0022320100803210010930f3006`. Use RAM size 256 bytes, initial word
`{address:128,value:7}`, max cycles 100, and latency 3. The store request is
sequence 1 at PC 4/address 128: issue cycle 5, held cycles `[6,7]`, ready at
cycle 7, WB commit at cycle 8. The invalid load is detected in EX but issues
no request. It remains in EX during both waits. Cycle 8 is the release/fault
selection cycle: IF/ID are empty or bubbles, the older store is in WB, and the
faulting load advances to MEM. The store effect commits once at cycle 8. The
terminal typed fault appears when PC 8 reaches WB in cycle 9:

```text
pc=8 raw_instruction=0x10002103 access=load
cause=load_access_fault address=256 width=4
fault_selection_cycle=8 terminal_cycle=9
```

Successful retirements are exactly `[0,4]`; PC 8 is reported only as the
fault, and PC 12 never retires or changes state. Final `x1=42`; all other
registers, including x2 and x31, are zero. RAM[128]=42; program words are
unchanged; all other data words are zero. There is one store receipt at cycle
8 and one request event for the store only. The sequential oracle runs only
the successful prefix (PC 0 and PC 4), then restores the complete program
image before comparing all registers and RAM. Counts are 9 cycles, 2
successful retirements, 1 forwarding operation, 2 stalls (both D-MEM waits),
and 0 flushes.

## Independent oracle derivation

The semantic oracle never reads `expected_trace.json`. For each nonfaulting
input it runs the same program and initial RAM through a fresh sequential
`RiscVSimulator`, and compares retired PCs, every architectural register, and
every RAM word. For the fault case it runs only the valid two-instruction
prefix through that independent simulator, restores the full guest image in
memory, and compares the prefix state. The precise fault identity, address,
width, selection order, and suppression of x2/x31 are checked directly against
the contract.

The timing oracle is a separate arithmetic model of the HOS-D06-GO1 cycle
convention, independent of the production request state and trace counters:

- latency 1: issue and ready at MEM-entry callback cycle `M`; no D-MEM hold;
- latency `L > 1`: issue at `M`, held cycles are exactly
  `M+1..M+L-1`, completion/ready is recorded at the final held-cycle boundary
  `M+L-1`, and release arbitration is `M+L`;
- request sequence is in guest order, with one request per valid aligned
  word access and none for faulting or squashed accesses;
- every wait snapshot preserves IF/ID/EX/MEM and next-fetch PC, clears WB,
  and performs no callback, request issue, WB callback, or retirement;
- at release the held MEM token advances to WB once; a store receipt is
  timestamped at that WB cycle, distinct from its earlier timing-ready cycle;
- when a younger EX fault waits behind an older MEM request, the wait action
  wins until release, then the older effect reaches WB before the terminal
  fault outcome.

The expected trace records the actual engine's per-cycle projection. The
independent timing model checks the explicit rows above against the recording;
it does not manufacture or feed instructions from those rows. For the
latency-one D05 case, separately compare the legacy trace against the existing
published trace file without changing it.

## Strict bounds and JSON safety

The eventual schema and fixture drivers must enforce these bounds before
execution:

- exactly three named cases, no unknown case IDs or object properties;
- 1 to 32 RV32I instructions; nonempty, even-byte, 4-byte-aligned
  `program_bytes_hex` in lowercase canonical hex;
- RAM size is a multiple of four from 4 through 65,536 bytes; program fits;
- initial words are aligned, in range, unique by address, and uint32 values;
- latency is a positive integer no greater than 400; max cycles is 1 through
  400; captured trace length and all cycle indices are bounded by max cycles;
- all JSON numeric fields are finite safe integers (`abs(value) <=
  9,007,199,254,740,991`); architectural values and addresses additionally
  obey uint32 bounds, and PCs/access sizes obey word alignment as applicable;
- request sequence values are 1-based contiguous integers; issue and ready
  cycles are positive; wait cycles are strictly ordered and exactly match the
  timing formula; completion is true exactly once;
- register arrays contain 32 uint32 values; `ram_words` length is exactly
  `ram_bytes / 4`; retired PCs, store receipts, and typed faults have strict
  per-case shapes.

`expected_trace.json` is compared only after native execution. A fixture-only
adapter may provide an explicit recapture command, but must first run the
independent semantic and arithmetic timing assertions before writing the
expected file. No package-level capability grant, shared package source,
original D05 fixture mutation, or VOS change is part of this contract.
