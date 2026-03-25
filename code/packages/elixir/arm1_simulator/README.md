# ARM1 Behavioral Simulator (Elixir)

A behavioral simulator for the ARM1 processor — the first ARM chip, designed
by Sophie Wilson and Steve Furber at Acorn Computers in 1984-1985.

The ARM1 was a 32-bit RISC processor with just 25,000 transistors. It famously
worked correctly on its very first power-on (April 26, 1985).

## Features

- Complete ARMv1 instruction set
- 16 data processing operations (AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC,
  TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN)
- Load/store (LDR, STR, LDRB, STRB with pre/post-indexed addressing)
- Block transfer (LDM, STM with all four stacking modes)
- Branch (B, BL)
- Software interrupt (SWI)
- Conditional execution on every instruction (16 condition codes)
- Inline barrel shifter (LSL, LSR, ASR, ROR, RRX)
- 4 processor modes with banked registers (USR, FIQ, IRQ, SVC)
- Immutable functional design: `step(cpu)` returns `{new_cpu, trace}`

## Usage

```elixir
alias CodingAdventures.Arm1Simulator, as: Sim

cpu = Sim.new(1024)
cpu = Sim.load_instructions(cpu, [
  Sim.encode_mov_imm(Sim.cond_al(), 0, 1),   # MOV R0, #1
  Sim.encode_mov_imm(Sim.cond_al(), 1, 2),   # MOV R1, #2
  Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 2, 0, 1),  # ADD R2, R0, R1
  Sim.encode_halt()
])
{cpu, traces} = Sim.run(cpu, 10)
Sim.read_register(cpu, 2)  #=> 3
```

## Development

```bash
mix deps.get
mix test
```
