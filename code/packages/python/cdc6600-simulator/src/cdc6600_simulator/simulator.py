"""
CDC 6600 (1964) Behavioral Simulator
======================================

The CDC 6600 was the world's first supercomputer — designed by Seymour Cray at
Control Data Corporation.  This module implements a behavioral simulation of its
Central Processor (CP) instruction set.

Architecture at a glance
------------------------

  Word width:   60 bits (unusual; Cray picked 60 to maximise FP precision in
                discrete logic without needing a full 64-bit datapath in 1964)

  Registers:
    X0–X7   60-bit "operand" registers  (integer and float data)
    A0–A7   18-bit "address" registers  (memory addresses, up to 262,143 words)
    B0–B7   18-bit "index" registers    (loop counters; B0 hardwired to 0)

  Program counter:
    P       "parcel pointer" — word_index × 4 + parcel_index (0–3).
            Instructions are packed four 15-bit parcels per 60-bit word.

  Instruction sizes:
    Short   15 bits  (one parcel)  — register-to-register ops
    Long    30 bits  (two parcels) — load-immediate, memory, branches

  Memory:   4 096 sixty-bit words (behavioural subset of 131 072)

Instruction encoding
---------------------

Short (15-bit):
  [14:9]  f   opcode (6 bits)
  [ 8:6]  i   destination register index (3 bits)
  [ 5:3]  j   left-source register index (3 bits)
  [ 2:0]  k   right-source register index (3 bits)

Long (30-bit) — occupies two consecutive parcels:
  [29:24]  f   opcode (6 bits)
  [23:21]  i   destination register index (3 bits)
  [20:18]  j   source register index / condition register (3 bits)
  [17: 0]  K   18-bit constant (address, immediate, or branch target)

HALT: a 15-bit all-zeros parcel (0x0000) halts the simulator.

Signed arithmetic
-----------------
The real CDC 6600 used one's-complement.  This simulator uses Python ints
(two's-complement) masked to 60 bits, interpreting bit 59 as the sign bit for
comparison instructions.  Programs that avoid the ±0 corner case behave
identically to one's-complement hardware.
"""

from __future__ import annotations

from simulator_protocol import (
    ExecutionResult,
    Simulator,
    StepTrace,
)

from .state import (
    MASK18,
    MASK60,
    MEMORY_WORDS,
    CDC6600State,
    make_initial_state,
    sext60,
)

# ── Opcode constants ────────────────────────────────────────────────────────────
#
# Short (15-bit) opcodes — f field values for Format 1 instructions.

F_TXB   = 1    # Xi = zero_extend60(Bj)
F_TBX   = 2    # Bi = Xj[17:0]
F_TAX   = 3    # Xi = zero_extend60(Aj)
F_TXA   = 4    # Ai = Xj[17:0]
F_IXPB  = 5    # Xi = Xj + Bk  (integer add X+B)
F_IXMB  = 6    # Xi = Xj - Bk  (integer subtract X-B)
F_IXXP  = 7    # Xi = Xj + Xk  (integer add X+X)
F_IXXM  = 8    # Xi = Xj - Xk  (integer subtract X-X)
F_BXND  = 9    # Xi = Xj & Xk  (boolean AND)
F_BXOR  = 10   # Xi = Xj | Xk  (boolean OR)
F_BXXR  = 11   # Xi = Xj ^ Xk  (boolean XOR)
F_BXMR  = 12   # Xi = ~Xj      (boolean complement; k ignored)
F_LSHL  = 13   # Xi = Xj << (Bk & 63)
F_LSHR  = 14   # Xi = Xj >> (Bk & 63)
F_IBBP  = 15   # Bi = Bj + Bk  (B-register add)
F_IBBM  = 16   # Bi = Bj - Bk  (B-register subtract)
F_IAAP  = 17   # Ai = Aj + Bk  (A-register add)
F_IAAM  = 18   # Ai = Aj - Bk  (A-register subtract)
F_CMPEQ = 19   # Bi = 1 if Xj == Xk else 0
F_CMPLT = 20   # Bi = 1 if signed(Xj) < signed(Xk) else 0
F_CMPGT = 21   # Bi = 1 if signed(Xj) > signed(Xk) else 0
F_IXMUL = 22   # Xi = (Xj * Xk)[59:0]

# Long (30-bit) opcodes — f field values for Format 2 instructions.

