"""GE-225 simulator oriented around documented instruction families.

The simulator uses the historical GE-225 mnemonic groups and octal encodings
from the programming manuals. It intentionally stops short of emulating every
peripheral subsystem, but it now models the central-processor instruction
repertoire as real GE-225 operations rather than a private backend subset.
"""

from __future__ import annotations

from dataclasses import dataclass

from ge225_simulator.state import GE225Indicators, GE225State

MASK_20 = (1 << 20) - 1
MASK_40 = (1 << 40) - 1
DATA_MASK = (1 << 19) - 1
SIGN_BIT = 1 << 19
ADDR_MASK = 0x1FFF
X_MASK = 0x7FFF
N_MASK = 0x3F
WORD_BYTES = 3
MAX_X_GROUPS = 32


# ---------------------------------------------------------------------------
# Base memory-reference opcodes (5-bit top field, expressed in octal docs)
# ---------------------------------------------------------------------------

OP_LDA = 0o00
OP_ADD = 0o01
OP_SUB = 0o02
OP_STA = 0o03
OP_BXL = 0o04
OP_BXH = 0o05
OP_LDX = 0o06
OP_SPB = 0o07
OP_DLD = 0o10
OP_DAD = 0o11
OP_DSU = 0o12
OP_DST = 0o13
OP_INX = 0o14
OP_MPY = 0o15
OP_DVD = 0o16
OP_STX = 0o17
OP_EXT = 0o20
OP_CAB = 0o21
OP_DCB = 0o22
OP_ORY = 0o23
OP_MOY = 0o24
OP_RCD = 0o25
OP_BRU = 0o26
OP_STO = 0o27

BASE_OPCODE_NAMES = {
    OP_LDA: "LDA",
    OP_ADD: "ADD",
    OP_SUB: "SUB",
    OP_STA: "STA",
    OP_BXL: "BXL",
    OP_BXH: "BXH",
    OP_LDX: "LDX",
    OP_SPB: "SPB",
    OP_DLD: "DLD",
    OP_DAD: "DAD",
    OP_DSU: "DSU",
    OP_DST: "DST",
    OP_INX: "INX",
    OP_MPY: "MPY",
    OP_DVD: "DVD",
    OP_STX: "STX",
    OP_EXT: "EXT",
    OP_CAB: "CAB",
    OP_DCB: "DCB",
    OP_ORY: "ORY",
    OP_MOY: "MOY",
    OP_RCD: "RCD",
    OP_BRU: "BRU",
    OP_STO: "STO",
}

NON_MODIFYING_MEMORY_REFERENCE = {"BXL", "BXH", "LDX", "SPB", "INX", "STX", "MOY"}


# ---------------------------------------------------------------------------
# Special fixed-word instructions.
# The values are the documented full 20-bit words interpreted from octal tables.
# ---------------------------------------------------------------------------

FIXED_WORDS = {
    "OFF": int("2500005", 8),
    "TYP": int("2500006", 8),
    "TON": int("2500007", 8),
    "RCS": int("2500011", 8),
    "HPT": int("2500016", 8),
    "LDZ": int("2504002", 8),
    "LDO": int("2504022", 8),
    "LMO": int("2504102", 8),
    "CPL": int("2504502", 8),
    "NEG": int("2504522", 8),
    "CHS": int("2504040", 8),
    "NOP": int("2504012", 8),
    "LAQ": int("2504001", 8),
    "LQA": int("2504004", 8),
    "XAQ": int("2504005", 8),
    "MAQ": int("2504006", 8),
    "ADO": int("2504032", 8),
    "SBO": int("2504112", 8),
    "SET_DECMODE": int("2506011", 8),
    "SET_BINMODE": int("2506012", 8),
    "SXG": int("2506013", 8),
    "SET_PST": int("2506015", 8),
    "SET_PBK": int("2506016", 8),
    "BOD": int("2514000", 8),
    "BEV": int("2516000", 8),
    "BMI": int("2514001", 8),
    "BPL": int("2516001", 8),
    "BZE": int("2514002", 8),
    "BNZ": int("2516002", 8),
    "BOV": int("2514003", 8),
    "BNO": int("2516003", 8),
    "BPE": int("2514004", 8),
    "BPC": int("2516004", 8),
    "BNR": int("2514005", 8),
    "BNN": int("2516005", 8),
}

