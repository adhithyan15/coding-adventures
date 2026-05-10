"""Tests for Z80 interrupt and interrupt mode instructions.

Covers: DI, EI
IM 0, IM 1, IM 2
        interrupt() with IM 0/1/2
        nmi()
        RETI, RETN.
"""

from z80_simulator import Z80Simulator

# ── DI / EI ───────────────────────────────────────────────────────────────────

class TestDIEI:
    def test_di_clears_iff1_iff2(self):
        # EI; DI → both IFFs = False
        sim = Z80Simulator()
        r = sim.execute(bytes([0xFB, 0xF3, 0x76]))   # EI; DI; HALT
        assert r.final_state.iff1 is False
        assert r.final_state.iff2 is False

    def test_ei_sets_iff1_iff2(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0xFB, 0x76]))   # EI; HALT
        assert r.final_state.iff1 is True
        assert r.final_state.iff2 is True

    def test_initial_state_interrupts_disabled(self):
        sim = Z80Simulator()
        assert sim.get_state().iff1 is False
        assert sim.get_state().iff2 is False


# ── IM 0 / IM 1 / IM 2 ───────────────────────────────────────────────────────

class TestInterruptMode:
    def test_im_0(self):
        # ED 46: IM 0
        sim = Z80Simulator()
        # ED 56 = IM 1, then ED 46 = IM 0; final state should be IM 0
        r = sim.execute(bytes([0xED, 0x56, 0xED, 0x46, 0x76]))
        assert r.final_state.im == 0

    def test_im_1(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0xED, 0x56, 0x76]))   # IM 1; HALT
        assert r.final_state.im == 1

    def test_im_2(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0xED, 0x5E, 0x76]))   # IM 2; HALT
        assert r.final_state.im == 2


# ── interrupt() ───────────────────────────────────────────────────────────────

class TestMaskableInterrupt:
    def test_interrupt_ignored_when_iff1_false(self):
        # Interrupts disabled: interrupt() should have no effect
        sim = Z80Simulator()
        sim.load(bytes([0x76]))   # HALT
        # Don't execute — just fire interrupt without EI
        sim.interrupt(0xFF)
        # PC should not have changed (still halted at 0x0001 after HALT would run)
        # Actually we haven't stepped, so PC=0
        assert sim.get_state().pc == 0

    def test_interrupt_im1_jumps_to_0038(self):
        # EI; IM 1; execute long enough to ensure halt is hit then interrupt
        sim = Z80Simulator()
        # Program: EI; IM 1; NOP loop (will be interrupted)
        # Put a HALT at 0x0038
        prog = bytearray(0x40)
        prog[0] = 0xFB        # EI
        prog[1] = 0xED        # IM 1
        prog[2] = 0x56
        prog[3] = 0x76        # HALT (will halt, but interrupts re-enable after)
        prog[0x38] = 0x76     # HALT at interrupt handler

        sim.load(bytes(prog))
        # Step through EI and IM 1
        sim.step()   # EI
        sim.step()   # IM 1
        # Now fire interrupt before HALT (IFF1=True, IM=1)
        sim.interrupt()
        # PC should be 0x0038 now
        s = sim.get_state()
        assert s.pc == 0x0038

    def test_interrupt_im0_rst(self):
        # IM 0: interrupt(data) executes RST p where p = data & 0x38
        sim = Z80Simulator()
        prog = bytearray(0x40)
        prog[0] = 0xFB        # EI
        prog[1] = 0xED        # IM 0
        prog[2] = 0x46
        prog[3] = 0x31        # LD SP, 0x8000
        prog[4] = 0x00
        prog[5] = 0x80
        prog[0x08] = 0x76     # HALT at RST 0x08 handler

        sim.load(bytes(prog))
        sim.step()  # EI
        sim.step()  # IM 0
        sim.step()  # LD SP, 0x8000
        sim.interrupt(0xCF)   # RST 0x08 (0xCF & 0x38 = 0x08)
        s = sim.get_state()
        assert s.pc == 0x0008

    def test_interrupt_im2(self):
        # IM 2: vector at I*256 + data (even byte from bus)
        # Put 0x0080 at the vector table entry I=0x02, data=0x00 → addr=0x0200
        # Actual handler at 0x0080
        sim = Z80Simulator()
        prog = bytearray(0x300)
        prog[0] = 0x3E
        prog[1] = 0x02   # LD A, 2
        prog[2] = 0xED
        prog[3] = 0x47   # LD I, A  (I=2)
        prog[4] = 0xFB                    # EI
        prog[5] = 0xED
        prog[6] = 0x5E   # IM 2
        prog[7] = 0x31
        prog[8] = 0x00
        prog[9] = 0x80  # LD SP, 0x8000
        # Vector table at I*256 = 0x0200; data=0x00 → read from 0x0200/0x0201
        prog[0x0200] = 0x80  # lo of handler
        prog[0x0201] = 0x00  # hi of handler → handler at 0x0080
        prog[0x0080] = 0x76  # HALT at handler

        sim.load(bytes(prog))
        for _ in range(5):  # step through setup
            sim.step()
        sim.interrupt(0x00)  # data=0x00
        s = sim.get_state()
        assert s.pc == 0x0080


