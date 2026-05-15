# riscv-rv64i-simulator

RISC-V RV64I + M extension behavioral simulator — Layer 07y in the
coding-adventures simulator series.

## What it implements

- Full **RV64I** base integer instruction set (64-bit)
- **M extension**: integer multiply and divide (64-bit and 32-bit word forms)
- SIM00 simulator protocol: `reset`, `load`, `step`, `execute`, `get_state`

## Usage

```python
from riscv_rv64i_simulator import RV64ISimulator

sim = RV64ISimulator()
# ADDI x10, x0, 42 (0x02A00513) + halt (0x00000000)
state = sim.execute(bytes.fromhex("1305A002" + "00000000"))
print(state.a0)  # 42
```

## Architecture

- 32 × 64-bit integer registers (x0–x31); x0 hardwired to zero
- Fixed 32-bit instruction width; little-endian
- 64 KiB flat memory (addresses masked to 16 bits)
- Halt sentinel: 32-bit zero word `0x00000000`

## Layer position

```
07a riscv (RV32I minimal) → ... → 07x ARMv7-A → [07y YOU ARE HERE] → 07z Apple M1
```

## Running tests

```bash
uv venv && uv pip install -e ../simulator-protocol -e ".[dev]"
python -m pytest tests/ -v
```
