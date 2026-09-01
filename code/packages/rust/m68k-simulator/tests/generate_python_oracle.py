"""Generate deterministic full-state vectors from the Python 68000 oracle.

The committed JSONL uses a 64 KiB backing window so generation and Rust test
runs stay fast, while every byte in that window is included in the FNV hash.
Separate lifecycle tests pin the exact 16 MiB architectural machine.
"""

from __future__ import annotations

import json

from motorola_68000_simulator import M68KSimulator


def words(*values: int) -> list[int]:
    return [byte for value in values for byte in ((value >> 8) & 0xFF, value & 0xFF)]


def long(value: int) -> list[int]:
    return words((value >> 16) & 0xFFFF, value & 0xFFFF)


def fnv1a(data: bytes | bytearray) -> str:
    value = 0xCBF29CE484222325
    for byte in data:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def mem_long(address: int, value: int) -> list[list[int]]:
    return [[address + index, byte] for index, byte in enumerate(long(value))]


def case(name: str, program: list[int], *, d: list[int] | None = None,
         a: list[int] | None = None, sr: int = 0x2714,
         memory: list[list[int]] | None = None, steps: int = 1) -> dict:
    return {
        "name": name,
        "program": program,
        "d": d or [0x12345678, 0x10, 3, 0, 0, 0, 0, 0],
        "a": a or [0x200, 0x300, 0, 0, 0, 0, 0, 0xF000],
        "sr": sr,
        "memory": memory or [],
        "steps": steps,
    }


cases: list[dict] = []

# Every effective-address form as a MOVE.L source into D0.
ea_cases = [
    ("dn", 0, 1, [], [], None),
    ("an", 1, 0, [], [], None),
    ("ind", 2, 0, [], mem_long(0x200, 0x11223344), None),
    ("postinc", 3, 0, [], mem_long(0x200, 0x11223344), None),
    ("predec", 4, 0, [], mem_long(0x200, 0x11223344), [0x204, 0x300, 0, 0, 0, 0, 0, 0xF000]),
    ("disp16", 5, 0, words(0x0010), mem_long(0x210, 0x11223344), None),
    ("index", 6, 0, words(0x1810), mem_long(0x220, 0x11223344), None),
    ("abs_short", 7, 0, words(0x0220), mem_long(0x220, 0x11223344), None),
    ("abs_long", 7, 1, long(0x220), mem_long(0x220, 0x11223344), None),
    ("pc_disp", 7, 2, words(0x021E), mem_long(0x220, 0x11223344), None),
    ("pc_index", 7, 3, words(0x1810), mem_long(0x22, 0x11223344), None),
    ("immediate", 7, 4, long(0x11223344), [], None),
]
for name, mode, reg, extension, memory, addresses in ea_cases:
    cases.append(case(
        f"ea_{name}",
        words(0x2000 | (mode << 3) | reg) + extension,
        a=addresses,
        memory=memory,
    ))

# Complete line-0 immediate and bit-operation families.
for name, opcode in (("ori", 0x0080), ("andi", 0x0280), ("subi", 0x0480),
                     ("addi", 0x0680), ("eori", 0x0A80), ("cmpi", 0x0C80)):
    cases.append(case(name, words(opcode) + long(0x0000000F)))
for name, opcode in (("btst_imm", 0x0800), ("bchg_imm", 0x0840),
                     ("bclr_imm", 0x0880), ("bset_imm", 0x08C0)):
    cases.append(case(name, words(opcode, 0x0003)))
for name, opcode in (("btst_reg", 0x0300), ("bchg_reg", 0x0340),
                     ("bclr_reg", 0x0380), ("bset_reg", 0x03C0)):
    cases.append(case(name, words(opcode)))
cases.extend([
    case("ori_ccr", words(0x003C, 0x0005)),
    case("andi_ccr", words(0x023C, 0x0014)),
    case("eori_ccr", words(0x0A3C, 0x000F)),
])

# Line 4 miscellaneous and control/stack families.
for name, opcode in (("negx", 0x4080), ("clr", 0x4280), ("neg", 0x4480),
                     ("not", 0x4680), ("tst", 0x4A80), ("swap", 0x4840),
                     ("ext_w", 0x4880), ("ext_l", 0x48C0),
                     ("move_sr", 0x40C0), ("move_ccr", 0x42C0)):
    cases.append(case(name, words(opcode)))
