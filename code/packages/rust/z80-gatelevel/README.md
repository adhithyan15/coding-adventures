# z80-gatelevel

Gate-level simulator for the **Zilog Z80** (1976) microprocessor.

Every arithmetic and logic operation routes through real gate primitives from the
`logic-gates` and `arithmetic` crates — no host integer arithmetic in the execution
path. Registers are modelled as D flip-flop arrays.

## Architecture

```
bits.rs        — integer ↔ LSB-first bit-vector helpers; 8/16-bit adders with half-carry
alu.rs         — ALUZ80: all 8-bit and 16-bit operations through gate chains
registers.rs   — RegisterFile with main/alternate banks; IX, IY; flag pack/unpack
cpu.rs         — GateLevelCpuZ80: full Z80 instruction set including CB/ED/DD/FD prefixes
```

## Quick start

```rust
use coding_adventures_z80_gatelevel::GateLevelCpuZ80;

let mut cpu = GateLevelCpuZ80::new();
// LD A, 5; LD B, 3; ADD A, B; HALT
let (traces, state) = cpu.run(&[0x3E, 0x05, 0x06, 0x03, 0x80, 0x76], 100);
assert_eq!(state.a, 8);
assert!(!state.flag_c);
```

## Z80 Flag register

```
Bit 7  S   Sign       — bit 7 of result
Bit 6  Z   Zero       — result == 0
Bit 5  Y   (undocumented, set to 0)
Bit 4  H   Half-carry — carry from bit 3 (add) / NOT(adder_hc) (sub)
Bit 3  X   (undocumented, set to 0)
Bit 2  P/V Parity (logical ops) / Overflow (arithmetic ops)
Bit 1  N   Subtract   — 1 after SUB/SBC/DEC/CP/NEG
Bit 0  C   Carry / Borrow
```

## Key differences from Intel 8080

| Feature         | Intel 8080          | Zilog Z80                      |
|-----------------|---------------------|--------------------------------|
| Flags           | S, Z, AC, P, CY     | S, Z, H, P/V, N, C + 2 undoc  |
| N flag          | None                | Subtract indicator for DAA     |
| Alternate bank  | None                | A',F',B',C',D',E',H',L'        |
| Index registers | None                | IX, IY with signed displacement|
| I/O             | IN/OUT (8-bit port) | 256 ports; also IN r,(C)       |
| Prefixes        | None                | CB, ED, DD, FD, DDCB, FDCB    |
| DAA             | After ADD only      | After ADD and SUB (uses N)     |

## Instruction set coverage

### Unprefixed (all standard opcodes)
- **LD**: reg-reg, immediate, (HL), (BC), (DE), (nn), 16-bit pairs
- **ALU**: ADD, ADC, SUB, SBC, AND, XOR, OR, CP (8-bit; all with register/immediate/memory)
- **INC/DEC**: 8-bit registers, 16-bit pairs, (HL)
- **ADD HL, rp**: 16-bit add (only H/N/C affected)
- **Rotates**: RLCA, RRCA, RLA, RRA (accumulator; only C affected)
- **Jumps**: JP, JP cc, JP (HL), JR, JR cc (NZ/Z/NC/C), DJNZ
- **Calls/Returns**: CALL, CALL cc, RET, RET cc, RST
- **Stack**: PUSH/POP all pairs (AF, BC, DE, HL)
- **Exchange**: EX DE,HL; EX AF,AF'; EXX; EX (SP),HL
- **I/O**: IN A,(n); OUT (n),A
- **Misc**: DAA, CPL, CCF, SCF, NOP, HALT, DI, EI

### CB prefix (rotate/shift/bit)
- **Rotates**: RLC, RRC, RL, RR (all registers + (HL))
- **Shifts**: SLA, SRA, SRL; SLL (undocumented)
- **Bit ops**: BIT, SET, RES (bits 0–7 × all registers + (HL))

### ED prefix (extended)
- **16-bit arithmetic**: ADC HL,rp; SBC HL,rp (all flags updated)
- **16-bit indirect**: LD rp,(nn); LD (nn),rp
- **NEG**: negate accumulator
- **Block**: LDI, LDD, LDIR, LDDR; CPI, CPD, CPIR, CPDR
- **I/O**: IN r,(C); OUT (C),r
- **Special**: LD A,I; LD A,R; LD I,A; LD R,A
- **Interrupt**: IM 0/1/2; RETI; RETN

### DD/FD prefix (IX/IY indexed)
- LD IX,nn; LD IX,(nn); LD (nn),IX; LD SP,IX; PUSH IX; POP IX
- LD r,(IX+d); LD (IX+d),r; LD (IX+d),n
- ALU A,(IX+d); INC/DEC (IX+d)
- ADD IX,rp; INC IX; DEC IX; JP (IX); EX (SP),IX

### DDCB/FDCB prefix
- BIT/SET/RES/rotation ops on (IX+d) / (IY+d)

## Gate count estimate

| Component                      | Gates |
|-------------------------------|-------|
| 8-bit ALU (add/sub/log/rot)   | ~145  |
| 16-bit adder (HL/IX ops)      | ~80   |
| Main registers (8×8-bit)      | ~512  |
| Alternate bank (8×8-bit)      | ~512  |
| IX, IY (2×16-bit)             | ~256  |
| SP, PC (2×16-bit)             | ~256  |
| Instruction decoder           | ~60   |
| Control + wiring              | ~200  |
| **Total**                     | **~2,021** |

Real Z80: ~8,500 transistors (~2,125 gate equivalents).

## Package layout

Part of the `coding-adventures` gate-level simulator series:

| Layer | Package                     | Processor      | Year |
|-------|-----------------------------|----------------|------|
| 07d2  | `intel4004-gatelevel`       | Intel 4004     | 1971 |
| 07f2  | `intel8008-gatelevel`       | Intel 8008     | 1972 |
| 07i2  | `intel8080-gatelevel`       | Intel 8080     | 1974 |
| 07j2  | `mos6502-gatelevel`         | MOS 6502       | 1975 |
| **07k2** | **`z80-gatelevel`**     | **Zilog Z80**  | **1976** |