F_LDXI  = 32   # Xi = K  (load 18-bit zero-extended constant)
F_LDBI  = 33   # Bi = K
F_LDAI  = 34   # Ai = K
F_LDX   = 35   # Xi = mem[Aj + K]
F_STX   = 36   # mem[Ai + K] = Xj
F_LDB   = 37   # Bi = mem[Aj + K][17:0]
F_STB   = 38   # mem[Ai + K][17:0] = Bj  (zero rest of word)
F_JEQ   = 40   # if Bj == 0: P = K
F_JNE   = 41   # if Bj != 0: P = K
F_JXZ   = 42   # if Xj == 0: P = K
F_JXN   = 43   # if Xj != 0: P = K
F_JMP   = 44   # P = K  (unconditional branch)
F_JSR   = 45   # B7 = P+2; P = K
F_RET   = 46   # P = Bj


class CDC6600Simulator(Simulator[CDC6600State]):
    """
    Behavioral simulator for the CDC 6600 Central Processor (1964).

    Usage
    -----
    >>> sim = CDC6600Simulator()
    >>> # Encode: LDXI X1, 42  then HALT
    >>> prog = long_instr(F_LDXI, 1, 0, 42) + HALT
    >>> result = sim.execute(prog)
    >>> result.final_state.x1
    42
    """

    def __init__(self) -> None:
        self._state: CDC6600State = make_initial_state()

    # ── SIM00 Protocol ─────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers and memory; set P=0; clear halted flag."""
        self._state = make_initial_state()

    def load(self, program: bytes) -> None:
        """
        Reset, then pack *program* bytes into 60-bit memory words.

        Each pair of bytes encodes one 15-bit parcel (big-endian, high-nibble
        used, low bit of second byte always 0):

            parcel_value = int.from_bytes(two_bytes, "big") >> 1

        Wait — actually parcels ARE 15-bit but we store them packed into 60-bit
        words with no padding bits.  The encoding is:

            bytes[0:2]  → parcel 0 of word 0 (bits [59:45])
            bytes[2:4]  → parcel 1 of word 0 (bits [44:30])
            bytes[4:6]  → parcel 2 of word 0 (bits [29:15])
            bytes[6:8]  → parcel 3 of word 0 (bits [14: 0])
            bytes[8:10] → parcel 0 of word 1  …

        Each 2-byte chunk encodes one 15-bit parcel as a big-endian unsigned
        integer (the low bit of each 2-byte chunk is unused / zero).  We mask
        to 15 bits and shift into position.

        For 30-bit long instructions the caller emits 4 bytes (two parcels of
        15 bits each) by calling ``long_instr()``.

        The program is padded with zero bytes to fill the last word.
        """
        self.reset()

        # Pad to a multiple of 8 bytes (4 parcels × 2 bytes = one 60-bit word)
        if len(program) % 8:
            program = program + b"\x00" * (8 - len(program) % 8)

        mem = list(self._state.memory)
        word_idx = 0
        for offset in range(0, len(program), 8):
            if word_idx >= MEMORY_WORDS:
                break
            word = 0
            for parcel in range(4):
                chunk = program[offset + parcel * 2 : offset + parcel * 2 + 2]
                p_val = int.from_bytes(chunk, "big") & 0x7FFF   # 15 bits
                word = (word << 15) | p_val
            mem[word_idx] = word
            word_idx += 1

        self._state = CDC6600State(
            p=0,
            x=self._state.x,
            a=self._state.a,
            b=self._state.b,
            memory=tuple(mem),
            halted=False,
        )

    def step(self) -> StepTrace:
        """
        Fetch and execute one instruction (15- or 30-bit parcel), advance P.

        Returns a StepTrace describing what happened.
        """
        state = self._state
        p_before = state.p

        if state.halted:
            self._state = CDC6600State(
                p=state.p, x=state.x, a=state.a, b=state.b,
                memory=state.memory, halted=True,
            )
            return StepTrace(
                pc_before=p_before,
                pc_after=p_before,
                mnemonic="HALT",
                description="HALT (already halted)",
            )

        try:
            mnemonic, p_after = self._fetch_and_execute()
            description = f"{mnemonic} @ parcel 0x{p_before:04X}"
            return StepTrace(
                pc_before=p_before,
                pc_after=p_after,
                mnemonic=mnemonic,
                description=description,
            )
        except Exception as exc:
            err_msg = f"ERROR: {exc}"
            return StepTrace(
                pc_before=p_before,
                pc_after=p_before,
                mnemonic=err_msg,
                description=err_msg,
            )

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult:
        """
        Load *program*, run until halted or *max_steps* reached.

        Returns an :class:`ExecutionResult` with the final state and all
        :class:`StepTrace` records.
        """
        self.load(program)
        traces: list[StepTrace] = []
        error: str | None = None

        for _ in range(max_steps):
            if self._state.halted:
                break
            trace = self.step()
            traces.append(trace)
            if self._state.halted:
                break
            if trace.mnemonic.startswith("ERROR:"):
                error = trace.mnemonic
                break
        else:
            error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self._state.halted,
            steps=len(traces),
            final_state=self._state,
            error=error,
            traces=traces,
        )

    def get_state(self) -> CDC6600State:
        """Return a frozen snapshot of the current simulator state."""
        return self._state

    # ── Internal helpers ───────────────────────────────────────────────────────

    def _read_parcel(self, p: int) -> int:
        """
        Read one 15-bit parcel from parcel address *p*.

        Parcel addresses:  word = p // 4,  parcel_in_word = p % 4.
        The four parcels in a 60-bit word are stored most-significant first:

            parcel 0 → bits [59:45] (shift right by 45)
            parcel 1 → bits [44:30] (shift right by 30)
            parcel 2 → bits [29:15] (shift right by 15)
            parcel 3 → bits [14: 0] (shift right by  0)
        """
        word_idx = p // 4
        parcel_in_word = p % 4
        if word_idx >= MEMORY_WORDS:
            return 0
        word = self._state.memory[word_idx]
        shift = (3 - parcel_in_word) * 15
        return (word >> shift) & 0x7FFF

    def _write_parcel(self, p: int, value: int) -> None:
        """Write one 15-bit parcel to parcel address *p* (used during load)."""
        word_idx = p // 4
        parcel_in_word = p % 4
        if word_idx >= MEMORY_WORDS:
            return
        mem = list(self._state.memory)
        shift = (3 - parcel_in_word) * 15
        mask = 0x7FFF << shift
        mem[word_idx] = (mem[word_idx] & ~mask) | ((value & 0x7FFF) << shift)
        self._state = CDC6600State(
            p=self._state.p,
            x=self._state.x,
            a=self._state.a,
            b=self._state.b,
            memory=tuple(mem),
            halted=self._state.halted,
        )

    def _fetch_and_execute(self) -> tuple[str, int]:
        """
        Fetch one instruction starting at the current parcel address P,
        execute it, update the state, and return (mnemonic, new_P).

        All register and memory updates are applied atomically by building
        new tuples and replacing ``self._state``.
        """
        state = self._state
        p = state.p

        # ── Fetch first parcel ───────────────────────────────────────────────
        p0 = self._read_parcel(p)

        # HALT: all-zeros parcel
        if p0 == 0:
            self._state = CDC6600State(
                p=p, x=state.x, a=state.a, b=state.b,
                memory=state.memory, halted=True,
            )
            return "HALT", p

        f = (p0 >> 9) & 0x3F    # opcode (bits [14:9])
        i = (p0 >> 6) & 0x07    # destination register (bits [8:6])
        j = (p0 >> 3) & 0x07    # left source register  (bits [5:3])
        k = (p0 >> 0) & 0x07    # right source register (bits [2:0])

        # ── Determine instruction length ─────────────────────────────────────
        # Long instructions (Format 2) use opcodes 32–63 (≥ 32 = bit 5 set).
        if f >= 32:
            # Fetch second parcel for the 18-bit K field
            p1 = self._read_parcel(p + 1)
            # K is built from the low 3 bits of j-field in p0 and all of p1:
            # Actually: the long format packs the full K across both parcels.
            # p0 already contains f(6), i(3), j(3), and the upper 3 bits of K.
            # Let's re-decode properly:
            #   p0[14:9] = f (6 bits)
            #   p0[8:6]  = i (3 bits)
            #   p0[5:3]  = j (3 bits)  — repurposed as source/condition reg
            #   p0[2:0]  = K[17:15]    (upper 3 bits of K)
            # p1[14:0]   = K[14:0]     (lower 15 bits of K)
            K_high = k & 0x7               # 3 bits from low of first parcel
            K_low  = p1 & 0x7FFF           # 15 bits from second parcel
            K      = (K_high << 15) | K_low
            new_p  = p + 2
            return self._exec_long(f, i, j, K, new_p)
        else:
            new_p = p + 1
            return self._exec_short(f, i, j, k, new_p)

    def _exec_short(
        self, f: int, i: int, j: int, k: int, new_p: int
    ) -> tuple[str, int]:
        """Execute a 15-bit (Format 1) instruction."""
        state = self._state

        # Working copies of register banks as lists for mutation
        x = list(state.x)
        a = list(state.a)
        b = list(state.b)

        Xj = state.x[j]
        Xk = state.x[k]
        Aj = state.a[j]
        Bj = state.b[j]
        Bk = state.b[k]

        mnemonic = "?"

        if f == F_TXB:
            # Xi = zero_extend60(Bj) — transmit B register to X register
            x[i] = Bj & MASK60
            mnemonic = f"TXB X{i},B{j}"

        elif f == F_TBX:
            # Bi = Xj[17:0] — transmit lower 18 bits of X to B register
            if i != 0:
                b[i] = Xj & MASK18
            mnemonic = f"TBX B{i},X{j}"

        elif f == F_TAX:
            # Xi = zero_extend60(Aj) — transmit A register to X register
            x[i] = Aj & MASK60
            mnemonic = f"TAX X{i},A{j}"

        elif f == F_TXA:
            # Ai = Xj[17:0] — transmit lower 18 bits of X to A register
            a[i] = Xj & MASK18
            mnemonic = f"TXA A{i},X{j}"

        elif f == F_IXPB:
            # Xi = Xj + Bk (integer; Bk zero-extended to 60 bits)
            x[i] = (Xj + (Bk & MASK18)) & MASK60
            mnemonic = f"IXPB X{i},X{j},B{k}"

        elif f == F_IXMB:
            # Xi = Xj - Bk
            x[i] = (Xj - (Bk & MASK18)) & MASK60
            mnemonic = f"IXMB X{i},X{j},B{k}"

        elif f == F_IXXP:
            # Xi = Xj + Xk (60-bit integer add)
            x[i] = (Xj + Xk) & MASK60
            mnemonic = f"IXXP X{i},X{j},X{k}"

        elif f == F_IXXM:
            # Xi = Xj - Xk (60-bit integer subtract)
            x[i] = (Xj - Xk) & MASK60
            mnemonic = f"IXXM X{i},X{j},X{k}"

        elif f == F_BXND:
            # Xi = Xj & Xk (boolean AND)
            x[i] = (Xj & Xk) & MASK60
            mnemonic = f"BXND X{i},X{j},X{k}"

        elif f == F_BXOR:
            # Xi = Xj | Xk (boolean OR)
            x[i] = (Xj | Xk) & MASK60
            mnemonic = f"BXOR X{i},X{j},X{k}"

        elif f == F_BXXR:
            # Xi = Xj ^ Xk (boolean XOR, "exclusive or")
            x[i] = (Xj ^ Xk) & MASK60
            mnemonic = f"BXXR X{i},X{j},X{k}"

        elif f == F_BXMR:
            # Xi = ~Xj (boolean complement; k ignored)
            x[i] = (~Xj) & MASK60
            mnemonic = f"BXMR X{i},X{j}"

        elif f == F_LSHL:
            # Xi = Xj << (Bk & 63) logical left shift
            shift = Bk & 63
            x[i] = (Xj << shift) & MASK60
            mnemonic = f"LSHL X{i},X{j},B{k}"

        elif f == F_LSHR:
            # Xi = Xj >> (Bk & 63) logical right shift (zero-fill)
            shift = Bk & 63
            x[i] = (Xj & MASK60) >> shift   # guaranteed non-negative (& MASK60)
            mnemonic = f"LSHR X{i},X{j},B{k}"

        elif f == F_IBBP:
            # Bi = Bj + Bk (18-bit integer add; B0 protected)
            if i != 0:
                b[i] = (Bj + Bk) & MASK18
            mnemonic = f"IBBP B{i},B{j},B{k}"

        elif f == F_IBBM:
            # Bi = Bj - Bk (18-bit integer subtract; B0 protected)
            if i != 0:
                b[i] = (Bj - Bk) & MASK18
            mnemonic = f"IBBM B{i},B{j},B{k}"

        elif f == F_IAAP:
            # Ai = Aj + Bk (18-bit address add)
            a[i] = (Aj + Bk) & MASK18
            mnemonic = f"IAAP A{i},A{j},B{k}"

        elif f == F_IAAM:
            # Ai = Aj - Bk (18-bit address subtract)
            a[i] = (Aj - Bk) & MASK18
            mnemonic = f"IAAM A{i},A{j},B{k}"

        elif f == F_CMPEQ:
            # Bi = 1 if Xj == Xk else 0 (compare into B register)
            if i != 0:
                b[i] = 1 if Xj == Xk else 0
            mnemonic = f"CMPEQ B{i},X{j},X{k}"

        elif f == F_CMPLT:
            # Bi = 1 if signed(Xj) < signed(Xk) else 0
            sj = sext60(Xj)
            sk = sext60(Xk)
            if i != 0:
                b[i] = 1 if sj < sk else 0
            mnemonic = f"CMPLT B{i},X{j},X{k}"

        elif f == F_CMPGT:
            # Bi = 1 if signed(Xj) > signed(Xk) else 0
            sj = sext60(Xj)
            sk = sext60(Xk)
            if i != 0:
                b[i] = 1 if sj > sk else 0
            mnemonic = f"CMPGT B{i},X{j},X{k}"

        elif f == F_IXMUL:
            # Xi = (Xj * Xk)[59:0] — lower 60 bits of 60-bit integer multiply
            x[i] = (Xj * Xk) & MASK60
            mnemonic = f"IXMUL X{i},X{j},X{k}"

        else:
            raise ValueError(
                f"Unknown short opcode f={f} (octal {f:o}) at parcel 0x{state.p:04X}"
            )

        # B0 must always read as 0 (enforce invariant)
        b[0] = 0

        self._state = CDC6600State(
            p=new_p,
            x=tuple(x),
            a=tuple(a),
            b=tuple(b),
            memory=state.memory,
            halted=False,
        )
        return mnemonic, new_p

    def _exec_long(
        self, f: int, i: int, j: int, K: int, new_p: int
    ) -> tuple[str, int]:
        """Execute a 30-bit (Format 2) instruction."""
        state = self._state
        x = list(state.x)
        a = list(state.a)
        b = list(state.b)
        mem = list(state.memory)

        halted = False
        branched = False
        branch_target = 0
        mnemonic = "?"

        if f == F_LDXI:
            # Xi = K (18-bit zero-extended constant into 60-bit X register)
            x[i] = K & MASK18
            mnemonic = f"LDXI X{i},{K}"

        elif f == F_LDBI:
            # Bi = K (18-bit constant into B register; B0 protected)
            if i != 0:
                b[i] = K & MASK18
            mnemonic = f"LDBI B{i},{K}"

        elif f == F_LDAI:
            # Ai = K (18-bit constant into A register)
            a[i] = K & MASK18
            mnemonic = f"LDAI A{i},{K}"

        elif f == F_LDX:
            # Xi = mem[Aj + K] — load 60-bit word from memory into Xi
            addr = (state.a[j] + K) & MASK18
            if addr >= MEMORY_WORDS:
                raise ValueError(
                    f"LDX: memory address {addr} out of bounds (max {MEMORY_WORDS - 1})"
                )
            x[i] = state.memory[addr] & MASK60
            mnemonic = f"LDX X{i},A{j}+{K}"

        elif f == F_STX:
            # mem[Ai + K] = Xj — store Xj to memory
            addr = (state.a[i] + K) & MASK18
            if addr >= MEMORY_WORDS:
                raise ValueError(
                    f"STX: memory address {addr} out of bounds (max {MEMORY_WORDS - 1})"
                )
            mem[addr] = state.x[j] & MASK60
            mnemonic = f"STX A{i}+{K},X{j}"

        elif f == F_LDB:
            # Bi = mem[Aj + K][17:0] — load lower 18 bits of a word into Bi
            addr = (state.a[j] + K) & MASK18
            if addr >= MEMORY_WORDS:
                raise ValueError(
                    f"LDB: memory address {addr} out of bounds (max {MEMORY_WORDS - 1})"
                )
            if i != 0:
                b[i] = state.memory[addr] & MASK18
            mnemonic = f"LDB B{i},A{j}+{K}"

        elif f == F_STB:
            # mem[Ai + K][17:0] = Bj — store Bj into lower 18 bits of word
            addr = (state.a[i] + K) & MASK18
            if addr >= MEMORY_WORDS:
                raise ValueError(
                    f"STB: memory address {addr} out of bounds (max {MEMORY_WORDS - 1})"
                )
            mem[addr] = state.b[j] & MASK18
            mnemonic = f"STB A{i}+{K},B{j}"

        elif f == F_JEQ:
            # if Bj == 0: P = K  (branch if B register equals zero)
            mnemonic = f"JEQ B{j}==0,{K}"
            if state.b[j] == 0:
                branched = True
                branch_target = K

        elif f == F_JNE:
            # if Bj != 0: P = K  (branch if B register non-zero)
            mnemonic = f"JNE B{j}!=0,{K}"
            if state.b[j] != 0:
                branched = True
                branch_target = K

        elif f == F_JXZ:
            # if Xj == 0: P = K  (branch if X register equals zero)
            mnemonic = f"JXZ X{j}==0,{K}"
            if state.x[j] == 0:
                branched = True
                branch_target = K

        elif f == F_JXN:
            # if Xj != 0: P = K  (branch if X register non-zero)
            mnemonic = f"JXN X{j}!=0,{K}"
            if state.x[j] != 0:
                branched = True
                branch_target = K

        elif f == F_JMP:
            # P = K  (unconditional branch; i and j fields ignored)
            mnemonic = f"JMP {K}"
            branched = True
            branch_target = K

        elif f == F_JSR:
            # B7 = P+2 (return address); P = K  (call subroutine)
            b[7] = new_p & MASK18   # new_p is already P+2 from fetch
            mnemonic = f"JSR B7={new_p},P={K}"
            branched = True
            branch_target = K

        elif f == F_RET:
            # P = Bj  (return: jump to parcel address stored in Bj)
            mnemonic = f"RET P=B{j}"
            branched = True
            branch_target = state.b[j] & MASK18

        else:
            raise ValueError(
                f"Unknown long opcode f={f} (octal {f:o}) at parcel 0x{state.p:04X}"
            )

        # B0 must always stay zero
        b[0] = 0

        final_p = branch_target if branched else new_p

        self._state = CDC6600State(
            p=final_p,
            x=tuple(x),
            a=tuple(a),
            b=tuple(b),
            memory=tuple(mem),
            halted=halted,
        )
        return mnemonic, final_p


