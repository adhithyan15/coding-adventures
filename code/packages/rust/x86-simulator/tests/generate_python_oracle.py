"""Generate deterministic one-step full-state vectors from the Python oracle."""

from __future__ import annotations

from pathlib import Path

from x86_64_simulator import X86_64Simulator


def imm64(value: int) -> bytes:
    return int(value & 0xFFFF_FFFF_FFFF_FFFF).to_bytes(8, "little")


def seeded(sim: X86_64Simulator, instruction: bytes, seed: int) -> None:
    sim.load(instruction + b"\xf4")
    cpu = sim._cpu
    for index in range(16):
        cpu.gpr[index] = (
            ((seed + 1) * 0x9E37_79B9_7F4A_7C15)
            ^ (index * 0xD1B5_4A32_D192_ED03)
        ) & 0xFFFF_FFFF_FFFF_FFFF
    cpu.gpr[0] = [0, 1, 0x8000_0000_0000_0000, 0xFFFF_FFFF_FFFF_FFFF][seed]
    cpu.gpr[1] = [0, 1, 7, 63][seed]
    cpu.gpr[2] = 0
    cpu.gpr[4] = 0xFF00
    cpu.gpr[7] = 0x0200
    cpu.rflags = [0, 1, 1 << 6, (1 << 7) | (1 << 11)][seed]
    for address in range(65_536):
        cpu.memory[address] = (address * 37 + seed * 53 + 11) & 0xFF
    cpu.memory[: len(instruction) + 1] = instruction + b"\xf4"


def hash_state(sim: X86_64Simulator) -> int:
    state = sim.get_state()
    value = 0xCBF2_9CE4_8422_2325

    def feed(data: bytes) -> None:
        nonlocal value
        for byte in data:
            value ^= byte
            value = (value * 0x100_0000_01B3) & 0xFFFF_FFFF_FFFF_FFFF

    feed(state.pc.to_bytes(8, "little"))
    for register in state.gpr:
        feed(register.to_bytes(8, "little"))
    feed(state.rflags.to_bytes(8, "little"))
    feed(bytes(state.memory))
    feed(bytes([state.halted]))
    return value


def main() -> None:
    instructions: list[tuple[str, bytes]] = [
        ("nop", b"\x90"),
        ("hlt", b"\xf4"),
        ("push-rax", b"\x50"),
        ("pop-rbx", b"\x5b"),
        ("push-imm8", b"\x6a\x80"),
        ("push-imm32", b"\x68\x78\x56\x34\x12"),
        ("movabs", b"\x48\xb8" + imm64(0x0123_4567_89AB_CDEF)),
        ("mov-eax", b"\xb8\x78\x56\x34\x12"),
        ("mov-rr", b"\x48\x8b\xc1"),
        ("mov-rm", b"\x48\x89\x07"),
        ("mov-imm-rm", b"\x48\xc7\x07\xff\xff\xff\xff"),
        ("movzx-byte", b"\x48\x0f\xb6\xc1"),
        ("movzx-word", b"\x48\x0f\xb7\xc1"),
        ("movsx-byte", b"\x48\x0f\xbe\xc1"),
        ("movsx-word", b"\x48\x0f\xbf\xc1"),
        ("movsxd", b"\x48\x63\xc1"),
        ("xchg", b"\x48\x87\xc1"),
        ("lea", b"\x48\x8d\x47\x08"),
        ("add-rm-r", b"\x48\x01\xc8"),
        ("add-r-rm", b"\x48\x03\xc1"),
        ("or-rm-r", b"\x48\x09\xc8"),
        ("or-r-rm", b"\x48\x0b\xc1"),
        ("adc-rm-r", b"\x48\x11\xc8"),
        ("adc-r-rm", b"\x48\x13\xc1"),
        ("sbb-rm-r", b"\x48\x19\xc8"),
        ("sbb-r-rm", b"\x48\x1b\xc1"),
        ("and-rm-r", b"\x48\x21\xc8"),
        ("and-r-rm", b"\x48\x23\xc1"),
        ("sub-rm-r", b"\x48\x29\xc8"),
        ("sub-r-rm", b"\x48\x2b\xc1"),
        ("xor-rm-r", b"\x48\x31\xc8"),
        ("xor-r-rm", b"\x48\x33\xc1"),
        ("cmp-rm-r", b"\x48\x39\xc8"),
        ("cmp-r-rm", b"\x48\x3b\xc1"),
        ("test", b"\x48\x85\xc8"),
        ("alu-add-imm", b"\x48\x83\xc0\x7f"),
        ("alu-sub-imm", b"\x48\x81\xe8\x78\x56\x34\x12"),
        ("rol", b"\x48\xc1\xc0\x01"),
        ("ror", b"\x48\xc1\xc8\x01"),
        ("shl", b"\x48\xc1\xe0\x01"),
        ("shr", b"\x48\xc1\xe8\x01"),
        ("sar", b"\x48\xc1\xf8\x01"),
        ("shl-cl", b"\x48\xd3\xe0"),
        ("imul-two", b"\x48\x0f\xaf\xc1"),
        ("imul-three", b"\x48\x6b\xc1\xfb"),
        ("mul", b"\x48\xf7\xe1"),
        ("not", b"\x48\xf7\xd0"),
        ("neg", b"\x48\xf7\xd8"),
        ("inc", b"\x48\xff\xc0"),
        ("dec", b"\x48\xff\xc8"),
        ("cmove", b"\x48\x0f\x44\xc1"),
        ("cmovne", b"\x48\x0f\x45\xc1"),
        ("setcc", b"\x0f\x94\xc0"),
        ("bsf", b"\x48\x0f\xbc\xc1"),
        ("bsr", b"\x48\x0f\xbd\xc1"),
        ("bt", b"\x48\x0f\xa3\xc8"),
        ("bswap", b"\x48\x0f\xc8"),
        ("rep-stosq", b"\xf3\x48\xab"),
        ("rep-stosd", b"\xf3\xab"),
        ("jcc-short", b"\x74\x00"),
        ("jcc-near", b"\x0f\x85\x00\x00\x00\x00"),
        ("loop", b"\xe2\x00"),
        ("loope", b"\xe1\x00"),
        ("loopne", b"\xe0\x00"),
        ("jrcxz", b"\xe3\x00"),
    ]

    lines = ["# name|seed|instruction|outcome|hash"]
    for name, instruction in instructions:
        for seed in range(4):
            sim = X86_64Simulator()
            seeded(sim, instruction, seed)
            sim.step()
            lines.append(
                f"{name}|{seed}|{instruction.hex()}|OK|{hash_state(sim):016x}"
            )

    for name, instruction in [("unknown", b"\x0f\x01"), ("truncated", b"\x48")]:
        lines.append(f"{name}|0|{instruction.hex()}|ERROR|0000000000000000")

    destination = Path(__file__).with_name("python_oracle_hashes.txt")
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} vectors to {destination}")


if __name__ == "__main__":
    main()
