# CodingAdventures::Intel8008Simulator

Behavioral simulator for the Intel 8008 (April 1972) — the world's first
commercial 8-bit microprocessor and the ancestor of the x86 architecture.

## Position in the Stack

```
Logic Gates → Arithmetic → CPU → [This Package] → Assembler → Lexer → Parser
```

Implements behavioral simulation (host-language arithmetic) rather than
gate-level simulation. For gate-level, see `intel8008-gatelevel`.

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 8 bits |
| Registers | A, B, C, D, E, H, L (7 × 8-bit) + M (memory pseudo-register) |
| Program counter | 14 bits (0x0000–0x3FFF) |
| Stack | 8-level internal push-down (entry 0 = PC) |
| Memory | 16,384 bytes |
| Flags | Carry (CY), Zero (Z), Sign (S), Parity (P) |
| I/O | 8 input ports, 24 output ports |

## Usage

```perl
use CodingAdventures::Intel8008Simulator;

my $cpu = CodingAdventures::Intel8008Simulator->new();

# 1 + 2 = 3
my $traces = $cpu->run([
    0x06, 0x01,   # MVI B, 1
    0x3E, 0x02,   # MVI A, 2
    0x80,         # ADD B
    0x76,         # HLT
]);

print "A = ", $cpu->a, "\n";            # 3
print "Carry = ", $cpu->flags->{carry}, "\n";  # 0
print "Parity = ", $cpu->flags->{parity}, "\n"; # 1 (even: 2 ones in 0x03)
```

## Instruction Set Summary

| Group | Instructions |
|-------|-------------|
| Data movement | MOV, MVI |
| Increment/Decrement | INR, DCR |
| ALU register | ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP |
| ALU immediate | ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI |
| Rotate | RLC, RRC, RAL, RAR |
| Jump | JMP, JFC/JTC, JFZ/JTZ, JFS/JTS, JFP/JTP |
| Call | CAL, CFC/CTC, CFZ/CTZ, CFS/CTS, CFP/CTP |
| Return | RET, RFC/RTC, RFZ/RTZ, RFS/RTS, RFP/RTP |
| Restart | RST 0–7 (1-byte call to address N×8) |
| I/O | IN 0–7, OUT 0–23 |
| Halt | HLT (0x76 and 0xFF) |

## Historical Note

The 8008 was designed for CTC's Datapoint 2200 terminal. CTC rejected it for
being too slow. Intel sold it commercially in April 1972. It inspired the 8080,
which inspired the Z80 and x86 — making every modern Intel/AMD processor a
distant descendant of this chip's architecture.

## Running Tests

```bash
cpanm --notest --quiet Test2::V0
prove -l -v t/
```
