#!/usr/bin/env python3
"""Reproduce bounded LM32 manual and pinned-RTL transition evidence.

The transition oracle below is deliberately independent of the Rust package.
It implements the fixed profile directly from the Processor Reference Manual.
The RTL corpus selects a separate set of source-backed boundaries.  With
``--rtl-root`` the immutable source hashes are verified before regeneration.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import re
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "tests" / "vectors"
MASK = 0xFFFF_FFFF
MEMORY_SIZE = 65_536

RTL_COMMIT = "84b3e3ca0ad9535acaef201c1482342871358b08"
RTL_HASHES = {
    "lm32_decoder.v": "9fd441f3ba425ed79042e5305e229087ba5525b3bfb4cb13057c92119c7173c1",
    "lm32_load_store_unit.v": "172706c683180f5cae98b7f4bd26efcc674b03de164cd4e0922ee4bd4f22af39",
    "lm32_interrupt.v": "495f56e562abb418fccb099a1ad2f4ce86757e223248346844a8bb1e729fc7ca",
    "lm32_cpu.v": "e54421ff9fbc6c996cc81ebeb5d98cb16af6737ca2f8571ebd5bd0e79ea8aa17",
    "lm32_debug.v": "08e45a33e38c91415d3cd9fb8bcb190567839646ab0b7bb06fa8786cd867328d",
}

OPS = {
    0x00: ("srui", "sru"),
    0x01: ("nori", "nor"),
    0x02: ("muli", "mul"),
    0x03: ("sh", "sh"),
    0x04: ("lb", "lb"),
    0x05: ("sri", "sr"),
    0x06: ("xori", "xor"),
    0x07: ("lh", "lh"),
    0x08: ("andi", "and"),
    0x09: ("xnori", "xnor"),
    0x0A: ("lw", "lw"),
    0x0B: ("lhu", "lhu"),
    0x0C: ("sb", "sb"),
    0x0D: ("addi", "add"),
    0x0E: ("ori", "or"),
    0x0F: ("sli", "sl"),
    0x10: ("lbu", "lbu"),
    0x11: ("be", "be"),
    0x12: ("bg", "bg"),
    0x13: ("bge", "bge"),
    0x14: ("bgeu", "bgeu"),
    0x15: ("bgu", "bgu"),
    0x16: ("sw", "sw"),
    0x17: ("bne", "bne"),
    0x18: ("andhi", "andhi"),
    0x19: ("cmpei", "cmpe"),
    0x1A: ("cmpgi", "cmpg"),
    0x1B: ("cmpgei", "cmpge"),
    0x1C: ("cmpgeui", "cmpgeu"),
    0x1D: ("cmpgui", "cmpgu"),
    0x1E: ("orhi", "orhi"),
    0x1F: ("cmpnei", "cmpne"),
    0x20: ("sru", "sru"),
    0x21: ("nor", "nor"),
    0x22: ("mul", "mul"),
    0x23: ("divu", "divu"),
    0x24: ("rcsr", "rcsr"),
    0x25: ("sr", "sr"),
    0x26: ("xor", "xor"),
    0x28: ("and", "and"),
    0x29: ("xnor", "xnor"),
    0x2C: ("sextb", "sextb"),
    0x2D: ("add", "add"),
    0x2E: ("or", "or"),
    0x2F: ("sl", "sl"),
    0x30: ("b", "branch"),
    0x31: ("modu", "modu"),
    0x32: ("sub", "sub"),
    0x34: ("wcsr", "wcsr"),
    0x36: ("call", "call"),
    0x37: ("sexth", "sexth"),
    0x38: ("bi", "bi"),
    0x39: ("cmpe", "cmpe"),
    0x3A: ("cmpg", "cmpg"),
    0x3B: ("cmpge", "cmpge"),
    0x3C: ("cmpgeu", "cmpgeu"),
    0x3D: ("cmpgu", "cmpgu"),
    0x3E: ("calli", "calli"),
    0x3F: ("cmpne", "cmpne"),
}


def u32(value: int) -> int:
    return value & MASK


def s32(value: int) -> int:
    value &= MASK
    return value - (1 << 32) if value & (1 << 31) else value


def sext(value: int, bits: int) -> int:
    sign = 1 << (bits - 1)
    return u32((value & (sign - 1)) - (value & sign))


def imm(op: int, ry: int = 1, rx: int = 3, value: int = 1) -> int:
    return (op << 26) | (ry << 21) | (rx << 16) | (value & 0xFFFF)


def reg(op: int, ry: int = 1, rz: int = 2, rx: int = 3, low: int = 0) -> int:
    return (op << 26) | (ry << 21) | (rz << 16) | (rx << 11) | low


@dataclass
class State:
    gpr: list[int] = field(default_factory=lambda: [0] * 32)
    pc: int = 0x100
    ie: bool = False
    eie: bool = False
    bie: bool = False
    dc_re: bool = False
    im: int = 0
    ip: int = 0
    lines: int = 0
    eba: int = 0
    deba: int = 0
    memory: bytearray = field(default_factory=lambda: bytearray(MEMORY_SIZE))


@dataclass
class Case:
    name: str
    word: int
    initial: State
    expected: State
    trace: dict


def tx(
    kind: str, address: int, width: int, value: int, enable: int, success: bool
) -> tuple:
    return (kind, u32(address), width, u32(value), enable, success)


def byte_enable(address: int, width: int) -> int:
    if width == 1:
        return 1 << (3 - (address & 3))
    if width == 2:
        return 0xC if address & 2 == 0 else 0x3
    return 0xF


def take_exception(state: State, eid: int, pc: int, debug: bool) -> tuple:
    before = state.ie
    if debug:
        state.gpr[31] = pc
        state.bie = state.ie
        register, base, saved = 31, state.deba, state.bie
    else:
        state.gpr[30] = pc
        state.eie = state.ie
        register = 30
        base = state.deba if state.dc_re else state.eba
        saved = state.eie
    state.ie = False
    vector = u32(base + eid * 32)
    state.pc = vector
    return (eid, register, pc, state.dc_re, base, vector, before, saved)


def read_csr(state: State, index: int) -> int:
    return {
        0: int(state.ie) | (int(state.eie) << 1) | (int(state.bie) << 2),
        1: state.im,
        2: state.ip,
        6: 0x0802_010F,
        7: state.eba,
        9: state.deba,
        10: 0,
    }[index]


def write_csr(state: State, index: int, value: int) -> None:
    if index == 0:
        state.ie, state.eie, state.bie = (
            bool(value & 1),
            bool(value & 2),
            bool(value & 4),
        )
    elif index == 1:
        state.im = value
    elif index == 2:
        state.ip &= ~value & MASK
    elif index == 7:
        state.eba = value if value & 0xFF == 0 else 0
    elif index == 8:
        state.dc_re = bool(value & 2)
    elif index == 9:
        state.deba = value if value & 0xFF == 0 else 0
    else:
        raise ValueError(f"unsupported write CSR {index}")


def step(initial: State, word: int) -> tuple[State, dict]:
    state = copy.deepcopy(initial)
    pc = state.pc
    previous_ip = state.ip
    state.ip |= state.lines
    pending = state.im & state.ip
    accepted = pending if state.ie else 0
    interrupt = (state.lines, state.ip & ~previous_ip, pending, accepted)
    transactions: list[tuple] = []
    if accepted:
        exception = take_exception(state, 6, pc, False)
        return state, trace(
            pc,
            state,
            None,
            "interrupt",
            (None, None),
            (None, None),
            None,
            None,
            transactions,
            interrupt,
            exception,
        )
    if pc > MEMORY_SIZE - 4:
        transactions.append(tx("InstructionRead", pc, 4, 0, 0xF, False))
        exception = take_exception(state, 2, pc, False)
        return state, trace(
            pc,
            state,
            None,
            "instruction_bus_error",
            (None, None),
            (None, None),
            None,
            None,
            transactions,
            interrupt,
            exception,
        )

    transactions.append(tx("InstructionRead", pc, 4, word, 0xF, True))
    opcode = word >> 26
    if word == 0xAC00_0002:
        mnemonic, operation = "break", "break"
        source0 = source1 = destination = immediate = 0
    elif word == 0xAC00_0007:
        mnemonic, operation = "scall", "scall"
        source0 = source1 = destination = immediate = 0
    else:
        mnemonic, operation = OPS[opcode]
        source0, source1 = (word >> 21) & 31, (word >> 16) & 31
        destination = source1 if opcode < 0x20 else (word >> 11) & 31
        if opcode in (0x38, 0x3E):
            immediate = sext(word & 0x03FF_FFFF, 26)
        elif opcode in (0x18, 0x1E):
            immediate = (word & 0xFFFF) << 16
        elif opcode in (0x01, 0x06, 0x08, 0x09, 0x0E, 0x1C, 0x1D):
            immediate = word & 0xFFFF
        else:
            immediate = sext(word & 0xFFFF, 16)

    a, b = state.gpr[source0], state.gpr[source1]
    register_form = bool(word >> 31)
    sources = (source0, source1) if register_form else (source0, None)
    values = (a, b) if register_form else (a, None)
    state.pc = u32(pc + 4)
    result = None
    branch = None
    exception = None
    rhs = b if register_form else immediate

    if operation == "add":
        result = u32(a + rhs)
    elif operation == "sub":
        result = u32(a - b)
    elif operation == "mul":
        result = u32(a * rhs)
    elif operation in ("divu", "modu"):
        if b == 0:
            exception = take_exception(state, 5, pc, False)
        else:
            result = a // b if operation == "divu" else a % b
    elif operation == "and":
        result = a & rhs
    elif operation == "andhi":
        result = a & immediate
    elif operation == "or":
        result = a | rhs
    elif operation == "orhi":
        result = a | immediate
    elif operation == "xor":
        result = a ^ rhs
    elif operation == "xnor":
        result = u32(~(a ^ rhs))
    elif operation == "nor":
        result = u32(~(a | rhs))
    elif operation == "sl":
        result = u32(a << (rhs & 31))
    elif operation == "sr":
        result = u32(s32(a) >> (rhs & 31))
    elif operation == "sru":
        result = a >> (rhs & 31)
    elif operation == "sextb":
        result = sext(a & 0xFF, 8)
    elif operation == "sexth":
        result = sext(a & 0xFFFF, 16)
    elif operation in ("cmpe", "cmpne", "cmpg", "cmpge", "cmpgu", "cmpgeu"):
        comparisons = {
            "cmpe": a == rhs,
            "cmpne": a != rhs,
            "cmpg": s32(a) > s32(rhs),
            "cmpge": s32(a) >= s32(rhs),
            "cmpgu": a > rhs,
            "cmpgeu": a >= rhs,
        }
        result = int(comparisons[operation])
    elif operation in ("lb", "lbu", "lh", "lhu", "lw"):
        width = 4 if operation == "lw" else 2 if operation in ("lh", "lhu") else 1
        address = u32(a + immediate)
        enable = byte_enable(address, width)
        if address + width > MEMORY_SIZE:
            transactions.append(tx("DataRead", address, width, 0, enable, False))
            exception = take_exception(state, 4, pc, False)
        else:
            value = int.from_bytes(state.memory[address : address + width], "big")
            transactions.append(tx("DataRead", address, width, value, enable, True))
            result = sext(value, width * 8) if operation in ("lb", "lh") else value
    elif operation in ("sb", "sh", "sw"):
        width = 4 if operation == "sw" else 2 if operation == "sh" else 1
        address = u32(a + immediate)
        enable = byte_enable(address, width)
        narrow = b & ((1 << (width * 8)) - 1)
        bus = narrow * (0x0101_0101 if width == 1 else 0x0001_0001 if width == 2 else 1)
        if address + width > MEMORY_SIZE:
            transactions.append(tx("DataWrite", address, width, bus, enable, False))
            exception = take_exception(state, 4, pc, False)
        else:
            state.memory[address : address + width] = narrow.to_bytes(width, "big")
            transactions.append(tx("DataWrite", address, width, bus, enable, True))
        sources, values = (source0, source1), (a, b)
    elif operation in ("be", "bne", "bg", "bge", "bgu", "bgeu"):
        predicates = {
            "be": a == b,
            "bne": a != b,
            "bg": s32(a) > s32(b),
            "bge": s32(a) >= s32(b),
            "bgu": a > b,
            "bgeu": a >= b,
        }
        taken = predicates[operation]
        target = u32(pc + u32(immediate << 2))
        if taken:
            state.pc = target
        branch = (taken, target)
        sources, values = (source0, source1), (a, b)
    elif operation in ("bi", "calli"):
        target = u32(pc + u32(immediate << 2))
        if operation == "calli":
            state.gpr[29] = u32(pc + 4)
            result = state.gpr[29]
        state.pc = target
        branch = (True, target)
        sources, values = (None, None), (None, None)
    elif operation in ("branch", "call"):
        target = a
        if operation == "call":
            state.gpr[29] = u32(pc + 4)
            result = state.gpr[29]
        if operation == "branch" and source0 == 30:
            state.ie = state.eie
        if operation == "branch" and source0 == 31:
            state.ie = state.bie
        state.pc = target
        branch = (True, target)
        sources, values = (source0, None), (a, None)
        if source0 == 30:
            mnemonic = "eret"
        if source0 == 31:
            mnemonic = "bret"
    elif operation == "rcsr":
        result = read_csr(state, source0)
        sources, values = (None, None), (None, None)
    elif operation == "wcsr":
        write_csr(state, source0, b)
        sources, values = (source1, None), (b, None)
    elif operation == "break":
        exception = take_exception(state, 1, pc, True)
        sources, values = (None, None), (None, None)
    elif operation == "scall":
        exception = take_exception(state, 7, pc, False)
        sources, values = (None, None), (None, None)
    else:
        raise AssertionError(operation)

    destination_effect = None
    if result is not None:
        register = 29 if operation in ("call", "calli") else destination
        state.gpr[register] = u32(result)
        destination_effect = (register, u32(result))
    return state, trace(
        pc,
        state,
        word,
        mnemonic,
        sources,
        values,
        destination_effect,
        branch,
        transactions,
        interrupt,
        exception,
    )


def trace(
    pc: int,
    state: State,
    word: int | None,
    mnemonic: str,
    sources: tuple,
    values: tuple,
    destination: tuple | None,
    branch: tuple | None,
    transactions: list[tuple],
    interrupt: tuple,
    exception: tuple | None,
) -> dict:
    return {
        "pc_before": pc,
        "pc_after": state.pc,
        "word": word,
        "mnemonic": mnemonic,
        "sources": sources,
        "values": values,
        "destination": destination,
        "branch": branch,
        "transactions": transactions,
        "interrupt": interrupt,
        "exception": exception,
    }


def base_state(word: int) -> State:
    state = State()
    state.memory[0x100:0x104] = word.to_bytes(4, "big")
    state.gpr[1] = 0x1234_5678
    state.gpr[2] = 3
    state.memory[0x200:0x204] = bytes.fromhex("80ff7f01")
    return state


def loaded_state(word: int) -> State:
    """State immediately after Rust `load` before vector-specific setup."""
    state = State()
    state.pc = 0x100
    state.memory[0x100:0x104] = word.to_bytes(4, "big")
    return state


def make_case(name: str, word: int, mutate=None) -> Case:
    initial = base_state(word)
    if mutate:
        mutate(initial)
    expected, expected_trace = step(initial, word)
    return Case(name, word, initial, expected, expected_trace)


def canonical_word(opcode: int) -> int:
    if opcode == 0x24:
        return reg(opcode, 6, 0, 3)
    if opcode == 0x34:
        return reg(opcode, 0, 2, 0)
    if opcode in (0x30, 0x36):
        return reg(opcode, 1, 0, 0)
    if opcode in (0x38, 0x3E):
        return opcode << 26
    if opcode >= 0x20:
        return reg(opcode)
    if opcode in (0x03, 0x04, 0x07, 0x0A, 0x0B, 0x0C, 0x10, 0x16):
        return imm(opcode, value=0)
    return imm(opcode)


def manual_corpus() -> list[Case]:
    kind = "manual"
    cases: list[Case] = []
    for opcode in OPS:
        word = canonical_word(opcode)

        def setup(state: State, op=opcode):
            if op in (0x03, 0x04, 0x07, 0x0A, 0x0B, 0x0C, 0x10, 0x16):
                state.gpr[1] = 0x200
            if op in (0x30, 0x36):
                state.gpr[1] = 0x100
            if op == 0x34:
                state.gpr[2] = 7

        cases.append(make_case(f"{kind}_opcode_{opcode:02x}", word, setup))
    cases.append(make_case(f"{kind}_raise_break", 0xAC00_0002))
    cases.append(make_case(f"{kind}_raise_scall", 0xAC00_0007))

    for opcode in (0x11, 0x12, 0x13, 0x14, 0x15, 0x17):
        word = imm(opcode, 1, 2, 1)
        cases.append(
            make_case(
                f"{kind}_branch_{opcode:02x}_false",
                word,
                lambda s: (s.gpr.__setitem__(1, 0), s.gpr.__setitem__(2, 1)),
            )
        )
    for opcode in (0x00, 0x05, 0x0F, 0x20, 0x25, 0x2F):
        for count in (0, 1, 31, 32, 63):
            word = imm(opcode, 1, 3, count) if opcode < 0x20 else reg(opcode)

            def shift_setup(state: State, op=opcode, value=count):
                state.gpr[1] = 0x8000_0001
                state.gpr[2] = value

            cases.append(
                make_case(f"{kind}_shift_{opcode:02x}_{count}", word, shift_setup)
            )

    for opcode, width, offsets in (
        (0x0C, 1, range(4)),
        (0x03, 2, (0, 2)),
        (0x16, 4, (0,)),
    ):
        for offset in offsets:
            word = imm(opcode, 1, 2, offset)
            for failing in (False, True):

                def store_setup(state: State, off=offset, bad=failing):
                    state.gpr[1] = 0x1_0000 if bad else 0x200
                    state.gpr[2] = 0x1234_ABCD

                cases.append(
                    make_case(
                        f"{kind}_store_{width}_{offset}_{'error' if failing else 'ok'}",
                        word,
                        store_setup,
                    )
                )

    for opcode, width, offsets in (
        (0x04, 1, range(4)),
        (0x10, 1, range(4)),
        (0x07, 2, (0, 2)),
        (0x0B, 2, (0, 2)),
        (0x0A, 4, (0,)),
    ):
        for offset in offsets:
            word = imm(opcode, 1, 3, offset)
            cases.append(
                make_case(
                    f"{kind}_load_{width}_{opcode:02x}_{offset}",
                    word,
                    lambda s: s.gpr.__setitem__(1, 0x200),
                )
            )

    for csr in (0, 1, 2, 6, 7, 9, 10):
        word = reg(0x24, csr, 0, 3)

        def read_setup(state: State):
            state.ie, state.eie, state.bie = True, True, False
            state.im, state.ip, state.eba, state.deba = 0x55, 0xAA, 0x1200, 0x3400

        cases.append(make_case(f"{kind}_rcsr_{csr}", word, read_setup))
    for csr, value in (
        (0, 7),
        (1, 0x55AA),
        (2, 0xF0),
        (7, 0x1200),
        (8, 2),
        (9, 0x3400),
    ):
        word = reg(0x34, csr, 2, 0)

        def write_setup(state: State, v=value):
            state.gpr[2] = v
            state.ip = 0xFFFF

        cases.append(make_case(f"{kind}_wcsr_{csr}", word, write_setup))

    ignored_positions = list(range(0, 11)) + list(range(16, 21))
    ignored_masks = [0] + [1 << bit for bit in ignored_positions]
    ignored_masks.append(sum(1 << bit for bit in ignored_positions))
    for opcode in (0x24, 0x2C, 0x37):
        source = 6 if opcode == 0x24 else 1
        for ordinal, mask in enumerate(ignored_masks):
            word = (opcode << 26) | (source << 21) | (3 << 11) | mask
            cases.append(make_case(f"{kind}_ignored_{opcode:02x}_{ordinal:02d}", word))

    for name, word, mutate in (
        ("scall", 0xAC00_0007, None),
        ("break", 0xAC00_0002, None),
        ("divide_zero", reg(0x23), lambda s: s.gpr.__setitem__(2, 0)),
        ("data_bus", imm(0x0A, 1, 3, 0), lambda s: s.gpr.__setitem__(1, 0x1_0000)),
        ("instruction_bus", 0, lambda s: setattr(s, "pc", 0xFFFF_FFFC)),
    ):
        for remap in (False, True):

            def exception_setup(state: State, inner=mutate, re=remap):
                state.ie, state.dc_re, state.eba, state.deba = True, re, 0x1000, 0x2000
                if inner:
                    inner(state)

            cases.append(
                make_case(
                    f"{kind}_exception_{name}_re{int(remap)}", word, exception_setup
                )
            )

    for line in range(32):
        for remap in (False, True):
            bit = 1 << line

            def interrupt_setup(state: State, mask=bit, re=remap):
                state.ie, state.im, state.lines = True, mask, mask
                state.dc_re, state.eba, state.deba = re, 0x1000, 0x2000

            cases.append(
                make_case(
                    f"{kind}_interrupt_{line}_re{int(remap)}",
                    imm(0x0D),
                    interrupt_setup,
                )
            )

    # The accepted manual matrix requires both outcomes for every predicate
    # at the positive and negative 16-bit displacement endpoints.
    branch_operands = {
        0x11: ((7, 7), (7, 8)),
        0x17: ((7, 8), (7, 7)),
        0x12: ((1, 0), (0, 1)),
        0x13: ((1, 1), (0, 1)),
        0x15: ((2, 1), (1, 2)),
        0x14: ((1, 1), (1, 2)),
    }
    for opcode, (taken_operands, untaken_operands) in branch_operands.items():
        for taken, operands in ((True, taken_operands), (False, untaken_operands)):
            for offset in (0x7FFF, 0x8000):
                word = imm(opcode, 1, 2, offset)

                def branch_setup(state: State, pair=operands):
                    state.gpr[1], state.gpr[2] = pair

                cases.append(
                    make_case(
                        f"manual_branch_{opcode:02x}_{'taken' if taken else 'not_taken'}_{offset:04x}",
                        word,
                        branch_setup,
                    )
                )

    # The I-format control operations have a distinct signed 26-bit range.
    for opcode in (0x38, 0x3E):
        for offset in (0x01FF_FFFF, 0x0200_0000):
            cases.append(
                make_case(
                    f"manual_control_{opcode:02x}_{offset:07x}",
                    (opcode << 26) | offset,
                )
            )

    # Legal data accesses at the last byte, half-word, and word complement
    # the low-address and out-of-range vectors above.
    for opcode, width, address in (
        (0x04, 1, 0xFFFF),
        (0x10, 1, 0xFFFF),
        (0x07, 2, 0xFFFE),
        (0x0B, 2, 0xFFFE),
        (0x0A, 4, 0xFFFC),
    ):

        def top_load_setup(state: State, base=address):
            state.gpr[1] = base
            state.memory[0xFFFC:0x10000] = bytes.fromhex("80ff7f01")

        cases.append(
            make_case(
                f"manual_top_load_{width}_{opcode:02x}",
                imm(opcode, 1, 3, 0),
                top_load_setup,
            )
        )
    for opcode, width, address in (
        (0x0C, 1, 0xFFFF),
        (0x03, 2, 0xFFFE),
        (0x16, 4, 0xFFFC),
    ):

        def top_store_setup(state: State, base=address):
            state.gpr[1] = base
            state.gpr[2] = 0x1234_ABCD

        cases.append(
            make_case(
                f"manual_top_store_{width}", imm(opcode, 1, 2, 0), top_store_setup
            )
        )

    # Source capture must precede the architectural ra link write.
    cases.append(
        make_case(
            "manual_call_ra_overlap",
            reg(0x36, 29, 0, 0),
            lambda state: state.gpr.__setitem__(29, 0x200),
        )
    )
    for name, word, mutate in (
        ("ret", reg(0x30, 29, 0, 0), lambda state: state.gpr.__setitem__(29, 0x200)),
        ("mv", reg(0x2E, 1, 0, 3), lambda state: state.gpr.__setitem__(1, 0x55AA_00FF)),
        ("mvhi", imm(0x1E, 0, 3, 0xA55A), None),
        (
            "not",
            imm(0x09, 1, 3, 0),
            lambda state: state.gpr.__setitem__(1, 0x55AA_00FF),
        ),
        ("mvi", imm(0x0D, 0, 3, 0x8001), None),
        ("nop", imm(0x0D, 0, 0, 0), None),
    ):
        cases.append(make_case(f"manual_alias_{name}", word, mutate))
    return cases


# Unlike the manual matrix above, this corpus is organized around concrete
# constructs parsed and hash-locked in the pinned RTL: decoder opcode wires,
# decoder groups, store-data/lane muxes, CSR storage, exception routing, and
# interrupt latch/acceptance logic. Expected architectural results still come
# from the independent manual oracle because the RTL is topology evidence, not
# production semantics.
RTL_OPCODE_WIRES = {
    "add": 0x2D,
    "and": 0x28,
    "andhi": 0x18,
    "b": 0x30,
    "bi": 0x38,
    "be": 0x11,
    "bg": 0x12,
    "bge": 0x13,
    "bgeu": 0x14,
    "bgu": 0x15,
    "bne": 0x17,
    "call": 0x36,
    "calli": 0x3E,
    "cmpe": 0x39,
    "cmpg": 0x3A,
    "cmpge": 0x3B,
    "cmpgeu": 0x3C,
    "cmpgu": 0x3D,
    "cmpne": 0x3F,
    "divu": 0x23,
    "lb": 0x04,
    "lbu": 0x10,
    "lh": 0x07,
    "lhu": 0x0B,
    "lw": 0x0A,
    "modu": 0x31,
    "mul": 0x22,
    "nor": 0x21,
    "or": 0x2E,
    "orhi": 0x1E,
    "rcsr": 0x24,
    "sb": 0x0C,
    "sextb": 0x2C,
    "sexth": 0x37,
    "sh": 0x03,
    "sl": 0x2F,
    "sr": 0x25,
    "sru": 0x20,
    "sub": 0x32,
    "sw": 0x16,
    "wcsr": 0x34,
    "xnor": 0x29,
    "xor": 0x26,
}


def rtl_corpus(opcode_wires: dict[str, int]) -> list[Case]:
    cases: list[Case] = []
    for wire, opcode in sorted(opcode_wires.items()):
        word = canonical_word(opcode)

        def decode_setup(state: State, op=opcode):
            if op in (0x03, 0x04, 0x07, 0x0A, 0x0B, 0x0C, 0x10, 0x16):
                state.gpr[1] = 0x200
            if op in (0x30, 0x36):
                state.gpr[1] = 0x100
            if op == 0x34:
                state.gpr[2] = 7

        cases.append(make_case(f"rtl_decoder_wire_{wire}", word, decode_setup))

    # The RTL selects register versus immediate operands from instruction bit
    # 31 around shared operation wires. Exercise every legal immediate primary
    # opcode as its own source-derived mux case rather than duplicating the
    # manual corpus's semantic matrix.
    for opcode in range(0x20):
        if opcode not in OPS:
            continue
        word = canonical_word(opcode)

        def immediate_setup(state: State, op=opcode):
            if op in (0x03, 0x04, 0x07, 0x0A, 0x0B, 0x0C, 0x10, 0x16):
                state.gpr[1] = 0x200

        cases.append(
            make_case(f"rtl_immediate_mux_{OPS[opcode][0]}", word, immediate_setup)
        )

    # RAISE is selected by a shared RTL wire and then split into the two
    # profile-visible canonical encodings.
    cases.extend(
        (
            make_case("rtl_raise_break", 0xAC00_0002),
            make_case("rtl_raise_scall", 0xAC00_0007),
        )
    )

    # Concrete load/store mux topology: every byte lane, both half lanes, the
    # word lane, legal top-of-memory addresses, and failed external requests.
    for opcode, width, offsets in (
        (0x0C, 1, range(4)),
        (0x03, 2, (0, 2)),
        (0x16, 4, (0,)),
    ):
        for offset in offsets:
            for label, base in (("lane", 0x200), ("bus_error", 0x1_0000)):

                def rtl_store_setup(state: State, value=base):
                    state.gpr[1] = value
                    state.gpr[2] = 0x1234_ABCD

                cases.append(
                    make_case(
                        f"rtl_store_mux_{width}_{offset}_{label}",
                        imm(opcode, 1, 2, offset),
                        rtl_store_setup,
                    )
                )
    for opcode, width, address in (
        (0x04, 1, 0xFFFF),
        (0x10, 1, 0xFFFF),
        (0x07, 2, 0xFFFE),
        (0x0B, 2, 0xFFFE),
        (0x0A, 4, 0xFFFC),
    ):

        def rtl_top_load_setup(state: State, base=address):
            state.gpr[1] = base
            state.memory[0xFFFC:0x10000] = bytes.fromhex("80ff7f01")

        cases.append(
            make_case(
                f"rtl_load_mux_top_{width}_{opcode:02x}",
                imm(opcode, 1, 3, 0),
                rtl_top_load_setup,
            )
        )

    # CSR mux/storage and debug-remap storage are separate RTL blocks.
    for csr in (0, 1, 2, 6, 7, 9, 10):

        def rtl_read_setup(state: State):
            state.ie, state.eie, state.bie = True, True, False
            state.im, state.ip, state.eba, state.deba = 0x55, 0xAA, 0x1200, 0x3400

        cases.append(
            make_case(f"rtl_csr_read_{csr}", reg(0x24, csr, 0, 3), rtl_read_setup)
        )
    for csr, value in (
        (0, 7),
        (1, 0x55AA),
        (2, 0xF0),
        (7, 0x1200),
        (8, 2),
        (9, 0x3400),
    ):

        def rtl_write_setup(state: State, data=value):
            state.gpr[2] = data
            state.ip = 0xFFFF

        cases.append(
            make_case(f"rtl_csr_write_{csr}", reg(0x34, csr, 2, 0), rtl_write_setup)
        )

    # Each modeled RTL exception source is checked with both normal and
    # remapped bases, while breakpoint remains on its dedicated debug path.
    for name, word, mutate in (
        ("scall", 0xAC00_0007, None),
        ("break", 0xAC00_0002, None),
        ("divide_zero", reg(0x23), lambda state: state.gpr.__setitem__(2, 0)),
        (
            "data_bus",
            imm(0x0A, 1, 3, 0),
            lambda state: state.gpr.__setitem__(1, 0x1_0000),
        ),
        ("instruction_bus", 0, lambda state: setattr(state, "pc", 0xFFFF_FFFC)),
    ):
        for remap in (False, True):

            def rtl_exception_setup(state: State, inner=mutate, re=remap):
                state.ie, state.dc_re, state.eba, state.deba = True, re, 0x1000, 0x2000
                if inner:
                    inner(state)

            cases.append(
                make_case(
                    f"rtl_exception_{name}_re{int(remap)}", word, rtl_exception_setup
                )
            )

    # The pinned interrupt block is parameterized over every line and combines
    # asserted, pending, mask, global enable, and exception routing.
    for line in range(32):
        bit = 1 << line
        for remap in (False, True):

            def rtl_interrupt_setup(state: State, mask=bit, re=remap):
                state.ie, state.im, state.lines = True, mask, mask
                state.dc_re, state.eba, state.deba = re, 0x1000, 0x2000

            cases.append(
                make_case(
                    f"rtl_interrupt_latch_{line}_re{int(remap)}",
                    imm(0x0D),
                    rtl_interrupt_setup,
                )
            )
    return cases


def rust_bool(value: bool) -> str:
    return "true" if value else "false"


def rust_opt(value, formatter=lambda item: str(item)) -> str:
    return "None" if value is None else f"Some({formatter(value)})"


def tuple_opt(values: tuple) -> str:
    return f"[{rust_opt(values[0])}, {rust_opt(values[1])}]"


def state_fields(state: State) -> str:
    gpr = ", ".join(f"0x{value:08x}" for value in state.gpr)
    return (
        f"StateFields {{ gpr: [{gpr}], pc: 0x{state.pc:08x}, ie: {rust_bool(state.ie)}, "
        f"eie: {rust_bool(state.eie)}, bie: {rust_bool(state.bie)}, "
        f"dc_re: {rust_bool(state.dc_re)}, im: 0x{state.im:08x}, ip: 0x{state.ip:08x}, "
        f"lines: 0x{state.lines:08x}, eba: 0x{state.eba:08x}, deba: 0x{state.deba:08x} }}"
    )


def memory_diff(before: State, after: State) -> str:
    pairs = [
        (index, value)
        for index, value in enumerate(after.memory)
        if value != before.memory[index]
    ]
    return (
        "&["
        + ", ".join(f"(0x{index:04x}, 0x{value:02x})" for index, value in pairs)
        + "]"
    )


def emit_cases(name: str, cases: list[Case]) -> str:
    rows = [
        "// @generated by tools/generate_vectors.py; do not edit.\n",
        f"const {name}: &[TransitionVector] = &[\n",
    ]
    for case in cases:
        tr = case.trace
        transactions = ", ".join(
            f"ExpectedTransaction {{ kind: {kind}, address: 0x{address:08x}, width: {width}, "
            f"value: 0x{value:08x}, byte_enable: 0x{enable:x}, success: {rust_bool(success)} }}"
            for kind, address, width, value, enable, success in tr["transactions"]
        )
        exception = rust_opt(
            tr["exception"],
            lambda ex: (
                f"ExpectedException {{ id: {ex[0]}, register: {ex[1]}, saved_pc: 0x{ex[2]:08x}, "
                f"dc_re: {rust_bool(ex[3])}, base: 0x{ex[4]:08x}, vector: 0x{ex[5]:08x}, "
                f"ie_before: {rust_bool(ex[6])}, saved_ie: {rust_bool(ex[7])} }}"
            ),
        )
        destination = rust_opt(
            tr["destination"], lambda item: f"({item[0]}, 0x{item[1]:08x})"
        )
        branch = rust_opt(
            tr["branch"], lambda item: f"({rust_bool(item[0])}, 0x{item[1]:08x})"
        )
        word = rust_opt(tr["word"], lambda item: f"0x{item:08x}")
        interrupt = tr["interrupt"]
        rows.append(
            "    TransitionVector {\n"
            f'        name: "{case.name}", word: 0x{case.word:08x},\n'
            f"        initial: {state_fields(case.initial)},\n"
            f"        expected: {state_fields(case.expected)},\n"
            f"        memory_before: {memory_diff(loaded_state(case.word), case.initial)},\n"
            f"        memory_after: {memory_diff(case.initial, case.expected)},\n"
            f"        trace: ExpectedTrace {{ pc_before: 0x{tr['pc_before']:08x}, "
            f'pc_after: 0x{tr["pc_after"]:08x}, word: {word}, mnemonic: "{tr["mnemonic"]}", '
            f"sources: {tuple_opt(tr['sources'])}, values: {tuple_opt(tr['values'])}, "
            f"destination: {destination}, branch: {branch}, transactions: &[{transactions}], "
            f"interrupt: (0x{interrupt[0]:08x}, 0x{interrupt[1]:08x}, "
            f"0x{interrupt[2]:08x}, 0x{interrupt[3]:08x}), exception: {exception} }}\n"
            "    },\n"
        )
    rows.append("];\n")
    return "".join(rows)


def rtl_evidence() -> str:
    rows = ["surface,source,sha256,lines,case_prefix\n"]
    facts = [
        (
            "decoder_opcode_wires",
            "lm32_decoder.v",
            "340-395",
            "rtl_decoder_wire_",
        ),
        (
            "immediate_operand_mux",
            "lm32_decoder.v",
            "428-583",
            "rtl_immediate_mux_",
        ),
        (
            "narrow_store_bus_replication",
            "lm32_load_store_unit.v",
            "506-515",
            "rtl_store_mux_",
        ),
        (
            "store_lane_select",
            "lm32_load_store_unit.v",
            "517-547",
            "rtl_store_mux_",
        ),
        ("interrupt_csr_surface", "lm32_interrupt.v", "180-220", "rtl_csr_"),
        (
            "interrupt_latch_and_enable",
            "lm32_interrupt.v",
            "228-317",
            "rtl_interrupt_latch_",
        ),
        ("exception_control", "lm32_cpu.v", "1680-1815", "rtl_exception_"),
        ("debug_exception_remap", "lm32_debug.v", "288-297", "rtl_exception_"),
    ]
    for surface, source, lines, prefix in facts:
        rows.append(f"{surface},{source},{RTL_HASHES[source]},{lines},{prefix}\n")
    return "".join(rows)


def lifecycle_evidence() -> str:
    actions = [
        ("load_empty", "EmptyProgram"),
        ("load_unaligned", "UnalignedProgram"),
        ("load_too_large", "ProgramTooLarge"),
        ("load_out_of_range", "ProgramOutOfRange"),
        ("restore_short_memory", "InvalidState"),
        ("restore_bad_metadata", "InvalidState"),
        ("restore_bad_base", "InvalidState"),
        ("step_no_program", "NoProgramLoaded"),
        ("step_misaligned_fetch", "MisalignedFetch"),
        ("step_outside", "FetchOutsideProgram"),
        ("step_unknown", "UnknownInstruction"),
        ("step_unsupported", "UnsupportedInstruction"),
        ("step_csr_read", "UnsupportedCsr"),
        ("step_csr_write", "UnsupportedCsr"),
        ("step_data_misaligned", "MisalignedData"),
        ("step_control_misaligned", "MisalignedControlFlow"),
        ("helper_out_of_range", "MemoryOutOfRange"),
        ("invalid_register", "InvalidRegister"),
        ("run_zero", "ZeroStepBudget"),
        ("run_retention", "RetentionLimitExceeded"),
        ("run_exhausted", "StepLimitExceeded"),
        ("execute_empty", "EmptyProgram"),
        ("execute_exhausted", "StepLimitExceeded"),
        ("alloc_snapshot", "AllocationFailed"),
        ("alloc_load", "AllocationFailed"),
        ("alloc_step", "AllocationFailed"),
        ("alloc_run_late", "AllocationFailed"),
        ("alloc_execute_late", "AllocationFailed"),
    ]
    rows = ["id,action,dc_re,expected_error,exact_rollback\n"]
    for action, error in actions:
        for dc_re in (0, 1):
            rows.append(f"{action}_re{dc_re},{action},{dc_re},{error},true\n")
    return "".join(rows)


def verify_rtl(root: Path) -> dict[str, int]:
    rtl = root / "rtl"
    source: dict[str, str] = {}
    for name, expected in RTL_HASHES.items():
        raw = (rtl / name).read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        if digest != expected:
            raise SystemExit(f"{name}: expected {expected}, got {digest}")
        source[name] = raw.decode()

    decoder_pairs = {
        (signal, macro.lower())
        for signal, macro in re.findall(
            r"assign\s+op_(\w+)\s*=.*?`LM32_OPCODE_([A-Z0-9_]+)\s*;",
            source["lm32_decoder.v"],
        )
    }
    expected_pairs = {(name, name) for name in RTL_OPCODE_WIRES}
    if not expected_pairs.issubset(decoder_pairs):
        missing = sorted(expected_pairs - decoder_pairs)
        raise SystemExit(f"lm32_decoder.v: missing opcode-wire evidence: {missing}")

    required_patterns = {
        "lm32_decoder.v": (
            r"assign\s+arith\s*=\s*op_add\s*\|\s*op_sub\s*;",
            r"assign\s+bi_conditional\s*=.*op_be.*op_bg.*op_bge.*op_bgeu.*op_bgu.*op_bne\s*;",
            r"instruction\[31\]\s*==\s*1'b0",
        ),
        "lm32_load_store_unit.v": (
            r"LM32_SIZE_BYTE:\s*store_data_x\s*=\s*\{4\{store_operand_x\[7:0\]\}\}\s*;",
            r"LM32_SIZE_HWORD:\s*store_data_x\s*=\s*\{2\{store_operand_x\[15:0\]\}\}\s*;",
            r"\{`LM32_SIZE_BYTE,\s*2'b11\}:\s*byte_enable_x\s*=\s*4'b0001\s*;",
            r"\{`LM32_SIZE_HWORD,\s*2'b0\?\}:\s*byte_enable_x\s*=\s*4'b1100\s*;",
            r"\{`LM32_SIZE_WORD,\s*2'b\?\?\}:\s*byte_enable_x\s*=\s*4'b1111\s*;",
        ),
        "lm32_interrupt.v": (
            r"assign\s+asserted\s*=\s*ip\s*\|\s*interrupt\s*;",
            r"ip\s*<=\s*asserted\s*&\s*~csr_write_data\[interrupts-1:0\]\s*;",
            r"assign\s+interrupt_exception\s*=\s*\(\|interrupt_n_exception\)\s*&\s*ie\s*;",
        ),
        "lm32_cpu.v": (
            r"assign\s+instruction_bus_error_exception\s*=",
            r"assign\s+data_bus_error_exception\s*=",
            r"assign\s+divide_by_zero_exception\s*=",
            r"assign\s+system_call_exception\s*=",
        ),
        "lm32_debug.v": (r"dc_re\s*<=\s*debug_csr_write_data\[1\]\s*;",),
    }
    for name, patterns in required_patterns.items():
        for pattern in patterns:
            if re.search(pattern, source[name], re.DOTALL) is None:
                raise SystemExit(f"{name}: missing structural evidence /{pattern}/")
    return {
        signal: RTL_OPCODE_WIRES[signal]
        for signal, macro in sorted(decoder_pairs)
        if signal in RTL_OPCODE_WIRES and macro == signal
    }


def write_or_check(path: Path, content: str, check: bool) -> None:
    if check:
        if not path.exists() or path.read_text() != content:
            raise SystemExit(f"stale generated fixture: {path.relative_to(ROOT)}")
    else:
        path.write_text(content)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--rtl-root", type=Path)
    args = parser.parse_args()
    opcode_wires = verify_rtl(args.rtl_root) if args.rtl_root else RTL_OPCODE_WIRES
    manual = manual_corpus()
    rtl = rtl_corpus(opcode_wires)
    write_or_check(
        VECTORS / "manual-transitions.rs",
        emit_cases("MANUAL_TRANSITIONS", manual),
        args.check,
    )
    write_or_check(
        VECTORS / "pinned-rtl-transitions.rs",
        emit_cases("RTL_TRANSITIONS", rtl),
        args.check,
    )
    write_or_check(VECTORS / "pinned-rtl-evidence.csv", rtl_evidence(), args.check)
    write_or_check(VECTORS / "lifecycle-faults.csv", lifecycle_evidence(), args.check)
    print(
        f"manual transitions: {len(manual)}; RTL transitions: {len(rtl)}; "
        f"source facts: 8; lifecycle/fault rows: {len(lifecycle_evidence().splitlines()) - 1}"
    )


if __name__ == "__main__":
    main()