FIXED_NAMES = {word: mnemonic for mnemonic, word in FIXED_WORDS.items()}


# ---------------------------------------------------------------------------
# Shift/normalize patterns, also documented as octal words with K in low bits.
# ---------------------------------------------------------------------------

SHIFT_BASES = {
    "SRA": int("2510000", 8),
    "SNA": int("2510100", 8),
    "SCA": int("2510040", 8),
    "SAN": int("2510400", 8),
    "SRD": int("2511000", 8),
    "NAQ": int("2511100", 8),
    "SCD": int("2511200", 8),
    "ANQ": int("2511400", 8),
    "SLA": int("2512000", 8),
    "SLD": int("2512200", 8),
    "NOR": int("2513000", 8),
    "DNO": int("2513200", 8),
}


TYPEWRITER_CODES = {
    0o00: "0",
    0o01: "1",
    0o02: "2",
    0o03: "3",
    0o04: "4",
    0o05: "5",
    0o06: "6",
    0o07: "7",
    0o10: "8",
    0o11: "9",
    0o13: "/",
    0o21: "A",
    0o22: "B",
    0o23: "C",
    0o24: "D",
    0o25: "E",
    0o26: "F",
    0o27: "G",
    0o30: "H",
    0o31: "I",
    0o33: "-",
    0o40: ".",
    0o41: "J",
    0o42: "K",
    0o43: "L",
    0o44: "M",
    0o45: "N",
    0o46: "O",
    0o47: "P",
    0o50: "Q",
    0o51: "R",
    0o53: "$",
    0o60: " ",
    0o62: "S",
    0o63: "T",
    0o64: "U",
    0o65: "V",
    0o66: "W",
    0o67: "X",
    0o70: "Y",
    0o71: "Z",
}


def _to_signed20(value: int) -> int:
    value &= MASK_20
    return value - (1 << 20) if value & SIGN_BIT else value


def _from_signed20(value: int) -> int:
    return value & MASK_20


def _to_signed40(value: int) -> int:
    value &= MASK_40
    return value - (1 << 40) if value & (1 << 39) else value


def _from_signed40(value: int) -> int:
    return value & MASK_40


def _split_signed40(value: int) -> tuple[int, int]:
    raw = _from_signed40(value)
    return (raw >> 20) & MASK_20, raw & MASK_20


def _combine_words(high: int, low: int) -> int:
    return ((high & MASK_20) << 20) | (low & MASK_20)


def _sign_of(word: int) -> int:
    return 1 if word & SIGN_BIT else 0


def _with_sign(word: int, sign: int) -> int:
    return ((sign & 1) << 19) | (word & DATA_MASK)


def _arith_compare(left: int, right: int) -> int:
    l_signed = _to_signed20(left)
    r_signed = _to_signed20(right)
    if l_signed < r_signed:
        return -1
    if l_signed > r_signed:
        return 1
    return 0


def _arith_compare_double(left_high: int, left_low: int, right_high: int, right_low: int) -> int:
    left = _to_signed40(_combine_words(left_high, left_low))
    right = _to_signed40(_combine_words(right_high, right_low))
    if left < right:
        return -1
    if left > right:
        return 1
    return 0


def encode_instruction(opcode: int, modifier: int, address: int) -> int:
    """Encode a base memory-reference instruction."""
    if not 0 <= opcode <= 0o37:
        raise ValueError(f"opcode out of range: {opcode}")
    if not 0 <= modifier <= 0o3:
        raise ValueError(f"modifier out of range: {modifier}")
    if not 0 <= address <= ADDR_MASK:
        raise ValueError(f"address out of range: {address}")
    return ((opcode & 0x1F) << 15) | ((modifier & 0x03) << 13) | (address & ADDR_MASK)