cases.extend([
    case("move_imm_ccr", words(0x44FC, 0x001F)),
    case("move_imm_sr", words(0x46FC, 0x2305)),
    case("pea", words(0x4850)),
    case("lea", words(0x41D0)),
    case("link", words(0x4E50, 0xFFF8)),
    case("trap", words(0x4E43)),
    case("stop", words(0x4E72, 0x2700)),
])

# Remaining decode lines and arithmetic edge families.
cases.extend([
    case("addq", words(0x5280)),
    case("subq", words(0x5380)),
    case("seq", words(0x57C0)),
    case("bra", words(0x6002)),
    case("bcc", words(0x6402)),
    case("moveq", words(0x7080)),
    case("or", words(0x8081)),
    case("divu", words(0x80C1), d=[100, 7, 0, 0, 0, 0, 0, 0]),
    case("divs", words(0x81C1), d=[(-100) & 0xFFFFFFFF, (-7) & 0xFFFFFFFF, 0, 0, 0, 0, 0, 0]),
    case("divu_overflow", words(0x80C1), d=[0xFFFFFFFF, 1, 0, 0, 0, 0, 0, 0]),
    case("divs_overflow", words(0x81C1), d=[0x80000000, 0xFFFFFFFF, 0, 0, 0, 0, 0, 0]),
    case("divu_zero", words(0x80C1), d=[100, 0, 0, 0, 0, 0, 0, 0]),
    case("sub", words(0x9081)),
    case("subx", words(0x9181)),
    case("subx_borrow_wrap", words(0x9181), d=[0, 0xFFFFFFFF, 0, 0, 0, 0, 0, 0]),
    case("cmp", words(0xB081)),
    case("eor", words(0xB180)),
    case("and", words(0xC081)),
    case("mulu", words(0xC0C1), d=[9, 7, 0, 0, 0, 0, 0, 0]),
    case("muls", words(0xC1C1), d=[(-9) & 0xFFFFFFFF, (-7) & 0xFFFFFFFF, 0, 0, 0, 0, 0, 0]),
    case("exg", words(0xC141)),
    case("add", words(0xD081)),
    case("addx", words(0xD181)),
    case("addx_carry_chain", words(0xD181), d=[0xFFFFFFFF, 0, 0, 0, 0, 0, 0, 0]),
])

# All register shift types/directions plus the oracle's memory forms.
for name, opcode in (
    ("asr", 0xE280), ("asl", 0xE380), ("lsr", 0xE288), ("lsl", 0xE388),
    ("roxr", 0xE290), ("roxl", 0xE390), ("ror", 0xE298), ("rol", 0xE398),
):
    cases.append(case(name, words(opcode)))
for name, opcode in (("mem_as", 0xE0D0), ("mem_ls", 0xE2D0),
                     ("mem_rox", 0xE4D0), ("mem_ro", 0xE6D0)):
    cases.append(case(name, words(opcode), memory=[[0x200, 0x80], [0x201, 0x01]]))


for record in cases:
    simulator = M68KSimulator()
    simulator._mem = bytearray(65_536)
    simulator._d = list(record["d"])
    simulator._a = list(record["a"])
    simulator._pc = 0
    simulator._sr = record["sr"]
    simulator._halted = False
    simulator._mem[:len(record["program"])] = bytes(record["program"])
    for address, value in record["memory"]:
        simulator._mem[address] = value

    mnemonics = []
    error = None
    try:
        for _ in range(record["steps"]):
            if simulator._halted:
                break
            mnemonics.append(simulator.step().mnemonic)
    except (RuntimeError, ValueError, IndexError, ZeroDivisionError, OverflowError) as exc:
        error = str(exc)

    print(json.dumps({
        **record,
        "mnemonics": mnemonics,
        "error": error,
        "expected": {
            "d": simulator._d,
            "a": simulator._a,
            "pc": simulator._pc,
            "sr": simulator._sr,
            "halted": simulator._halted,
            "memory_hash": fnv1a(simulator._mem),
        },
    }, separators=(",", ":")))
