# CPU Simulator (Rust)

Minimal CPU simulator types for ISA-specific simulators. Provides:

- `RegisterFile` -- fixed-size array of 32-bit registers with optional zero-register hardwiring
- `Memory` -- byte-addressable memory with little-endian word operations

## Usage

```rust
use cpu_simulator::{RegisterFile, Memory};

let mut regs = RegisterFile::new(32, true); // 32 regs, x0 hardwired to 0
regs.write(1, 42);
assert_eq!(regs.read(1), 42);
assert_eq!(regs.read(0), 0); // always 0

let mut mem = Memory::new(65536);
mem.write_word(0x100, 0xDEADBEEF);
assert_eq!(mem.read_word(0x100), 0xDEADBEEF);
```
