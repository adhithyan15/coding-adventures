"""Intel 4004 code generator for the Tetrad JIT (TET05).

Translates optimized JIT IR (``list[IRInstr]``) to Intel 4004 machine code
(``bytes``) that can be loaded directly into ``Intel4004Simulator``.

=== Register pair convention ===

The 4004 has 16 × 4-bit registers, organized as 8 pairs (P0–P7):

    P0 (R0:R1)   — argument 0 / return value
    P1 (R2:R3)   — argument 1
    P2 (R4:R5)   — local virtual variable 0
    P3 (R6:R7)   — local virtual variable 1
    P4 (R8:R9)   — local virtual variable 2
    P5 (R10:R11) — local virtual variable 3
    P6 (R12:R13) — RAM address register (reserved for load_var / store_var)
    P7 (R14:R15) — scratch / immediate temp (reserved)

Each Tetrad u8 value is stored hi-nibble in R(2p), lo-nibble in R(2p+1).
``FIM Pp, d8`` loads both nibbles in one 2-byte instruction.

=== RAM variable layout ===

Tetrad local variables (``store_var`` / ``load_var``) are kept in
4004 RAM bank 0, register 0.  Variable i occupies character slots 2i (hi)
and 2i+1 (lo).  P6 is used as the SRC address pair.  Maximum 8 variables.

=== Deopt ===

Operations without a 4004 encoding (mul, div, mod, bitwise, io, call)
cause ``codegen(ir)`` to return ``None``.  The JIT then falls back to the
interpreter.

=== Two-pass assembler ===

Abstract instructions are tuples, e.g. ``("LD", 3)``, ``("JUN", "lbl_0")``.
A label marker ``("LABEL", "lbl_0")`` contributes zero bytes.

Pass 1 — compute byte offset of every instruction and build label → address.
Pass 2 — encode each instruction, resolving label names to ROM addresses.
"""

from __future__ import annotations

from intel4004_backend.ir import IRInstr

__all__ = ["codegen", "run_on_4004", "DeoptimizerError"]


class DeoptimizerError(Exception):
    """Raised when an IR operation cannot be compiled to 4004 binary."""


# ---------------------------------------------------------------------------
# Supported / unsupported IR ops
# ---------------------------------------------------------------------------

_DEOPT_OPS = frozenset({
    "mul", "div", "mod", "and", "or", "xor", "not",
    "shl", "shr", "logical_not", "io_in", "io_out", "call", "deopt",
})

_MAX_VARS = 6    # P0–P5 (P6 and P7 are reserved)
_MAX_PARAMS = 2  # only P0 and P1 can hold arguments


# ---------------------------------------------------------------------------
# Register pair layout helpers
# ---------------------------------------------------------------------------

def _hi(pair: int) -> int:
    """Return the index of the high-nibble register in a pair."""
    return pair * 2


def _lo(pair: int) -> int:
    """Return the index of the low-nibble register in a pair."""
    return pair * 2 + 1


# ---------------------------------------------------------------------------
# Abstract instruction size table (for the two-pass assembler)
# ---------------------------------------------------------------------------

_TWO_BYTE_MNEMS = frozenset({"FIM", "JUN", "JMS", "JCN", "ISZ"})


def _asm_size(instr: tuple) -> int:
    """Return the byte size of an abstract 4004 instruction."""
    return 0 if instr[0] == "LABEL" else (2 if instr[0] in _TWO_BYTE_MNEMS else 1)


# ---------------------------------------------------------------------------
# Abstract instruction encoder (pass 2)
# ---------------------------------------------------------------------------

def _resolve(operand: str | int, labels: dict[str, int]) -> int:
    return labels[operand] if isinstance(operand, str) else operand


