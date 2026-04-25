"""Protocol intermediate representation for the in-memory data store."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

ResponseKind = Literal["simple_string", "error", "integer", "bulk_string", "array"]


def ascii_upper(data: bytes | bytearray | memoryview) -> str:
    """Return an ASCII-uppercase command name from raw bytes."""

    return bytes(data).upper().decode("ascii")


@dataclass(frozen=True, slots=True)
class CommandFrame:
    command: str
    args: tuple[bytes, ...] = ()

    @classmethod
    def new(cls, command: str, args: list[bytes] | tuple[bytes, ...] = ()) -> CommandFrame:
        return cls(command, tuple(bytes(arg) for arg in args))

    @classmethod
    def from_parts(cls, parts: list[bytes] | tuple[bytes, ...]) -> CommandFrame | None:
        if not parts:
            return None
        command, *args = parts
        return cls(ascii_upper(command), tuple(bytes(arg) for arg in args))

    def to_parts(self) -> list[bytes]:
        return [self.command.encode("ascii"), *self.args]


@dataclass(frozen=True, slots=True)
class EngineResponse:
    kind: ResponseKind
    value: str | int | bytes | tuple[EngineResponse, ...] | None

    @classmethod
    def simple_string(cls, value: str) -> EngineResponse:
        return cls("simple_string", value)

    @classmethod
    def error(cls, value: str) -> EngineResponse:
        return cls("error", value)

    @classmethod
    def integer(cls, value: int) -> EngineResponse:
        return cls("integer", value)

    @classmethod
    def bulk_string(cls, value: bytes | None) -> EngineResponse:
        return cls("bulk_string", None if value is None else bytes(value))

    @classmethod
    def array(cls, value: list[EngineResponse] | tuple[EngineResponse, ...] | None) -> EngineResponse:
        return cls("array", None if value is None else tuple(value))

    @classmethod
    def ok(cls) -> EngineResponse:
        return cls.simple_string("OK")

    @classmethod
    def null(cls) -> EngineResponse:
        return cls.bulk_string(None)

    @classmethod
    def zero(cls) -> EngineResponse:
        return cls.integer(0)

    @classmethod
    def one(cls) -> EngineResponse:
        return cls.integer(1)