# ── nmi() ─────────────────────────────────────────────────────────────────────

class TestNMI:
    def test_nmi_jumps_to_0066(self):
        sim = Z80Simulator()
        prog = bytearray(0x70)
        prog[0] = 0x31
        prog[1] = 0x00
        prog[2] = 0x80  # LD SP, 0x8000
        prog[0x66] = 0x76   # HALT at NMI handler

        sim.load(bytes(prog))
        sim.step()   # LD SP, 0x8000
        sim.nmi()
        s = sim.get_state()
        assert s.pc == 0x0066

    def test_nmi_always_accepted(self):
        # NMI fires even when IFF1=False (interrupts disabled)
        sim = Z80Simulator()
        prog = bytearray(0x70)
        prog[0] = 0xF3        # DI (IFF1=False)
        prog[0x66] = 0x76
        sim.load(bytes(prog))
        sim.step()   # DI
        sim.nmi()
        assert sim.get_state().pc == 0x0066

    def test_nmi_saves_iff1_to_iff2(self):
        # After EI (IFF1=True), NMI should save IFF1 → IFF2 and clear IFF1
        sim = Z80Simulator()
        prog = bytearray(0x70)
        prog[0] = 0xFB        # EI
        prog[0x66] = 0x76
        sim.load(bytes(prog))
        sim.step()   # EI
        assert sim.get_state().iff1 is True
        sim.nmi()
        s = sim.get_state()
        assert s.iff1 is False
        assert s.iff2 is True   # old IFF1 saved here

    def test_nmi_pushes_pc_to_stack(self):
        sim = Z80Simulator()
        prog = bytearray(0x70)
        prog[0] = 0x31
        prog[1] = 0x00
        prog[2] = 0x80  # LD SP, 0x8000
        prog[3] = 0x00   # NOP at 0x0003 (this is where we fire NMI)
        prog[0x66] = 0x76
        sim.load(bytes(prog))
        sim.step()       # LD SP
        # PC is now 3; fire NMI
        sim.nmi()
        s = sim.get_state()
        # Return address 0x0003 on stack
        assert s.memory[0x7FFE] == 0x03
        assert s.memory[0x7FFF] == 0x00


# ── RETI / RETN ───────────────────────────────────────────────────────────────

class TestRETI_RETN:
    def test_retn_restores_iff1_from_iff2(self):
        # RETN: IFF1 ← IFF2; pop PC
        # Setup: EI → IFF1=IFF2=True; NMI → IFF1=False, IFF2=True
        # In handler: RETN → IFF1=IFF2=True (restored)
        sim = Z80Simulator()
        prog = bytearray(0x70)
        prog[0] = 0x31
        prog[1] = 0x00
        prog[2] = 0x80  # LD SP, 0x8000
        prog[3] = 0xFB    # EI
        prog[4] = 0x76    # HALT (return address after RETN)
        prog[0x66] = 0xED
        prog[0x67] = 0x45   # RETN at NMI handler
        sim.load(bytes(prog))
        sim.step()   # LD SP
        sim.step()   # EI → IFF1=IFF2=True
        sim.nmi()    # IFF2=True, IFF1=False; pushes return addr 0x0004
        sim.step()   # RETN → IFF1=IFF2=True; pops to 0x0004
        s = sim.get_state()
        assert s.iff1 is True
        assert s.pc == 0x0004