def _encode(instr: tuple, labels: dict[str, int], current_addr: int) -> bytes:
    """Encode one abstract instruction to 4004 bytes."""
    m = instr[0]

    if m == "LABEL":
        return b""
    if m == "NOP":
        return bytes([0x00])
    if m == "HLT":
        return bytes([0x01])            # simulator-only halt
    if m == "CLB":
        return bytes([0xF0])
    if m == "CLC":
        return bytes([0xF1])
    if m == "IAC":
        return bytes([0xF2])
    if m == "CMC":
        return bytes([0xF3])
    if m == "CMA":
        return bytes([0xF4])
    if m == "RAL":
        return bytes([0xF5])
    if m == "RAR":
        return bytes([0xF6])
    if m == "TCC":
        return bytes([0xF7])
    if m == "DAC":
        return bytes([0xF8])
    if m == "TCS":
        return bytes([0xF9])
    if m == "STC":
        return bytes([0xFA])
    if m == "DAA":
        return bytes([0xFB])
    if m == "WRM":
        return bytes([0xE0])
    if m == "WMP":
        return bytes([0xE1])
    if m == "RDM":
        return bytes([0xE9])
    if m == "LD":
        return bytes([0xA0 | (instr[1] & 0xF)])
    if m == "XCH":
        return bytes([0xB0 | (instr[1] & 0xF)])
    if m == "ADD":
        return bytes([0x80 | (instr[1] & 0xF)])
    if m == "SUB":
        return bytes([0x90 | (instr[1] & 0xF)])
    if m == "INC":
        return bytes([0x60 | (instr[1] & 0xF)])
    if m == "LDM":
        return bytes([0xD0 | (instr[1] & 0xF)])
    if m == "BBL":
        return bytes([0xC0 | (instr[1] & 0xF)])
    if m == "SRC":
        pair = instr[1]
        return bytes([0x20 | (pair * 2 + 1)])
    if m == "FIM":
        pair, data = instr[1], instr[2] & 0xFF
        return bytes([0x20 | (pair * 2), data])
    if m == "JUN":
        target = _resolve(instr[1], labels)
        return bytes([0x40 | ((target >> 8) & 0xF), target & 0xFF])
    if m == "JMS":
        target = _resolve(instr[1], labels)
        return bytes([0x50 | ((target >> 8) & 0xF), target & 0xFF])
    if m == "JCN":
        cond, label = instr[1], instr[2]
        target = _resolve(label, labels)
        page_rel = target & 0xFF
        return bytes([0x10 | (cond & 0xF), page_rel])
    if m == "ISZ":
        reg, label = instr[1], instr[2]
        target = _resolve(label, labels)
        page_rel = target & 0xFF
        return bytes([0x70 | (reg & 0xF), page_rel])
    raise ValueError(f"unknown abstract mnemonic: {m!r}")


# ---------------------------------------------------------------------------
# Two-pass assembler
# ---------------------------------------------------------------------------

def _assemble(asm: list[tuple]) -> bytes:
    """Two-pass assembler: abstract instructions → Intel 4004 binary."""
    # Pass 1: compute byte addresses for each label.
    labels: dict[str, int] = {}
    addr = 0
    for instr in asm:
        if instr[0] == "LABEL":
            labels[instr[1]] = addr
        else:
            addr += _asm_size(instr)

    # Pass 2: encode every instruction.
    result = bytearray()
    addr = 0
    for instr in asm:
        b = _encode(instr, labels, addr)
        result.extend(b)
        addr += len(b)

    return bytes(result)


# ---------------------------------------------------------------------------
# Code generator
# ---------------------------------------------------------------------------

