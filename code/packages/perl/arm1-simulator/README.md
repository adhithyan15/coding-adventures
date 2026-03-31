# coding-adventures-arm1-simulator (Perl)

A complete behavioral simulator for the ARM1 processor — the very first ARM chip,
designed by Sophie Wilson and Steve Furber at Acorn in 1985.

## What is the ARM1?

The ARM1 (Advanced RISC Machine 1) introduced the load-store architecture,
conditional execution on every instruction, and the barrel shifter that still
define ARM chips today. It is a RISC processor with:

- 32-bit fixed-length instructions
- 16 visible registers (R0–R15), with R15 combining PC, flags, and mode bits
- 25 physical registers across 4 modes with banked registers for fast context switch
- A barrel shifter on every data processing instruction (shift for free)
- All 16 condition codes on every instruction (predication before it had a name)
- 26-bit address space (ARMv1 era)
- 3-stage pipeline: Fetch → Decode → Execute (PC = current_addr + 8)

## Package Contents

```
lib/
  CodingAdventures/
    ARM1Simulator.pm   -- complete simulator
t/
  00-load.t            -- module loads cleanly
  01-arm1-simulator.t  -- comprehensive tests
Makefile.PL
cpanfile
BUILD
BUILD_windows
```

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::ARM1Simulator;

my $ARM1 = 'CodingAdventures::ARM1Simulator';

# Create a simulator with 4096 bytes of RAM
my $cpu = $ARM1->new(4096);

# Build and load a tiny program:  MOV R0, #42  + HALT
$cpu->load_instructions([
    $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 42),
    $ARM1->encode_halt(),
]);

# Run up to 100 steps; returns an arrayref of traces
my $traces = $cpu->run(100);

printf "R0 = %d\n", $cpu->read_register(0);   # 42
printf "Steps: %d\n", scalar @$traces;
```

## API

### Constructor

```perl
my $cpu = CodingAdventures::ARM1Simulator->new($memory_size);
```

### Registers

```perl
$cpu->read_register($n);      # read R0–R15 (mode-banked)
$cpu->write_register($n, $v); # write R0–R15
$cpu->get_pc();               # return PC (bits 25:2 of R15, shifted)
$cpu->get_mode();             # return mode bits (0=USR, 1=FIQ, 2=IRQ, 3=SVC)
$cpu->get_flags();            # return { n, z, c, v } hashref
```

### Memory

```perl
$cpu->read_word($addr);        # read 32-bit little-endian word
$cpu->write_word($addr, $val); # write 32-bit little-endian word
$cpu->read_byte($addr);
$cpu->write_byte($addr, $val);
```

### Execution

```perl
$cpu->load_instructions(\@words); # write instruction words starting at address 0
$cpu->step();                     # execute one instruction; returns trace hashref
$cpu->run($max_steps);            # run until halt or max_steps; returns arrayref of traces
$cpu->reset();                    # zero all registers and memory
```

### Encoding helpers

```perl
# All are class or package methods (no object needed)
$ARM1->encode_mov_imm($cond, $rd, $imm8);
$ARM1->encode_alu_reg($cond, $opcode, $s, $rd, $rn, $rm);
$ARM1->encode_alu_reg_shift($cond, $opcode, $s, $rd, $rn, $rm, $shift_type, $shift_amount);
$ARM1->encode_branch($cond, $link, $byte_offset);
$ARM1->encode_ldr($cond, $rd, $rn, $offset, $writeback);
$ARM1->encode_str($cond, $rd, $rn, $offset, $writeback);
$ARM1->encode_ldm($cond, $rn, $reg_list, $writeback, $mode);  # mode = 'IA','IB','DA','DB'
$ARM1->encode_stm($cond, $rn, $reg_list, $writeback, $mode);
$ARM1->encode_halt();  # encodes SWI 0x123456
```

### Constants

```perl
# Condition codes
$ARM1->COND_EQ, COND_NE, COND_CS, COND_CC, COND_MI, COND_PL,
COND_VS, COND_VC, COND_HI, COND_LS, COND_GE, COND_LT,
COND_GT, COND_LE, COND_AL, COND_NV

# ALU opcodes
$ARM1->OP_AND, OP_EOR, OP_SUB, OP_RSB, OP_ADD, OP_ADC,
OP_SBC, OP_RSC, OP_TST, OP_TEQ, OP_CMP, OP_CMN,
OP_ORR, OP_MOV, OP_BIC, OP_MVN

# Shift types
$ARM1->SHIFT_LSL, SHIFT_LSR, SHIFT_ASR, SHIFT_ROR

# Processor modes
$ARM1->MODE_USR, MODE_FIQ, MODE_IRQ, MODE_SVC

# R15 flag bits
$ARM1->FLAG_N, FLAG_Z, FLAG_C, FLAG_V, FLAG_I, FLAG_F
```

## Where It Fits

```
arm1_gatelevel   ← gate-level simulation of ARM1 (built from logic-gates)
arm1_simulator   ← this package (behavioral / functional simulation)
arm_simulator    ← higher-level ARM family simulator
```

## Running Tests

```bash
prove -l -v t/
```
