# GE-225 Simulator (Python)

Behavioral Python simulator for the **GE-225 instruction repertoire**.

This package is intentionally narrower than a full Dartmouth Time-Sharing System
emulator. It is the first executable backend target for the GE-225 spec:

`Dartmouth BASIC frontend -> Semantic IR -> GE-225 backend -> GE-225 simulator`

## Scope

- 20-bit word-addressed memory
- historical base memory-reference instruction form
- documented fixed-word instructions such as `LQA`, `XAQ`, `NOP`, `RCS`, `TON`, `TYP`
- documented shift/normalize instruction families such as `SRA`, `SLD`, `NOR`, `SAN`, `SNA`
- index-group state and skip-style branch/test behavior
- simple host-side card-reader queue and console typewriter buffer
- frozen machine-state snapshots
- protocol-style `execute()` entry point for end-to-end testing

## Instruction Model

The package is organized around the **documented GE-225 mnemonics and octal
instruction forms**, not a private backend-only opcode map. The current focus is
the central processor plus development-time host hooks for a few console/device
paths, rather than a full DTSS or peripheral recreation.

## Implemented Families

- Base arithmetic/data movement:
  `LDA`, `ADD`, `SUB`, `STA`, `DLD`, `DAD`, `DSU`, `DST`, `MPY`, `DVD`
- Base transfer/modify/branch:
  `LDX`, `STX`, `INX`, `BXL`, `BXH`, `SPB`, `EXT`, `CAB`, `DCB`, `ORY`, `BRU`, `STO`
- Block/data movement:
  `MOY`, `RCD`
- Fixed-word core commands:
  `LDZ`, `LDO`, `LMO`, `CPL`, `NEG`, `CHS`, `NOP`, `LAQ`, `LQA`, `XAQ`, `MAQ`, `ADO`, `SBO`
- Console and mode commands:
  `RCS`, `TON`, `TYP`, `OFF`, `HPT`, `SET_DECMODE`, `SET_BINMODE`, `SET_PST`, `SET_PBK`
- Branch/test commands:
  `BOD`, `BEV`, `BMI`, `BPL`, `BZE`, `BNZ`, `BOV`, `BNO`, `BPE`, `BPC`, `BNR`, `BNN`
- Shift/normalize families:
  `SRA`, `SLA`, `SCA`, `SAN`, `SNA`, `SRD`, `NAQ`, `SCD`, `ANQ`, `SLD`, `NOR`, `DNO`

## Running Tests

```bash
uv venv --quiet --clear
uv pip install -e ../simulator-protocol -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```
