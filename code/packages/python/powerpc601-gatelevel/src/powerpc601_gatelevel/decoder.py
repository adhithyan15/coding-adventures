"""decoder.py — Combinational instruction decoder for the PowerPC 601.

The decoder is a pure function: it takes a 32-bit instruction word and
returns a dict of all decoded fields.  No state is modified; this models
the combinational decode stage of a real pipeline.

PowerPC 601 Instruction Formats
────────────────────────────────
All instructions are 32 bits, big-endian.  Bit 31 is the MSB (most
significant bit).  In Python, we work with the instruction as an integer
where bit 31 has weight 2^31.

Primary opcode field: bits [31:26] (6 bits), extracted as (word >> 26) & 0x3F.

I-form (branch):   [31:26]OPCD [25:2]LI(24b) [1]AA [0]LK
B-form (br cond):  [31:26]OPCD [25:21]BO [20:16]BI [15:2]BD(14b) [1]AA [0]LK
D-form (imm arith):[31:26]OPCD [25:21]rD [20:16]rA [15:0]SIMM(16b)
X-form (reg-reg):  [31:26]OPCD [25:21]rS [20:16]rA [15:11]rB [10:1]XO(10b) [0]Rc
XO-form (arith):   [31:26]OPCD [25:21]rD [20:16]rA [15:11]rB [10]OE [9:1]XO(9b) [0]Rc
XFX-form (SPR):    [31:26]OPCD [25:21]rS [20:11]SPR(10b,split) [10:1]XO(10b) [0]-
XL-form (br via):  [31:26]OPCD [25:21]BO [20:16]BI [15:11]BH [10:1]XO(10b) [0]LK
M-form (rotate):   [31:26]OPCD [25:21]rS [20:16]rA [15:11]rB/SH [10:6]MB [5:1]ME [0]Rc

SPR encoding (split field in XFX-form):
  The 10-bit SPR is stored as two 5-bit halves:
    Bits [20:16]: high 5 bits of SPR (bits 5–9 of SPR number)
    Bits [15:11]: low 5 bits of SPR (bits 0–4 of SPR number)
  Decode: spr = ((bits[15:11]) << 5) | bits[20:16]
  Encode: spr_enc = ((spr & 0x1F) << 5) | ((spr >> 5) & 0x1F)

Mnemonic naming
───────────────
The decoder assigns a mnemonic based on opcode and extended opcode.
For instructions with the Rc bit set, the mnemonic gets a "." suffix.
"""

from __future__ import annotations

# ── Sign-extension helpers (address-domain arithmetic, not data-path) ──────────


def _sext16(v: int) -> int:
    """Sign-extend a 16-bit value to a Python signed integer."""
    v = v & 0xFFFF
    if v & 0x8000:
        return v - 0x10000
    return v


def _sext24(v: int) -> int:
    """Sign-extend a 24-bit value to a Python signed integer."""
    v = v & 0xFFFFFF
    if v & 0x800000:
        return v - 0x1000000
    return v


def _sext14(v: int) -> int:
    """Sign-extend a 14-bit value to a Python signed integer."""
    v = v & 0x3FFF
    if v & 0x2000:
        return v - 0x4000
    return v


# ── XO-form mnemonic table (opcode 31) ─────────────────────────────────────────

_XO_ARITH_MNEMONICS: dict[int, str] = {
    266: "add",
    10:  "addc",
    138: "adde",
    234: "addme",
    202: "addze",
    40:  "subf",
    8:   "subfc",
    136: "subfe",
    232: "subfme",
    200: "subfze",
    75:  "mulhw",
    11:  "mulhwu",
    235: "mullw",
    491: "divw",
    459: "divwu",
    104: "neg",
}

_X_LOGIC_MNEMONICS: dict[int, str] = {
    28:  "and",
    444: "or",
    316: "xor",
    476: "nand",
    124: "nor",
    284: "eqv",
    60:  "andc",
    412: "orc",
    24:  "slw",
    536: "srw",
    792: "sraw",
    824: "srawi",
    26:  "cntlzw",
    0:   "cmp",
    32:  "cmpl",
    20:  "lwarx",
    150: "stwcx.",
    247: "stbux",
    215: "stbx",
    439: "sthux",
    407: "sthx",
    183: "stwux",
    151: "stwx",
    87:  "lbzx",
    119: "lbzux",
    279: "lhzx",
    311: "lhzux",
    343: "lhax",
    375: "lhaux",
    23:  "lwzx",
    55:  "lwzux",
    467: "mtspr",
    339: "mfspr",
    19:  "mfcr",
    144: "mtcrf",
}

_XL_MNEMONICS: dict[int, str] = {
    16:  "bclr",
    528: "bcctr",
    257: "crand",
    289: "crnand",
    225: "cror",
    417: "crnor",
    193: "crxor",
    449: "creqv",
    33:  "crandc",
    129: "crorc",
    0:   "mcrf",
    150: "isync",
}

_PO_MNEMONICS: dict[int, str] = {
    8:  "subfic",
    10: "cmpli",
    11: "cmpi",
    12: "addic",
    13: "addic.",
    14: "addi",
    15: "addis",
    16: "bc",
    18: "b",
    24: "ori",
    25: "oris",
    26: "xori",
    27: "xoris",
    28: "andi.",
    29: "andis.",
    20: "rlwimi",
    21: "rlwinm",
    23: "rlwnm",
    32: "lwz",
    33: "lwzu",
    34: "lbz",
    35: "lbzu",
    36: "stw",
    37: "stwu",
    38: "stb",
    39: "stbu",
    40: "lhz",
    41: "lhzu",
    42: "lha",
    43: "lhau",
    44: "sth",
    45: "sthu",
    46: "lmw",
    47: "stmw",
}