# ── Encoding helpers (exported for use in tests and programs) ──────────────────


def short_instr(f: int, i: int, j: int, k: int) -> bytes:
    """
    Encode a 15-bit CDC 6600 short instruction into 2 bytes (big-endian).

    The 15-bit value is stored right-aligned in the 16-bit big-endian integer,
    i.e. the high bit of the first byte is always 0.

        Bit layout in two bytes:
          byte 0: [0][f5][f4][f3][f2][f1][f0][i2]
          byte 1: [i1][i0][j2][j1][j0][k2][k1][k0]

    Example:
        short_instr(F_IXXP, 1, 2, 3)  →  b'\\x0e\\x13'
        f=7, i=1, j=2, k=3
        = 0b000_0111_001_010_011 = 0x0393 … wait that's 15 bits:
        bits: 000111 001 010 011 → 0x1CB = 459
    """
    v = ((f & 0x3F) << 9) | ((i & 0x7) << 6) | ((j & 0x7) << 3) | (k & 0x7)
    return v.to_bytes(2, "big")


def long_instr(f: int, i: int, j: int, K: int) -> bytes:
    """
    Encode a 30-bit CDC 6600 long instruction into 4 bytes (big-endian).

    The 30-bit value is split across two consecutive 15-bit parcels:
      first parcel  (bits [29:15]): f(6), i(3), j(3), K[17:15](3)
      second parcel (bits [14: 0]): K[14:0](15)

    Each parcel is stored right-aligned in a 16-bit big-endian value.
    """
    K = K & 0x3FFFF   # clamp to 18 bits
    K_high = (K >> 15) & 0x7     # top 3 bits of K
    K_low  = K & 0x7FFF          # bottom 15 bits of K
    first  = ((f & 0x3F) << 9) | ((i & 0x7) << 6) | ((j & 0x7) << 3) | K_high
    second = K_low
    return first.to_bytes(2, "big") + second.to_bytes(2, "big")


HALT: bytes = b"\x00\x00"   # 15-bit all-zeros parcel — stops the simulator
