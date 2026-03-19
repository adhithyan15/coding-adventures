"""RegisterFile -- general-purpose register file for the Core.

The register file is the Core's fast, small storage that the pipeline
reads and writes every cycle.

# Why a Custom Register File?

The cpu-simulator package has a RegisterFile, but it uses uint32 values
and panics on out-of-range access. The Core needs a register file that:

  - Uses int values (matching PipelineToken fields)
  - Supports configurable width (32 or 64 bit)
  - Optionally hardwires register 0 to zero (RISC-V convention)
  - Returns 0 instead of raising on out-of-range access

# Zero Register Convention

In RISC-V and MIPS, register x0 (or $zero) is hardwired to the value 0.
Writes to it are silently discarded. This simplifies instruction encoding:

    MOV Rd, Rs  = ADD Rd, Rs, x0   (add zero)
    NOP         = ADD x0, x0, x0   (write nothing to zero register)
    NEG Rd, Rs  = SUB Rd, x0, Rs   (subtract from zero)

ARM does NOT have a zero register (all 31 registers are general-purpose).
x86 does not have one either. The zero_register config controls this.
"""

from __future__ import annotations

from core.config import RegisterFileConfig, default_register_file_config


class RegisterFile:
    """The Core's register file -- fast operand storage.

    Attributes:
        _config: The register file configuration.
        _values: List of register values. _values[0] is R0.
        _mask: Bit mask for the register width.
    """

    def __init__(self, config: RegisterFileConfig | None = None) -> None:
        """Create a new register file from the given configuration.

        All registers are initialized to 0. If config is None, the default
        configuration is used (16 registers, 32-bit, zero register enabled).

        Args:
            config: Register file configuration, or None for defaults.
        """
        cfg = config if config is not None else default_register_file_config()
        self._config = cfg
        self._values: list[int] = [0] * cfg.count

        # Compute the bit mask for the register width.
        # For 32-bit: mask = 0xFFFFFFFF
        # For 64-bit: mask = 0xFFFFFFFFFFFFFFFF
        if cfg.width >= 64:
            self._mask = (1 << 64) - 1
        else:
            self._mask = (1 << cfg.width) - 1

    def read(self, index: int) -> int:
        """Return the value of the register at the given index.

        If the zero register convention is enabled, reading register 0 always
        returns 0, regardless of what was written to it.

        Returns 0 if the index is out of range (defensive -- avoids crashes
        in the pipeline, which processes untrusted instruction data).

        Args:
            index: Register number to read.

        Returns:
            The register value, or 0 if out of range.
        """
        if index < 0 or index >= self._config.count:
            return 0
        if self._config.zero_register and index == 0:
            return 0
        return self._values[index]

    def write(self, index: int, value: int) -> None:
        """Store a value into the register at the given index.

        The value is masked to the register width (e.g., 32-bit mask for
        32-bit registers). Writes to register 0 are silently ignored when
        the zero register convention is enabled.

        Writes to out-of-range indices are silently ignored (defensive).

        Args:
            index: Register number to write.
            value: Value to store.
        """
        if index < 0 or index >= self._config.count:
            return
        if self._config.zero_register and index == 0:
            return  # writes to zero register are discarded
        self._values[index] = value & self._mask

    def values(self) -> list[int]:
        """Return a copy of all register values (for inspection and debugging)."""
        return list(self._values)

    @property
    def count(self) -> int:
        """Return the number of registers."""
        return self._config.count

    @property
    def width(self) -> int:
        """Return the bit width of each register."""
        return self._config.width

    @property
    def config(self) -> RegisterFileConfig:
        """Return the register file configuration."""
        return self._config

    def reset(self) -> None:
        """Set all registers to zero."""
        for i in range(len(self._values)):
            self._values[i] = 0

    def __str__(self) -> str:
        """Return a human-readable dump of all registers.

        Format: RegisterFile(16x32): R1=42 R2=100 ...
        """
        s = f"RegisterFile({self._config.count}x{self._config.width}):"
        for i in range(self._config.count):
            if self._values[i] != 0:
                s += f" R{i}={self._values[i]}"
        return s