def decode_instruction(word: int) -> dict:
    """Decode a 32-bit PowerPC instruction word into its fields.

    This is a pure function (no side effects) modeling the combinational
    decode stage of the PowerPC 601 pipeline.

    Returns a dict with keys:
      op      : 6-bit primary opcode (bits [31:26])
      rd      : bits [25:21] — destination register (also rS in store X-form)
      ra      : bits [20:16] — first source register
      rb      : bits [15:11] — second source register
      xo      : 10-bit extended opcode (bits [10:1]) for X/XO/XFX/XL-form
      xo9     : 9-bit XO (xo & 0x1FF) for XO-form arithmetic
      oe      : bit 10 — OE flag (enable overflow exception in XO-form)
      rc      : bit 0 — Rc flag (update CR0 based on result)
      simm    : signed 16-bit immediate (bits [15:0])
      uimm    : unsigned 16-bit immediate (bits [15:0])
      li      : signed 26-bit branch offset (bits [25:2] * 4 + sign-ext)
      bd      : signed 16-bit branch displacement (bits [15:2] * 4 + sign-ext)
      bo      : 5-bit BO field (bits [25:21]) — branch options
      bi      : 5-bit BI field (bits [20:16]) — CR bit index
      bh      : 5-bit BH field (bits [15:11]) — branch hint
      aa      : bit 1 — Absolute Address flag
      lk      : bit 0 — Link flag (save CIA+4 to LR)
      spr     : decoded 10-bit SPR number (after un-swapping)
      sh      : bits [15:11] — shift amount (X-form shifts)
      mb      : bits [10:6] — mask begin (M-form rotate)
      me      : bits [5:1] — mask end (M-form rotate)
      crfd    : bits [25:23] — CR field destination for compare
      fxm     : bits [19:12] — 8-bit field mask for mtcrf
      mnemonic: instruction name string
    """
    op = (word >> 26) & 0x3F

    # Common fields shared across formats
    rd   = (word >> 21) & 0x1F  # also rS in X-form stores
    ra   = (word >> 16) & 0x1F
    rb   = (word >> 11) & 0x1F
    xo   = (word >>  1) & 0x3FF
    xo9  = xo & 0x1FF
    oe   = (word >> 10) & 0x1
    rc   =  word        & 0x1
    lk   =  word        & 0x1
    aa   = (word >>  1) & 0x1
    simm = _sext16(word & 0xFFFF)
    uimm = word & 0xFFFF

    # I-form LI (bits [25:2], sign-extended 24-bit, then << 2)
    li_raw = (word >> 2) & 0xFF_FFFF
    li = _sext24(li_raw) << 2

    # B-form BD (bits [15:2], sign-extended 14-bit, then << 2)
    bd_raw = (word >> 2) & 0x3FFF
    bd = _sext14(bd_raw) << 2

    # Branch fields
    bo = (word >> 21) & 0x1F
    bi = (word >> 16) & 0x1F
    bh = (word >> 11) & 0x1F

    # SPR encoding: bits [20:11] split as high[20:16] | low[15:11]
    spr_enc = (word >> 11) & 0x3FF
    spr = ((spr_enc & 0x1F) << 5) | (spr_enc >> 5)

    # M-form rotate fields
    sh = rb   # shift amount from rB field in X-form shifts
    mb = (word >> 6) & 0x1F   # bits [10:6]
    me = (word >> 1) & 0x1F   # bits [5:1]

    # Compare field: crfD at bits [25:23]
    crfd = (word >> 23) & 0x7

    # mtcrf FXM field: bits [19:12]
    fxm = (word >> 12) & 0xFF

    # ── Determine mnemonic ─────────────────────────────────────────────────────
    if word == 0:
        mnemonic = "halt"
    elif op == 18:  # I-form branch
        dot = ""
        mnemonic = ("bl" if (lk and not aa) else
                    "ba" if (aa and not lk) else
                    "bla" if (lk and aa) else "b") + dot
    elif op == 16:  # B-form branch conditional
        mnemonic = "bc"
    elif op == 19:  # XL-form
        mnemonic = _XL_MNEMONICS.get(xo, f"xl_{xo}")
    elif op == 31:  # X/XO/XFX-form
        # Check XO-form arithmetic (9-bit XO, OE may be 0 or 1)
        base_xo9 = xo9
        if base_xo9 in _XO_ARITH_MNEMONICS:
            mn = _XO_ARITH_MNEMONICS[base_xo9]
            oe_suffix = "o" if oe else ""
            rc_suffix = "." if rc else ""
            mnemonic = mn + oe_suffix + rc_suffix
        elif xo in _X_LOGIC_MNEMONICS:
            mn = _X_LOGIC_MNEMONICS[xo]
            # stwcx. already has the dot in the table
            mnemonic = mn if mn.endswith(".") else mn + ("." if rc else "")
        else:
            mnemonic = f"x31_{xo}"
    elif op in _PO_MNEMONICS:
        mnemonic = _PO_MNEMONICS[op]
    else:
        mnemonic = f"op{op}"

    return {
        "op":      op,
        "rd":      rd,
        "ra":      ra,
        "rb":      rb,
        "xo":      xo,
        "xo9":     xo9,
        "oe":      oe,
        "rc":      rc,
        "lk":      lk,
        "aa":      aa,
        "simm":    simm,
        "uimm":    uimm,
        "li":      li,
        "bd":      bd,
        "bo":      bo,
        "bi":      bi,
        "bh":      bh,
        "spr":     spr,
        "sh":      sh,
        "mb":      mb,
        "me":      me,
        "crfd":    crfd,
        "fxm":     fxm,
        "mnemonic": mnemonic,
    }
