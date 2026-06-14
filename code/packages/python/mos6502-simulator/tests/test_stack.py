"""Tests for PHA, PLA, PHP, PLP and stack register interactions."""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


def run(prog: list[int]):
    return MOS6502Simulator().execute(bytes(prog + [0x00])).final_state


class TestPHA:
    def test_push_and_pull(self) -> None:
        # LDA #$42; PHA; LDA #$00; PLA → A=$42
        s = run([0xA9, 0x42, 0x48, 0xA9, 0x00, 0x68])
        assert s.a == 0x42

    def test_pla_sets_nz(self) -> None:
        # Push 0 then pull — should set Z
        s = run([0xA9, 0x00, 0x48, 0xA9, 0x01, 0x68])
        assert s.flag_z is True
        assert s.a == 0x00

    def test_pla_sets_n(self) -> None:
        # Push 0x80 (negative); pull it
        s = run([0xA9, 0x80, 0x48, 0xA9, 0x00, 0x68])
        assert s.flag_n is True
        assert s.a == 0x80

    def test_multiple_pushes(self) -> None:
        # LDA #1; PHA; LDA #2; PHA; PLA; PLA → A=1 (LIFO)
        s = run([0xA9, 0x01, 0x48, 0xA9, 0x02, 0x48, 0x68, 0x68])
        assert s.a == 0x01

    def test_stack_pointer_decrements(self) -> None:
        # After reset S=0xFD; PHA decrements to 0xFC.
        # We check S right after PHA (before BRK) because BRK itself pushes
        # PC and P to the stack, which would decrement S three more times.
        sim = MOS6502Simulator()
        sim.reset()
        initial_s = sim._s  # 0xFD
        sim.load(bytes([0xA9, 0x42, 0x48, 0x00]))  # LDA #$42; PHA; BRK
        sim.step()  # LDA #$42
        sim.step()  # PHA  ← S decrements here
        assert sim.get_state().s == initial_s - 1

    def test_stack_pointer_increments_on_pull(self) -> None:
        # Push one byte then pull it — S returns to its initial value.
        # We stop right after PLA to avoid BRK's own stack modifications.
        sim = MOS6502Simulator()
        sim.reset()
        initial_s = sim._s
        sim.load(bytes([0xA9, 0x55, 0x48, 0x68, 0x00]))
        sim.step()  # LDA #$55
        sim.step()  # PHA  → S = initial_s - 1
        sim.step()  # PLA  → S = initial_s
        assert sim.get_state().s == initial_s


class TestPHP:
    def test_php_plp_round_trip(self) -> None:
        # SEC; PHP; CLC; PLP → C=1 (restored)
        s = run([0x38, 0x08, 0x18, 0x28])
        assert s.flag_c is True

    def test_php_plp_restores_all_flags(self) -> None:
        # Set Z,N,C then PHP; clear all; PLP → flags restored
        # LDA #0 (Z=1,N=0); SEC (C=1); PHP; CLC; PLP
        s = run([0xA9, 0x00, 0x38, 0x08, 0x18, 0xA9, 0x01, 0x28])
        assert s.flag_c is True
        assert s.flag_z is True

    def test_plp_sets_flags_from_stack(self) -> None:
        # Push a known P byte using PHA then do PLP
        # Craft P byte with C=1, Z=0, N=0, I=1 (bit5=1 always, bit4=B=0 in pulled)
        # P = 0b00100101 = 0x25 (N=0,V=0,bit5=1,B=0,D=0,I=1,Z=0,C=1)
        s = run([0xA9, 0x25, 0x48, 0x28])  # LDA #$25; PHA; PLP
        assert s.flag_c is True
        assert s.flag_i is True
        assert s.flag_z is False

    def test_php_b_flag_set(self) -> None:
        # PHP always pushes P with bit 4 (B) set = 1
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0x08, 0x00]))  # PHP; BRK
        while not sim._halted:
            sim.step()
        # Stack is at page 1; after reset S=0xFD; PHP decrements to 0xFC
        # Pushed value at memory[0x01FD]
        pushed = sim._memory[0x01FD]
        assert pushed & 0x10 != 0  # bit 4 (B) must be set

    def test_bit5_always_set(self) -> None:
        # PHP pushes P with bit 5 always set
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0x08, 0x00]))
        while not sim._halted:
            sim.step()
        pushed = sim._memory[0x01FD]
        assert pushed & 0x20 != 0  # bit 5 always 1