def decode_instruction(word: int) -> tuple[int, int, int]:
    """Decode a base memory-reference instruction word."""
    word &= MASK_20
    return ((word >> 15) & 0x1F, (word >> 13) & 0x03, word & ADDR_MASK)


def assemble_fixed(mnemonic: str) -> int:
    """Assemble a documented fixed-word instruction by mnemonic."""
    try:
        return FIXED_WORDS[mnemonic]
    except KeyError as exc:
        raise ValueError(f"unknown fixed GE-225 instruction: {mnemonic}") from exc


def assemble_shift(mnemonic: str, count: int) -> int:
    """Assemble a documented shift/normalize instruction with count K."""
    if not 0 <= count <= 0o37:
        raise ValueError(f"shift count out of range: {count}")
    try:
        return SHIFT_BASES[mnemonic] | count
    except KeyError as exc:
        raise ValueError(f"unknown GE-225 shift instruction: {mnemonic}") from exc


def pack_words(words: list[int]) -> bytes:
    """Pack 20-bit words into 3-byte big-endian containers."""
    blob = bytearray()
    for word in words:
        blob.extend((word & MASK_20).to_bytes(WORD_BYTES, byteorder="big"))
    return bytes(blob)


def unpack_words(program: bytes) -> list[int]:
    """Unpack bytes into 20-bit words using 3 bytes per word."""
    if len(program) % WORD_BYTES != 0:
        raise ValueError(
            f"GE-225 byte stream must be a multiple of {WORD_BYTES} bytes, got {len(program)}"
        )
    return [
        int.from_bytes(program[i : i + WORD_BYTES], "big") & MASK_20
        for i in range(0, len(program), WORD_BYTES)
    ]


@dataclass
class GE225Trace:
    """Record of one executed GE-225 instruction."""

    address: int
    instruction_word: int
    mnemonic: str
    a_before: int
    a_after: int
    q_before: int
    q_after: int
    effective_address: int | None = None


@dataclass(frozen=True)
class DecodedInstruction:
    """Decoded GE-225 instruction."""

    mnemonic: str
    opcode: int | None
    modifier: int | None
    address: int | None
    count: int | None = None
    fixed_word: bool = False


