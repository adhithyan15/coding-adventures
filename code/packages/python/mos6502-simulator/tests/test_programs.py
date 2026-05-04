"""End-to-end programs: sum 1..N, multiply, Fibonacci.

All programs use zero-page addresses $40+ for data storage so that there is no
overlap with the code bytes (which are loaded at address $0000 and typically
span $00..$30).  Using low ZP addresses like $10 causes self-modification
because the CPU's flat address space means ZP $10 IS the same byte as the
instruction at code address $0010.
"""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


class TestSumProgram:
    """Sum 1..N using a DEX / BNE loop, result in A."""

    def test_sum_1_to_5(self) -> None:
        # Sum 1..5 = 15 = 0x0F
        # ZP $40 holds the running sum.
        #
        # $00: A9 00    LDA #0
        # $02: 85 40    STA $40      ; sum = 0
        # $04: A2 05    LDX #5       ; counter = 5
        # loop ($06):
        # $06: 8A       TXA          ; A = X (current term)
        # $07: 18       CLC
        # $08: 65 40    ADC $40      ; A = term + sum
        # $0A: 85 40    STA $40      ; sum = A
        # $0C: CA       DEX
        # $0D: D0 F7    BNE -9 → $06 (after $0F: $0F + (-9) = $06)
        # $0F: A5 40    LDA $40      ; load final sum
        # $11: 00       BRK
        prog = bytes([
            0xA9, 0x00,        # $00
            0x85, 0x40,        # $02
            0xA2, 0x05,        # $04
            0x8A,              # $06  ← loop start
            0x18,              # $07
            0x65, 0x40,        # $08
            0x85, 0x40,        # $0A
            0xCA,              # $0C
            0xD0, 0xF7,        # $0D  BNE -9 → $06
            0xA5, 0x40,        # $0F
            0x00,              # $11
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 15  # 1+2+3+4+5

    def test_sum_1_to_10(self) -> None:
        # Sum 1..10 = 55 = 0x37
        prog = bytes([
            0xA9, 0x00,
            0x85, 0x40,
            0xA2, 0x0A,        # LDX #10
            0x8A,
            0x18,
            0x65, 0x40,
            0x85, 0x40,
            0xCA,
            0xD0, 0xF7,
            0xA5, 0x40,
            0x00,
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 55


class TestMultiply:
    """Multiply two numbers via repeated addition."""

    def test_multiply_3_by_4(self) -> None:
        # 3 × 4 = 12: add 3 four times.  ZP $40 = running product.
        #
        # $00: A9 00    LDA #0
        # $02: 85 40    STA $40      ; product = 0
        # $04: A2 04    LDX #4       ; outer counter = 4
        # loop ($06):
        # $06: 18       CLC
        # $07: A9 03    LDA #3       ; addend = 3
        # $09: 65 40    ADC $40      ; A = 3 + product
        # $0B: 85 40    STA $40      ; product = A
        # $0D: CA       DEX
        # $0E: D0 F6    BNE -10 → $06  (after $10: $10 + (-10) = $06)
        # $10: A5 40    LDA $40
        # $12: 00       BRK
        prog = bytes([
            0xA9, 0x00,
            0x85, 0x40,
            0xA2, 0x04,
            0x18,              # loop
            0xA9, 0x03,
            0x65, 0x40,
            0x85, 0x40,
            0xCA,
            0xD0, 0xF6,        # BNE -10
            0xA5, 0x40,
            0x00,
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 12

    def test_multiply_7_by_8(self) -> None:
        # 7 × 8 = 56 = 0x38
        prog = bytes([
            0xA9, 0x00,
            0x85, 0x40,
            0xA2, 0x08,        # counter = 8
            0x18,
            0xA9, 0x07,        # add 7
            0x65, 0x40,
            0x85, 0x40,
            0xCA,
            0xD0, 0xF6,
            0xA5, 0x40,
            0x00,
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 56


class TestFibonacci:
    """Compute Fibonacci numbers stored in zero page / memory."""

    def test_fib_7th_term(self) -> None:
        # Fibonacci sequence: 0,1,1,2,3,5,8,13 ...
        # Compute fib(7) = 13 = 0x0D by iterating 6 times.
        #
        # ZP layout: $40 = prev, $41 = curr, $42 = temp (all above code end ~$1E)
        #
        # $00-$03: A9 00 85 40    prev = 0
        # $04-$07: A9 01 85 41    curr = 1
        # $08-$09: A2 06          X = 6 (iterations)
        # loop ($0A):
        # $0A-$0B: A5 41          LDA curr
        # $0C:     18             CLC
        # $0D-$0E: 65 40          ADC prev   → new value
        # $0F-$10: 85 42          STA temp
        # $11-$12: A5 41          LDA curr
        # $13-$14: 85 40          prev = old curr
        # $15-$16: A5 42          LDA temp
        # $17-$18: 85 41          curr = new
        # $19:     CA             DEX
        # $1A-$1B: D0 EE          BNE -18 → $0A  (after $1C: $1C-18=10=$0A ✓)
        # $1C-$1D: A5 41          LDA $41 = fib result
        # $1E:     00             BRK
        prog = bytes([
            0xA9, 0x00, 0x85, 0x40,    # prev = 0
            0xA9, 0x01, 0x85, 0x41,    # curr = 1
            0xA2, 0x06,                # X = 6
            # loop at $0A:
            0xA5, 0x41,                # LDA curr
            0x18,                      # CLC
            0x65, 0x40,                # ADC prev
            0x85, 0x42,                # STA temp
            0xA5, 0x41,                # LDA curr
            0x85, 0x40,                # prev = old curr
            0xA5, 0x42,                # LDA temp (new)
            0x85, 0x41,                # curr = new
            0xCA,                      # DEX
            0xD0, 0xEE,                # BNE -18 → $0A
            0xA5, 0x41,                # LDA $41 = result
            0x00,                      # BRK
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 13

    def test_fib_first_few_in_memory(self) -> None:
        # Store the Fibonacci sequence 0,1,1,2,3,5 at absolute addresses $60..$65.
        #
        # ZP layout: $40=prev  $41=curr  $42=temp  (well above code end at ~$2C)
        # Output at $0060..$0065 via STA $0060,Y  (absolute indexed Y).
        # Y starts at 0 after reset; we use it as the output index.
        #
        # Phase 1: store fib[0]=0 and fib[1]=1 explicitly.
        # Phase 2: loop 4 times to produce fib[2..5], appending with STA abs,Y.
        #
        # $00: A9 00 85 40        prev = 0
        # $04: A9 01 85 41        curr = 1
        # $08: A5 40 99 60 00 C8  store prev → $0060[Y], Y++  (fib[0])
        # $0E: A5 41 99 60 00 C8  store curr → $0060[Y], Y++  (fib[1])
        # $14: A2 04              X = 4 (4 more terms)
        # loop ($16):
        # $16: A5 41 18 65 40 85 42    new = curr + prev → temp
        # $1D: A5 41 85 40             prev = old curr
        # $21: A5 42 85 41             curr = new
        # $25: 99 60 00               STA $0060,Y  → store new value
        # $28: C8                     INY
        # $29: CA                     DEX
        # $2A: D0 EA                  BNE -22 → $16  (after $2C: $2C-22=44-22=22=$16 ✓)
        # $2C: 00                     BRK
        prog = bytes([
            0xA9, 0x00, 0x85, 0x40,        # $00: prev = 0
            0xA9, 0x01, 0x85, 0x41,        # $04: curr = 1
            0xA5, 0x40, 0x99, 0x60, 0x00,  # $08: LDA prev; STA $0060,Y
            0xC8,                          # $0D: INY
            0xA5, 0x41, 0x99, 0x60, 0x00,  # $0E: LDA curr; STA $0060,Y
            0xC8,                          # $13: INY
            0xA2, 0x04,                    # $14: LDX #4
            # loop at $16:
            0xA5, 0x41,                    # $16: LDA curr
            0x18,                          # $18: CLC
            0x65, 0x40,                    # $19: ADC prev
            0x85, 0x42,                    # $1B: STA temp
            0xA5, 0x41,                    # $1D: LDA curr
            0x85, 0x40,                    # $1F: prev = old curr
            0xA5, 0x42,                    # $21: LDA temp (new)
            0x85, 0x41,                    # $23: curr = new
            0x99, 0x60, 0x00,              # $25: STA $0060,Y
            0xC8,                          # $28: INY
            0xCA,                          # $29: DEX
            0xD0, 0xEA,                    # $2A: BNE -22 → $16
            0x00,                          # $2C: BRK
        ])
        result = MOS6502Simulator().execute(prog)
        mem = result.final_state.memory
        assert mem[0x60] == 0   # fib(0)
        assert mem[0x61] == 1   # fib(1)
        assert mem[0x62] == 1   # fib(2)
        assert mem[0x63] == 2   # fib(3)
        assert mem[0x64] == 3   # fib(4)
        assert mem[0x65] == 5   # fib(5)


class TestSubroutineCall:
    """Test JSR/RTS with a meaningful subroutine."""

    def test_double_via_subroutine(self) -> None:
        # Main calls 'double' subroutine: A = A * 2 using ASL
        # $0000: A9 07     LDA #7
        # $0002: 20 07 00  JSR $0007    (call double)
        # $0005: 00        BRK
        # $0006: 00        padding
        # $0007: 0A        ASL A        (double subroutine: A <<= 1)
        # $0008: 60        RTS
        prog = bytes([
            0xA9, 0x07,        # $0000: LDA #7
            0x20, 0x07, 0x00,  # $0002: JSR $0007
            0x00,              # $0005: BRK
            0x00,              # $0006: padding
            0x0A,              # $0007: ASL A
            0x60,              # $0008: RTS
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 14

    def test_nested_calls(self) -> None:
        # A = ((3 * 2) + 1) via two subroutines
        # double: ASL A; RTS
        # inc_a: CLC; ADC #1; RTS
        # $0000: A9 03     LDA #3
        # $0002: 20 0C 00  JSR double ($000C)
        # $0005: 20 0F 00  JSR inc_a ($000F)
        # $0008: 00        BRK
        # $0009-$000B: padding
        # $000C: 0A        ASL A  (double)
        # $000D: 60        RTS
        # $000E: padding
        # $000F: 18        CLC    (inc_a)
        # $0010: 69 01     ADC #1
        # $0012: 60        RTS
        prog = bytes([
            0xA9, 0x03,        # $0000
            0x20, 0x0C, 0x00,  # $0002: JSR double
            0x20, 0x0F, 0x00,  # $0005: JSR inc_a
            0x00,              # $0008: BRK
            0x00, 0x00, 0x00,  # $0009-$000B
            0x0A,              # $000C: ASL A
            0x60,              # $000D: RTS
            0x00,              # $000E: padding
            0x18,              # $000F: CLC
            0x69, 0x01,        # $0010: ADC #1
            0x60,              # $0012: RTS
        ])
        result = MOS6502Simulator().execute(prog)
        assert result.final_state.a == 7  # (3*2)+1 = 7
