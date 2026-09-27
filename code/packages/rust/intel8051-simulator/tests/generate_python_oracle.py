"""Generate exhaustive deterministic full-state vectors from the Python oracle."""

from __future__ import annotations

from intel8051_simulator import I8051Simulator
from intel8051_simulator.state import (
    SFR_ACC,
    SFR_B,
    SFR_DPH,
    SFR_DPL,
    SFR_P2,
    SFR_PSW,
    SFR_SP,
)

PC = 0x1000


def fnv1a(parts: list[bytes]) -> str:
    value = 0xCBF29CE484222325
    for part in parts:
        for byte in part:
            value ^= byte
            value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def seed(opcode: int) -> I8051Simulator:
    simulator = I8051Simulator()
    simulator._code[:] = bytes((index * 37 + 11) & 0xFF for index in range(65536))
    simulator._xdata[:] = bytes((index * 17 + 3) & 0xFF for index in range(65536))
    simulator._iram[:] = bytes((index * 29 + 7) & 0xFF for index in range(256))
    simulator._pc = PC
    simulator._halted = False
    simulator._code[PC:PC + 3] = bytes((opcode, 0x20, 0x02))
    simulator._iram[0] = 0x40
    simulator._iram[1] = 0x41
    simulator._iram[SFR_SP] = 0x30
    simulator._iram[SFR_DPL] = 0x45
    simulator._iram[SFR_DPH] = 0x23
    simulator._iram[SFR_P2] = 0x12
    simulator._iram[SFR_PSW] = 0xC0
    simulator._iram[SFR_ACC] = 0x35
    simulator._iram[SFR_B] = 0x07
    return simulator


for opcode in range(256):
    simulator = seed(opcode)
    trace = simulator.step()
    state = simulator.get_state()
    digest = fnv1a([
        state.pc.to_bytes(2, "big"),
        bytes((int(state.halted),)),
        bytes(state.iram),
        bytes(state.xdata),
        bytes(state.code),
    ])
    del trace
    print(digest)