class GE225Simulator:
    """Behavioral GE-225 simulator using documented instruction names/opcodes."""

    def __init__(self, memory_words: int = 4096) -> None:
        if memory_words <= 0:
            raise ValueError("memory_words must be positive")
        self._memory_size = memory_words
        self._memory: list[int] = [0] * memory_words
        self._card_reader_queue: list[list[int]] = []
        self.reset()

    @property
    def state(self) -> GE225State:
        return self.get_state()

    def reset(self) -> None:
        self._a = 0
        self._q = 0
        self._m = 0
        self._n = 0
        self._pc = 0
        self._ir = 0
        self._overflow = False
        self._parity_error = False
        self._decimal_mode = False
        self._automatic_interrupt_mode = False
        self._selected_x_group = 0
        self._n_ready = True
        self._typewriter_power = False
        self._typewriter_output: list[str] = []
        self._control_switches = 0
        self._halted = False
        self._x_groups = [[0, 0, 0, 0] for _ in range(MAX_X_GROUPS)]

    def get_state(self) -> GE225State:
        return GE225State(
            a=self._a,
            q=self._q,
            m=self._m,
            n=self._n,
            pc=self._pc,
            ir=self._ir,
            indicators=GE225Indicators(
                carry=self._overflow,
                zero=self._a == 0,
                negative=bool(self._a & SIGN_BIT),
                overflow=self._overflow,
                parity_error=self._parity_error,
            ),
            overflow=self._overflow,
            parity_error=self._parity_error,
            decimal_mode=self._decimal_mode,
            automatic_interrupt_mode=self._automatic_interrupt_mode,
            selected_x_group=self._selected_x_group,
            n_ready=self._n_ready,
            typewriter_power=self._typewriter_power,
            control_switches=self._control_switches,
            x_words=tuple(self._x_groups[self._selected_x_group]),
            halted=self._halted,
            memory=tuple(self._memory),
        )

    def set_control_switches(self, value: int) -> None:
        self._control_switches = value & MASK_20

    def queue_card_reader_record(self, words: list[int]) -> None:
        self._card_reader_queue.append([word & MASK_20 for word in words])

    def get_typewriter_output(self) -> str:
        return "".join(self._typewriter_output)

    def load_words(self, words: list[int], start_address: int = 0) -> None:
        for offset, word in enumerate(words):
            self.write_word(start_address + offset, word)

    def load_program_bytes(self, program: bytes, start_address: int = 0) -> None:
        self.load_words(unpack_words(program), start_address=start_address)

    def read_word(self, address: int) -> int:
        self._check_address(address)
        return self._memory[address]

    def write_word(self, address: int, value: int) -> None:
        self._check_address(address)
        self._memory[address] = value & MASK_20

    def disassemble_word(self, word: int) -> str:
        decoded = self._decode_word(word)
        if decoded.fixed_word:
            return decoded.mnemonic if decoded.count is None else f"{decoded.mnemonic} {decoded.count}"
        return f"{decoded.mnemonic} 0x{decoded.address:03X},X{decoded.modifier}"

    def step(self) -> GE225Trace:
        if self._halted:
            raise RuntimeError("cannot step a halted GE-225 simulator")

        pc_before = self._pc
        self._ir = self.read_word(self._pc)
        self._pc = (self._pc + 1) % self._memory_size

        decoded = self._decode_word(self._ir)
        a_before = self._a
        q_before = self._q
        effective_address: int | None = None

        if not decoded.fixed_word:
            assert decoded.modifier is not None
            assert decoded.address is not None
            address = decoded.address
            if decoded.mnemonic not in NON_MODIFYING_MEMORY_REFERENCE:
                effective_address = self._resolve_effective_address(address, decoded.modifier)
            self._execute_memory_reference(
                decoded.mnemonic,
                decoded.modifier,
                address if effective_address is None else effective_address,
                address,
                pc_before,
            )
        else:
            self._execute_fixed(decoded)

        return GE225Trace(
            address=pc_before,
            instruction_word=self._ir,
            mnemonic=self.disassemble_word(self._ir),
            a_before=a_before,
            a_after=self._a,
            q_before=q_before,
            q_after=self._q,
            effective_address=effective_address,
        )

    def run(self, max_steps: int = 100_000) -> list[GE225Trace]:
        traces: list[GE225Trace] = []
        steps = 0
        while not self._halted and steps < max_steps:
            traces.append(self.step())
            steps += 1
        return traces

    def execute(self, program: bytes, max_steps: int = 100_000):
        from simulator_protocol import ExecutionResult, StepTrace

        self.reset()
        self.load_program_bytes(program)

        traces: list[StepTrace] = []
        steps = 0
        while not self._halted and steps < max_steps:
            pc_before = self._pc
            trace = self.step()
            traces.append(
                StepTrace(
                    pc_before=pc_before,
                    pc_after=self._pc,
                    mnemonic=trace.mnemonic,
                    description=f"{trace.mnemonic} @ 0x{pc_before:03X}",
                )
            )
            steps += 1

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=None if self._halted else f"max_steps ({max_steps}) exceeded",
            traces=traces,
        )

    def _get_x_word(self, slot: int) -> int:
        return self._x_groups[self._selected_x_group][slot] & X_MASK

    def _set_x_word(self, slot: int, value: int) -> None:
        self._x_groups[self._selected_x_group][slot] = value & X_MASK

    def _execute_memory_reference(
        self,
        mnemonic: str,
        modifier: int,
        effective_or_raw_address: int,
        raw_address: int,
        pc_before: int,
    ) -> None:
        effective_address = effective_or_raw_address % self._memory_size

        if mnemonic == "LDA":
            self._m = self.read_word(effective_address)
            self._a = self._m
        elif mnemonic == "ADD":
            self._m = self.read_word(effective_address)
            total = _to_signed20(self._a) + _to_signed20(self._m)
            self._a = _from_signed20(total)
            self._overflow = not (-(1 << 19) <= total <= ((1 << 19) - 1))
        elif mnemonic == "SUB":
            self._m = self.read_word(effective_address)
            total = _to_signed20(self._a) - _to_signed20(self._m)
            self._a = _from_signed20(total)
            self._overflow = not (-(1 << 19) <= total <= ((1 << 19) - 1))
        elif mnemonic == "STA":
            self.write_word(effective_address, self._a)
        elif mnemonic == "BXL":
            if (self._get_x_word(modifier) & ADDR_MASK) >= raw_address:
                self._pc = (self._pc + 1) % self._memory_size
        elif mnemonic == "BXH":
            if (self._get_x_word(modifier) & ADDR_MASK) < raw_address:
                self._pc = (self._pc + 1) % self._memory_size
        elif mnemonic == "LDX":
            self._set_x_word(modifier, self.read_word(raw_address % self._memory_size))
        elif mnemonic == "SPB":
            self._set_x_word(modifier, pc_before)
            self._pc = raw_address % self._memory_size
        elif mnemonic == "DLD":
            first = self.read_word(effective_address)
            if effective_address & 1:
                self._a = first
                self._q = first
            else:
                self._a = first
                self._q = self.read_word((effective_address + 1) % self._memory_size)
        elif mnemonic == "DAD":
            left = _to_signed40(_combine_words(self._a, self._q))
            first = self.read_word(effective_address)
            second = first if effective_address & 1 else self.read_word((effective_address + 1) % self._memory_size)
            right = _to_signed40(_combine_words(first, second))
            total = left + right
            self._a, self._q = _split_signed40(total)
            self._overflow = not (-(1 << 39) <= total <= ((1 << 39) - 1))
        elif mnemonic == "DSU":
            left = _to_signed40(_combine_words(self._a, self._q))
            first = self.read_word(effective_address)
            second = first if effective_address & 1 else self.read_word((effective_address + 1) % self._memory_size)
            right = _to_signed40(_combine_words(first, second))
            total = left - right
            self._a, self._q = _split_signed40(total)
            self._overflow = not (-(1 << 39) <= total <= ((1 << 39) - 1))
        elif mnemonic == "DST":
            if effective_address & 1:
                self.write_word(effective_address, self._q)
            else:
                self.write_word(effective_address, self._a)
                self.write_word((effective_address + 1) % self._memory_size, self._q)
        elif mnemonic == "INX":
            total = (self._get_x_word(modifier) + raw_address) & X_MASK
            self._set_x_word(modifier, total)
        elif mnemonic == "MPY":
            self._m = self.read_word(effective_address)
            product = (_to_signed20(self._q) * _to_signed20(self._m)) + _to_signed20(self._a)
            self._a, self._q = _split_signed40(product)
            self._overflow = not (-(1 << 39) <= product <= ((1 << 39) - 1))
        elif mnemonic == "DVD":
            self._m = self.read_word(effective_address)
            divisor = _to_signed20(self._m)
            if divisor == 0:
                raise ZeroDivisionError("GE-225 divide by zero")
            if abs(divisor) <= abs(_to_signed20(self._a)):
                self._overflow = True
                return
            dividend = _to_signed40(_combine_words(self._a, self._q))
            quotient_mag, remainder_mag = divmod(abs(dividend), abs(divisor))
            quotient_sign = -1 if (dividend < 0) ^ (divisor < 0) else 1
            quotient = quotient_mag * quotient_sign
            remainder = remainder_mag * quotient_sign
            self._a = _from_signed20(quotient)
            self._q = _from_signed20(remainder)
            self._overflow = not (-(1 << 19) <= quotient <= ((1 << 19) - 1))
        elif mnemonic == "STX":
            self.write_word(raw_address % self._memory_size, self._get_x_word(modifier))
        elif mnemonic == "EXT":
            self._m = self.read_word(effective_address)
            self._a &= ~self._m & MASK_20
        elif mnemonic == "CAB":
            self._m = self.read_word(effective_address)
            relation = _arith_compare(self._m, self._a)
            if relation == 0:
                self._pc = (self._pc + 1) % self._memory_size
            elif relation < 0:
                self._pc = (self._pc + 2) % self._memory_size
        elif mnemonic == "DCB":
            first = self.read_word(effective_address)
            second = first if effective_address & 1 else self.read_word((effective_address + 1) % self._memory_size)
            relation = _arith_compare_double(first, second, self._a, self._q)
            if relation == 0:
                self._pc = (self._pc + 1) % self._memory_size
            elif relation < 0:
                self._pc = (self._pc + 2) % self._memory_size
        elif mnemonic == "ORY":
            self.write_word(effective_address, self.read_word(effective_address) | self._a)
        elif mnemonic == "MOY":
            word_count = max(0, -_to_signed20(self._q))
            destination = self._a & X_MASK
            moved = [self.read_word((raw_address + offset) % self._memory_size) for offset in range(word_count)]
            for offset, word in enumerate(moved):
                self.write_word((destination + offset) % self._memory_size, word)
            self._set_x_word(0, self._pc)
            self._a = 0
        elif mnemonic == "RCD":
            if not self._card_reader_queue:
                raise RuntimeError("RCD executed with no queued card-reader record")
            record = self._card_reader_queue.pop(0)
            for offset, word in enumerate(record):
                self.write_word((effective_address + offset) % self._memory_size, word)
        elif mnemonic == "BRU":
            self._pc = effective_address
        elif mnemonic == "STO":
            existing = self.read_word(effective_address)
            self.write_word(effective_address, (existing & ~ADDR_MASK) | (self._a & ADDR_MASK))
        else:
            raise NotImplementedError(f"unimplemented GE-225 memory-reference instruction: {mnemonic}")

    def _execute_fixed(self, decoded: DecodedInstruction) -> None:
        mnemonic = decoded.mnemonic
        count = decoded.count

        if mnemonic == "OFF":
            self._typewriter_power = False
            self._n_ready = True
        elif mnemonic == "TYP":
            if not self._typewriter_power:
                self._n_ready = False
                return
            code = self._n & N_MASK
            if code == 0o37:
                self._typewriter_output.append("\r")
                self._n_ready = True
            elif code == 0o72 or code == 0o75:
                self._n_ready = True
            elif code == 0o76:
                self._typewriter_output.append("\t")
                self._n_ready = True
            else:
                char = TYPEWRITER_CODES.get(code)
                if char is None:
                    self._n_ready = False
                else:
                    self._typewriter_output.append(char)
                    self._n_ready = True
        elif mnemonic == "TON":
            self._typewriter_power = True
        elif mnemonic == "RCS":
            self._a |= self._control_switches
        elif mnemonic == "HPT":
            self._n_ready = False
        elif mnemonic == "LDZ":
            self._a = 0
        elif mnemonic == "LDO":
            self._a = 1
        elif mnemonic == "LMO":
            self._a = MASK_20
        elif mnemonic == "CPL":
            self._a = (~self._a) & MASK_20
        elif mnemonic == "NEG":
            before = _to_signed20(self._a)
            self._a = _from_signed20(-before)
            self._overflow = before == -(1 << 19)
        elif mnemonic == "CHS":
            self._a ^= SIGN_BIT
        elif mnemonic == "NOP":
            pass
        elif mnemonic == "LAQ":
            self._a = self._q
        elif mnemonic == "LQA":
            self._q = self._a
        elif mnemonic == "XAQ":
            self._a, self._q = self._q, self._a
        elif mnemonic == "MAQ":
            self._q = self._a
            self._a = 0
        elif mnemonic == "ADO":
            total = _to_signed20(self._a) + 1
            self._a = _from_signed20(total)
            self._overflow = not (-(1 << 19) <= total <= ((1 << 19) - 1))
        elif mnemonic == "SBO":
            total = _to_signed20(self._a) - 1
            self._a = _from_signed20(total)
            self._overflow = not (-(1 << 19) <= total <= ((1 << 19) - 1))
        elif mnemonic == "SET_DECMODE":
            self._decimal_mode = True
        elif mnemonic == "SET_BINMODE":
            self._decimal_mode = False
        elif mnemonic == "SXG":
            self._selected_x_group = self._a & 0x1F
        elif mnemonic == "SET_PST":
            self._automatic_interrupt_mode = True
        elif mnemonic == "SET_PBK":
            self._automatic_interrupt_mode = False
        elif mnemonic in {
            "BOD",
            "BEV",
            "BMI",
            "BPL",
            "BZE",
            "BNZ",
            "BOV",
            "BNO",
            "BPE",
            "BPC",
            "BNR",
            "BNN",
        }:
            self._execute_branch_test(mnemonic)
        elif mnemonic in SHIFT_BASES:
            assert count is not None
            self._execute_shift(mnemonic, count)
        else:
            raise NotImplementedError(f"unimplemented GE-225 fixed instruction: {mnemonic}")

    def _execute_branch_test(self, mnemonic: str) -> None:
        cond = {
            "BOD": bool(self._a & 1),
            "BEV": not bool(self._a & 1),
            "BMI": bool(self._a & SIGN_BIT),
            "BPL": not bool(self._a & SIGN_BIT),
            "BZE": self._a == 0,
            "BNZ": self._a != 0,
            "BOV": self._overflow,
            "BNO": not self._overflow,
            "BPE": self._parity_error,
            "BPC": not self._parity_error,
            "BNR": self._n_ready,
            "BNN": not self._n_ready,
        }[mnemonic]
        if mnemonic in {"BOV", "BNO"}:
            self._overflow = False
        if mnemonic in {"BPE", "BPC"}:
            self._parity_error = False
        if cond:
            self._pc = (self._pc + 1) % self._memory_size

    def _execute_shift(self, mnemonic: str, count: int) -> None:
        if count == 0:
            if mnemonic == "SRD":
                self._q = _with_sign(self._q, _sign_of(self._a))
            elif mnemonic == "SLD":
                self._a = _with_sign(self._a, _sign_of(self._q))
            return

        a_sign = _sign_of(self._a)
        a_data = self._a & DATA_MASK
        q_sign = _sign_of(self._q)
        q_data = self._q & DATA_MASK

        if mnemonic == "SRA":
            shifted = _to_signed20(self._a) >> min(count, 19)
            self._a = _from_signed20(shifted)
        elif mnemonic == "SLA":
            shifted_out = a_data >> max(0, 19 - count)
            self._overflow = shifted_out != 0
            self._a = _with_sign((a_data << count) & DATA_MASK, a_sign)
        elif mnemonic == "SCA":
            rotation = count % 19
            if rotation:
                a_data = ((a_data >> rotation) | (a_data << (19 - rotation))) & DATA_MASK
            self._a = _with_sign(a_data, a_sign)
        elif mnemonic == "SAN":
            fill = ((1 << count) - 1) if a_sign else 0
            combined = ((a_data & DATA_MASK) << 6) | (self._n & N_MASK)
            combined = ((fill << 25) | combined) >> count
            self._a = _with_sign((combined >> 6) & DATA_MASK, a_sign)
            self._n = combined & N_MASK
        elif mnemonic == "SNA":
            combined = ((self._n & N_MASK) << 19) | a_data
            combined >>= count
            self._n = (combined >> 19) & N_MASK
            self._a = _with_sign(combined & DATA_MASK, a_sign)
        elif mnemonic == "SRD":
            value = (_combine_words(self._a, self._q) >> count) & MASK_40
            self._a = _with_sign((value >> 20) & DATA_MASK, a_sign)
            self._q = _with_sign(value & DATA_MASK, a_sign)
        elif mnemonic == "NAQ":
            combined = ((self._n & N_MASK) << 38) | ((a_data & DATA_MASK) << 19) | q_data
            combined >>= count
            self._n = (combined >> 38) & N_MASK
            self._a = _with_sign((combined >> 19) & DATA_MASK, a_sign)
            self._q = _with_sign(combined & DATA_MASK, a_sign)
        elif mnemonic == "SCD":
            rotation = count % 38
            combined = ((a_data & DATA_MASK) << 19) | q_data
            if rotation:
                combined = ((combined >> rotation) | (combined << (38 - rotation))) & ((1 << 38) - 1)
            self._a = _with_sign((combined >> 19) & DATA_MASK, a_sign)
            self._q = _with_sign(combined & DATA_MASK, a_sign)
        elif mnemonic == "ANQ":
            for _ in range(count):
                bit = self._a & 1
                self._a = _from_signed20(_to_signed20(self._a) >> 1)
                q_data = ((bit << 18) | (self._q & DATA_MASK) >> 1) & DATA_MASK
                self._q = _with_sign(q_data, a_sign)
                self._n = ((bit << 5) | (self._n >> 1)) & N_MASK
        elif mnemonic == "SLD":
            combined = ((a_data & DATA_MASK) << 19) | q_data
            shifted_out = combined >> max(0, 38 - count)
            self._overflow = shifted_out != 0
            combined = (combined << count) & ((1 << 38) - 1)
            self._a = _with_sign((combined >> 19) & DATA_MASK, q_sign)
            self._q = _with_sign(combined & DATA_MASK, q_sign)
        elif mnemonic == "NOR":
            shifts = 0
            target_bit = 0 if a_sign == 0 else 1
            while shifts < count:
                lead = (a_data >> 18) & 1
                if lead != target_bit:
                    break
                shifted_out = (a_data >> 18) & 1
                self._overflow = self._overflow or (shifted_out == 1)
                a_data = (a_data << 1) & DATA_MASK
                shifts += 1
            self._a = _with_sign(a_data, a_sign)
            self._set_x_word(0, count - shifts)
        elif mnemonic == "DNO":
            shifts = 0
            target_bit = 0 if a_sign == 0 else 1
            combined = ((a_data & DATA_MASK) << 19) | q_data
            while shifts < count:
                lead = (combined >> 37) & 1
                if lead != target_bit:
                    break
                self._overflow = self._overflow or (lead == 1)
                combined = (combined << 1) & ((1 << 38) - 1)
                shifts += 1
            self._a = _with_sign((combined >> 19) & DATA_MASK, q_sign)
            self._q = _with_sign(combined & DATA_MASK, q_sign)
            self._set_x_word(0, count - shifts)
        else:
            raise NotImplementedError(f"shift instruction not implemented: {mnemonic}")

    def _decode_word(self, word: int) -> DecodedInstruction:
        word &= MASK_20

        if word in FIXED_NAMES:
            return DecodedInstruction(
                mnemonic=FIXED_NAMES[word],
                opcode=None,
                modifier=None,
                address=None,
                fixed_word=True,
            )

        for mnemonic, base in SHIFT_BASES.items():
            if (word & ~0o37) == base:
                return DecodedInstruction(
                    mnemonic=mnemonic,
                    opcode=None,
                    modifier=None,
                    address=None,
                    count=word & 0o37,
                    fixed_word=True,
                )

        opcode, modifier, address = decode_instruction(word)
        mnemonic = BASE_OPCODE_NAMES.get(opcode)
        if mnemonic is None:
            raise ValueError(f"unknown GE-225 opcode field {opcode:o}")
        return DecodedInstruction(
            mnemonic=mnemonic,
            opcode=opcode,
            modifier=modifier,
            address=address,
            fixed_word=False,
        )

    def _resolve_effective_address(self, address: int, modifier: int) -> int:
        base = address % self._memory_size
        if modifier == 0:
            return base
        return (base + (self._get_x_word(modifier) % self._memory_size)) % self._memory_size

    def _check_address(self, address: int) -> None:
        if not 0 <= address < self._memory_size:
            raise ValueError(f"address out of range: {address}")