class _Codegen:
    """Stateful code generator: tracks variable → register pair allocation."""

    # Pair 6 (R12:R13) is the RAM address register.
    # Pair 7 (R14:R15) is the scratch temp for immediates.
    _RAM_ADDR_PAIR = 6
    _TEMP_PAIR = 7

    def __init__(self) -> None:
        self._asm: list[tuple] = []
        self._var_pair: dict[str, int] = {}
        self._next_pair = 0           # next allocatable pair (0=P0, 1=P1, …, 5=P5)
        self._ft_ctr = 0             # fall-through label counter (for comparisons)
        self._last_use: dict[str, int] = {}   # var → last IR index where it appears as src
        self._instr_idx: int = 0              # current IR instruction index

    # ------------------------------------------------------------------
    # Pair allocation
    # ------------------------------------------------------------------

    def _alloc_param(self, var: str, param_idx: int) -> None:
        """Bind a param variable to its fixed pair (P0 or P1)."""
        if param_idx >= _MAX_PARAMS:
            raise DeoptimizerError(
                f"too many params: only {_MAX_PARAMS} supported, got param {param_idx}"
            )
        self._var_pair[var] = param_idx
        # Ensure _next_pair is after all param slots.
        if self._next_pair <= param_idx:
            self._next_pair = param_idx + 1

    def _pair_of(self, var: str) -> int:
        """Return the pair index for *var*, allocating or recycling as needed.

        Liveness-based recycling: before allocating a fresh pair, scan for an
        existing variable whose last use is strictly before the current IR
        index (i.e., it is dead). P0 and P1 are param slots and never recycled.
        Lowest-numbered dead pair is chosen to keep allocation deterministic.
        """
        if var in self._var_pair:
            return self._var_pair[var]
        # Try to recycle the lowest-indexed dead pair (skip P0/P1 param slots).
        dead_candidate: tuple[int, str] | None = None  # (pair, dead_var)
        for dead_var, pair in self._var_pair.items():
            if (
                pair >= 2
                and self._last_use.get(dead_var, -1) < self._instr_idx
                and (dead_candidate is None or pair < dead_candidate[0])
            ):
                dead_candidate = (pair, dead_var)
        if dead_candidate is not None:
            recycled_pair, dead_var = dead_candidate
            del self._var_pair[dead_var]
            self._var_pair[var] = recycled_pair
            return recycled_pair
        # No recyclable pair; allocate fresh.
        if self._next_pair >= _MAX_VARS:
            raise DeoptimizerError(
                f"too many virtual variables: limit is {_MAX_VARS}"
            )
        self._var_pair[var] = self._next_pair
        self._next_pair += 1
        return self._var_pair[var]

    def _pair_src(self, src: str | int) -> int:
        """Return the pair holding *src* (variable or immediate).

        Immediates are loaded into the scratch pair P7.
        """
        if isinstance(src, int):
            # Load immediate into P7.
            self._asm.append(("FIM", self._TEMP_PAIR, src & 0xFF))
            return self._TEMP_PAIR
        return self._pair_of(src)

    # ------------------------------------------------------------------
    # Abstract instruction emitters
    # ------------------------------------------------------------------

    def _emit(self, *args: object) -> None:
        self._asm.append(tuple(args))

    def _new_ft_label(self) -> str:
        lbl = f"_ft_{self._ft_ctr}"
        self._ft_ctr += 1
        return lbl

    # ------------------------------------------------------------------
    # 8-bit operations on register pairs
    # ------------------------------------------------------------------

    def _emit_add(self, pa: int, pb: int, pv: int) -> None:
        """Emit: u8 add  Pa + Pb → Pv  (carry-propagating nibble add)."""
        self._emit("CLC")
        self._emit("LD",  _lo(pa))
        self._emit("ADD", _lo(pb))
        self._emit("XCH", _lo(pv))
        self._emit("LD",  _hi(pa))
        self._emit("ADD", _hi(pb))
        self._emit("XCH", _hi(pv))

    def _emit_sub(self, pa: int, pb: int, pv: int) -> None:
        """Emit: u8 sub  Pa - Pb → Pv.

        The simulator's SUB Rn computes A = A + ~Rn + (1 - CY), so to obtain
        A - B (no initial borrow) we start with CY=0 (borrow_in = 1).

        After the lo nibble: CY=1 means no borrow, CY=0 means borrow.
        We CMC before the hi nibble to flip the carry so the hi nibble uses
        the correct borrow-in: if lo had no borrow (CY=1→flipped→CY=0,
        borrow_in=1), hi gets A+~B+1 = A-B ✓; if lo had borrow (CY=0→
        flipped→CY=1, borrow_in=0), hi gets A+~B = A-B-1 ✓.
        """
        self._emit("CLC")
        self._emit("LD",  _lo(pa))
        self._emit("SUB", _lo(pb))
        self._emit("XCH", _lo(pv))
        self._emit("CMC")          # flip carry so hi uses correct borrow-in
        self._emit("LD",  _hi(pa))
        self._emit("SUB", _hi(pb))
        self._emit("XCH", _hi(pv))

    def _emit_copy(self, psrc: int, pdst: int) -> None:
        """Copy register pair Psrc → Pdst (4 instructions)."""
        if psrc == pdst:
            return
        self._emit("LD",  _hi(psrc))
        self._emit("XCH", _hi(pdst))
        self._emit("LD",  _lo(psrc))
        self._emit("XCH", _lo(pdst))

    def _emit_load_ram_var(self, var_idx: int, pdst: int) -> None:
        """Load 8-bit variable *var_idx* from RAM into register pair Pdst.

        RAM layout: var i → character 2i (hi nibble), character 2i+1 (lo nibble)
        in bank 0, register 0.  P6 is used as the SRC address pair.
        """
        hi_char = var_idx * 2
        lo_char = var_idx * 2 + 1
        rp = self._RAM_ADDR_PAIR
        # Load hi nibble.
        self._emit("FIM", rp, hi_char & 0xFF)
        self._emit("SRC", rp)
        self._emit("RDM")
        self._emit("XCH", _hi(pdst))
        # Load lo nibble.
        self._emit("FIM", rp, lo_char & 0xFF)
        self._emit("SRC", rp)
        self._emit("RDM")
        self._emit("XCH", _lo(pdst))

    def _emit_store_ram_var(self, var_idx: int, psrc: int) -> None:
        """Store 8-bit register pair Psrc into RAM variable *var_idx*."""
        hi_char = var_idx * 2
        lo_char = var_idx * 2 + 1
        rp = self._RAM_ADDR_PAIR
        # Store hi nibble.
        self._emit("FIM", rp, hi_char & 0xFF)
        self._emit("SRC", rp)
        self._emit("LD",  _hi(psrc))
        self._emit("WRM")
        # Store lo nibble.
        self._emit("FIM", rp, lo_char & 0xFF)
        self._emit("SRC", rp)
        self._emit("LD",  _lo(psrc))
        self._emit("WRM")

    def _emit_cmp_lt(self, pa: int, pb: int, pv: int) -> None:
        """Emit: Pv = 1 if Pa < Pb else 0.

        Compute Pa − Pb using CLC + CMC (same borrow-chain as _emit_sub but
        discarding the result nibbles).  After the hi nibble:
          CY=1 → Pa >= Pb (no borrow),  CY=0 → Pa < Pb (borrow).
        CMC gives CY=1 iff Pa < Pb; TCC materialises the boolean.
        """
        self._emit("CLC")
        self._emit("LD",  _lo(pa))
        self._emit("SUB", _lo(pb))          # lo subtract; result discarded
        self._emit("CMC")                   # flip for hi borrow-in
        self._emit("LD",  _hi(pa))
        self._emit("SUB", _hi(pb))          # hi subtract; CY=1 iff Pa>=Pb
        self._emit("CMC")                   # CY=1 iff Pa < Pb
        self._emit("TCC")                   # A = 1 if Pa<Pb, 0 otherwise
        self._emit("XCH", _lo(pv))
        self._emit("LDM", 0)
        self._emit("XCH", _hi(pv))

    def _emit_cmp_le(self, pa: int, pb: int, pv: int) -> None:
        """Emit: Pv = 1 if Pa <= Pb else 0.

        Compute Pb − Pa (reversed operands).  After the hi nibble:
          CY=1 → Pb >= Pa (no borrow) → Pa <= Pb.
        TCC gives the result directly.
        """
        self._emit("CLC")
        self._emit("LD",  _lo(pb))
        self._emit("SUB", _lo(pa))
        self._emit("CMC")
        self._emit("LD",  _hi(pb))
        self._emit("SUB", _hi(pa))          # CY=1 iff Pb >= Pa (Pa <= Pb)
        self._emit("TCC")                   # A = 1 if Pa<=Pb
        self._emit("XCH", _lo(pv))
        self._emit("LDM", 0)
        self._emit("XCH", _hi(pv))

    def _emit_cmp_gt(self, pa: int, pb: int, pv: int) -> None:
        """Emit: Pv = 1 if Pa > Pb else 0  (= Pb < Pa)."""
        self._emit_cmp_lt(pb, pa, pv)

    def _emit_cmp_ge(self, pa: int, pb: int, pv: int) -> None:
        """Emit: Pv = 1 if Pa >= Pb else 0  (= Pb <= Pa)."""
        self._emit_cmp_le(pb, pa, pv)

    def _emit_cmp_eq(self, pa: int, pb: int, pv: int) -> None:
        """Emit: Pv = 1 if Pa == Pb else 0.

        CLC + SUB gives A=0 when both nibbles are equal (A + ~B + 1 = 16
        mod 16 = 0 when A==B).  JCN 0xC jumps when A != 0.
        """
        ineq = self._new_ft_label()
        done = self._new_ft_label()

        # Check hi nibbles.
        self._emit("CLC")
        self._emit("LD",  _hi(pa))
        self._emit("SUB", _hi(pb))         # A=0 if equal
        self._emit("JCN", 0xC, ineq)      # jump if A != 0

        # Hi equal → check lo nibbles.
        self._emit("CLC")
        self._emit("LD",  _lo(pa))
        self._emit("SUB", _lo(pb))
        self._emit("JCN", 0xC, ineq)

        # Both equal: Pv = 1.
        self._emit("LDM", 1)
        self._emit("XCH", _lo(pv))
        self._emit("LDM", 0)
        self._emit("XCH", _hi(pv))
        self._emit("JUN", done)

        # Not equal: Pv = 0.
        self._emit("LABEL", ineq)
        self._emit("LDM", 0)
        self._emit("XCH", _lo(pv))
        self._emit("LDM", 0)
        self._emit("XCH", _hi(pv))

        self._emit("LABEL", done)

    def _emit_cmp_ne(self, pa: int, pb: int, pv: int) -> None:
        """Emit: Pv = 1 if Pa != Pb else 0.

        Same nibble-equality check as _emit_cmp_eq, with result inverted.
        """
        neq = self._new_ft_label()
        done = self._new_ft_label()

        # Check hi nibbles.
        self._emit("CLC")
        self._emit("LD",  _hi(pa))
        self._emit("SUB", _hi(pb))
        self._emit("JCN", 0xC, neq)   # hi differ → not equal → result=1

        # Hi equal → check lo.
        self._emit("CLC")
        self._emit("LD",  _lo(pa))
        self._emit("SUB", _lo(pb))
        self._emit("JCN", 0xC, neq)   # lo differ

        # Both equal: Pv = 0.
        self._emit("LDM", 0)
        self._emit("XCH", _lo(pv))
        self._emit("LDM", 0)
        self._emit("XCH", _hi(pv))
        self._emit("JUN", done)

        # Not equal: Pv = 1.
        self._emit("LABEL", neq)
        self._emit("LDM", 1)
        self._emit("XCH", _lo(pv))
        self._emit("LDM", 0)
        self._emit("XCH", _hi(pv))

        self._emit("LABEL", done)

    def _emit_jz(self, ptest: int, lbl: str) -> None:
        """Jump to *lbl* if register pair Ptest == 0.

        The 4004 can only test a 4-bit nibble at a time.  We check hi first:
        if hi ≠ 0 the pair is definitely nonzero.  If hi == 0 we then check lo.
        """
        ft = self._new_ft_label()
        # If hi != 0: not zero, fall through.
        self._emit("LD", _hi(ptest))
        self._emit("JCN", 0xC, ft)     # jump if A != 0 → skip the JUN lbl
        # hi == 0: check lo.
        self._emit("LD", _lo(ptest))
        self._emit("JCN", 0xC, ft)     # jump if lo != 0 → skip
        # Both zero: take the jump.
        self._emit("JUN", lbl)
        self._emit("LABEL", ft)

    def _emit_jnz(self, ptest: int, lbl: str) -> None:
        """Jump to *lbl* if register pair Ptest != 0."""
        # If hi != 0: jump immediately.
        self._emit("LD", _hi(ptest))
        self._emit("JCN", 0xC, lbl)
        # hi == 0: check lo.
        self._emit("LD", _lo(ptest))
        self._emit("JCN", 0xC, lbl)
        # Both zero: don't jump (fall through).

    # ------------------------------------------------------------------
    # Liveness pre-scan
    # ------------------------------------------------------------------

    def _compute_last_use(self, ir: list[IRInstr]) -> None:
        """Record the last IR index at which each variable appears as a source.

        This enables _pair_of() to recycle register pairs whose variable is
        dead at the current instruction, keeping peak pair usage low.
        """
        for idx, instr in enumerate(ir):
            for src in instr.srcs:
                if isinstance(src, str):
                    self._last_use[src] = idx

    # ------------------------------------------------------------------
    # Main IR → abstract-assembly translation
    # ------------------------------------------------------------------

    def generate(self, ir: list[IRInstr]) -> None:
        """Translate IR to abstract 4004 instructions stored in self._asm."""
        self._compute_last_use(ir)
        for idx, instr in enumerate(ir):
            self._instr_idx = idx
            op = instr.op

            if op in _DEOPT_OPS:
                raise DeoptimizerError(f"unsupported IR op: {op!r}")

            elif op == "param":
                self._alloc_param(instr.dst, instr.srcs[0])  # type: ignore[arg-type]

            elif op == "const":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                imm = instr.srcs[0]
                assert isinstance(imm, int)
                self._emit("FIM", pv, imm & 0xFF)

            elif op == "load_var":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                var_idx = instr.srcs[0]
                assert isinstance(var_idx, int)
                if var_idx * 2 + 1 > 15:
                    raise DeoptimizerError(
                        f"load_var: variable index {var_idx} exceeds RAM capacity"
                    )
                self._emit_load_ram_var(var_idx, pv)

            elif op == "store_var":
                var_idx = instr.srcs[0]
                src_var = instr.srcs[1]
                assert isinstance(var_idx, int)
                if var_idx * 2 + 1 > 15:
                    raise DeoptimizerError(
                        f"store_var: variable index {var_idx} exceeds RAM capacity"
                    )
                psrc = self._pair_src(src_var)  # type: ignore[arg-type]
                self._emit_store_ram_var(var_idx, psrc)

            elif op == "add":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_add(pa, pb, pv)

            elif op == "sub":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_sub(pa, pb, pv)

            elif op == "cmp_lt":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_cmp_lt(pa, pb, pv)

            elif op == "cmp_le":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_cmp_le(pa, pb, pv)

            elif op == "cmp_gt":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_cmp_gt(pa, pb, pv)

            elif op == "cmp_ge":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_cmp_ge(pa, pb, pv)

            elif op == "cmp_eq":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_cmp_eq(pa, pb, pv)

            elif op == "cmp_ne":
                assert instr.dst is not None
                pv = self._pair_of(instr.dst)
                pa = self._pair_src(instr.srcs[0])  # type: ignore[arg-type]
                pb = self._pair_src(instr.srcs[1])  # type: ignore[arg-type]
                self._emit_cmp_ne(pa, pb, pv)

            elif op == "jmp":
                lbl = instr.srcs[0]
                assert isinstance(lbl, str)
                self._emit("JUN", lbl)

            elif op == "jz":
                ptest = self._pair_of(instr.srcs[0])  # type: ignore[arg-type]
                lbl = instr.srcs[1]
                assert isinstance(lbl, str)
                self._emit_jz(ptest, lbl)

            elif op == "jnz":
                ptest = self._pair_of(instr.srcs[0])  # type: ignore[arg-type]
                lbl = instr.srcs[1]
                assert isinstance(lbl, str)
                self._emit_jnz(ptest, lbl)

            elif op == "label":
                lbl = instr.srcs[0]
                assert isinstance(lbl, str)
                self._emit("LABEL", lbl)

            elif op == "ret":
                src = instr.srcs[0]
                assert isinstance(src, str)
                presult = self._pair_of(src)
                # Move result to P0 (the return pair) if it is not already there.
                if presult != 0:
                    self._emit_copy(presult, 0)
                self._emit("HLT")

            else:
                raise DeoptimizerError(f"unhandled IR op: {op!r}")


# ---------------------------------------------------------------------------
# Public entry points
# ---------------------------------------------------------------------------


def codegen(ir: list[IRInstr]) -> bytes | None:
    """Compile IR to Intel 4004 machine code.

    Returns ``None`` if any IR instruction cannot be encoded (deopt path).
    """
    gen = _Codegen()
    try:
        gen.generate(ir)
    except DeoptimizerError:
        return None

    try:
        binary = _assemble(gen._asm)
    except Exception:
        return None

    # Guard: every compiled program must fit in one 4004 ROM page (256 bytes).
    if len(binary) > 256:
        return None

    return binary


def run_on_4004(binary: bytes, args: list[int]) -> int:
    """Load *binary* into a fresh ``Intel4004Simulator``, set arguments,
    run until HLT, and return the u8 result from P0 (R0:R1).

    The simulator's ``reset()`` zeroes all registers.  We then call
    ``_write_pair`` directly before execution to inject the argument values.
    """
    from intel4004_simulator import Intel4004Simulator  # noqa: PLC0415

    sim = Intel4004Simulator()
    sim.reset()
    sim.load_program(binary)
    sim._prepare_execution()  # noqa: SLF001

    if len(args) >= 1:
        sim._write_pair(0, args[0] & 0xFF)  # noqa: SLF001
    if len(args) >= 2:
        sim._write_pair(1, args[1] & 0xFF)  # noqa: SLF001

    for _ in range(100_000):
        if sim.halted:
            break
        if sim._code is None or sim._vm.pc >= len(sim._code.instructions):  # noqa: SLF001
            break
        sim.step()

    return sim._read_pair(0) & 0xFF  # noqa: SLF001
