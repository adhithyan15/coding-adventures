"""Python sugar over Rust-owned Board VM protocol frames."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable

from . import board_vm_native as _native


DEFAULT_HOST_NAME = "python-board-vm"
DEFAULT_HOST_NONCE = 0xB0A2D001
DEFAULT_PROGRAM_ID = 1
DEFAULT_INSTRUCTION_BUDGET = 12
GPIO_READ_MODES = {
    "input": 0,
    "input_pullup": 2,
    "pullup": 2,
    "input_pulldown": 3,
    "pulldown": 3,
}


@dataclass(frozen=True)
class Capability:
    raw: dict[str, Any]

    @property
    def id(self) -> int:
        return int(self.raw["id"])

    @property
    def version(self) -> int:
        return int(self.raw["version"])

    @property
    def flags(self) -> int:
        return int(self.raw["flags"])

    @property
    def name(self) -> str:
        return str(self.raw["name"])

    @property
    def bytecode_callable(self) -> bool:
        return bool(self.raw.get("bytecode_callable", self.flags & 0x0001 != 0))

    @property
    def protocol_feature(self) -> bool:
        return bool(self.raw.get("protocol_feature", self.flags & 0x0002 != 0))

    @property
    def board_metadata(self) -> bool:
        return bool(self.raw.get("board_metadata", self.flags & 0x0004 != 0))

    @property
    def flag_names(self) -> list[str]:
        names = self.raw.get("flag_names")
        if names is not None:
            return [str(name) for name in names]
        result = []
        if self.bytecode_callable:
            result.append("bytecode_callable")
        if self.protocol_feature:
            result.append("protocol_feature")
        if self.board_metadata:
            result.append("board_metadata")
        return result


@dataclass(frozen=True)
class BoardDescriptor:
    raw: dict[str, Any]

    @property
    def board_id(self) -> str:
        return str(self.raw["board_id"])

    @property
    def runtime_id(self) -> str:
        return str(self.raw["runtime_id"])

    @property
    def max_program_bytes(self) -> int:
        return int(self.raw["max_program_bytes"])

    @property
    def max_stack_values(self) -> int:
        return int(self.raw["max_stack_values"])

    @property
    def max_handles(self) -> int:
        return int(self.raw["max_handles"])

    @property
    def supports_store_program(self) -> bool:
        return bool(self.raw["supports_store_program"])

    @property
    def capabilities(self) -> list[Capability]:
        return [Capability(item) for item in self.raw.get("capabilities", [])]

    def supports(self, name_or_id: str | int) -> bool:
        return self.capability(name_or_id) is not None

    def capability(self, name_or_id: str | int) -> Capability | None:
        for capability in self.capabilities:
            if isinstance(name_or_id, int) and capability.id == name_or_id:
                return capability
            if not isinstance(name_or_id, int) and capability.name == str(name_or_id):
                return capability
        return None

    @property
    def capability_names(self) -> list[str]:
        return [capability.name for capability in self.capabilities]


@dataclass(frozen=True)
class ProtocolResult:
    command: str
    frame: bytes
    response: bytes | None = None
    decoded_response: dict[str, Any] | None = None

    @property
    def kind(self) -> str | None:
        if self.decoded_response is None:
            return None
        return self.decoded_response.get("kind")

    @property
    def payload(self) -> dict[str, Any] | None:
        if self.decoded_response is None:
            return None
        return self.decoded_response.get("payload")

    @property
    def board_descriptor(self) -> BoardDescriptor | None:
        if self.kind != "caps_report" or self.payload is None:
            return None
        return BoardDescriptor(self.payload)


@dataclass(frozen=True)
class SessionResult:
    results: list[ProtocolResult]

    @property
    def frames(self) -> list[bytes]:
        return [result.frame for result in self.results]

    @property
    def responses(self) -> list[bytes | None]:
        return [result.response for result in self.results]

    @property
    def decoded_responses(self) -> list[dict[str, Any] | None]:
        return [result.decoded_response for result in self.results]

    @property
    def board_descriptor(self) -> BoardDescriptor | None:
        for result in self.results:
            descriptor = result.board_descriptor
            if descriptor is not None:
                return descriptor
        return None


class Session:
    def __init__(self, *, next_request_id: int = 1, transport: Any = None, timeout_ms: int = 1000):
        self.next_request_id = next_request_id
        self.transport = transport
        self.timeout_ms = timeout_ms

    def hello(self, host_name: str = DEFAULT_HOST_NAME, host_nonce: int = DEFAULT_HOST_NONCE) -> ProtocolResult:
        frame = self._call_native(_native.hello_wire, host_name, host_nonce)
        return self._dispatch("hello", frame)

    def capabilities(self) -> ProtocolResult:
        frame = self._call_native(_native.caps_query_wire)
        return self._dispatch("capabilities", frame)

    caps = capabilities

    def board_descriptor(self) -> BoardDescriptor | None:
        return self.capabilities().board_descriptor

    def blink_module(self, pin: int = 13, high_ms: int = 250, low_ms: int = 250, max_stack: int = 4) -> bytes:
        return _native.blink_module(pin, high_ms, low_ms, max_stack)

    def gpio_read_module(self, *, pin: int, mode: str | int = "input", max_stack: int = 2) -> bytes:
        return _native.gpio_read_module(pin, self._gpio_read_mode(mode), max_stack)

    def upload(self, *, program_id: int = DEFAULT_PROGRAM_ID, module_bytes: bytes) -> SessionResult:
        return SessionResult([
            self._dispatch("program_begin", self._call_native(_native.program_begin_wire, program_id, module_bytes)),
            self._dispatch("program_chunk", self._call_native(_native.program_chunk_wire, program_id, 0, module_bytes)),
            self._dispatch("program_end", self._call_native(_native.program_end_wire, program_id)),
        ])

    def upload_blink(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        pin: int = 13,
        high_ms: int = 250,
        low_ms: int = 250,
        max_stack: int = 4,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.blink_module(pin=pin, high_ms=high_ms, low_ms=low_ms, max_stack=max_stack),
        )

    def upload_gpio_read(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        pin: int,
        mode: str | int = "input",
        max_stack: int = 2,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.gpio_read_module(pin=pin, mode=mode, max_stack=max_stack),
        )

    def run(self, *, program_id: int = DEFAULT_PROGRAM_ID, instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET) -> ProtocolResult:
        frame = self._call_native(_native.run_background_wire, program_id, instruction_budget)
        return self._dispatch("run", frame)

    def stop(self) -> ProtocolResult:
        frame = self._call_native(_native.stop_wire)
        return self._dispatch("stop", frame)

    def blink(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        handshake: bool = False,
        query_caps: bool = False,
        pin: int = 13,
        high_ms: int = 250,
        low_ms: int = 250,
        max_stack: int = 4,
    ) -> SessionResult:
        results: list[ProtocolResult] = []
        if handshake:
            results.append(self.hello())
        if query_caps:
            results.append(self.capabilities())
        results.extend(
            self.upload_blink(
                program_id=program_id,
                pin=pin,
                high_ms=high_ms,
                low_ms=low_ms,
                max_stack=max_stack,
            ).results
        )
        results.append(self.run(program_id=program_id, instruction_budget=instruction_budget))
        return SessionResult(results)

    def gpio_read(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        handshake: bool = False,
        query_caps: bool = False,
        pin: int,
        mode: str | int = "input",
        max_stack: int = 2,
    ) -> SessionResult:
        results: list[ProtocolResult] = []
        if handshake:
            results.append(self.hello())
        if query_caps:
            results.append(self.capabilities())
        results.extend(
            self.upload_gpio_read(
                program_id=program_id,
                pin=pin,
                mode=mode,
                max_stack=max_stack,
            ).results
        )
        results.append(self.run(program_id=program_id, instruction_budget=instruction_budget))
        return SessionResult(results)

    def run_command(self, line: str, **options: Any) -> SessionResult:
        words = line.split()
        if not words:
            return SessionResult([])
        command = words.pop(0)
        if command == "hello":
            self._ensure_no_extra(words, command)
            return SessionResult([self.hello(**options)])
        if command in {"caps", "capabilities"}:
            self._ensure_no_extra(words, command)
            return SessionResult([self.capabilities()])
        if command == "upload-blink":
            self._ensure_no_extra(words, command)
            return self.upload_blink(**options)
        if command in {"upload-gpio-read", "upload-gpio.read"}:
            return self.upload_gpio_read(
                **self._with_gpio_read_options(words, command, options, allow_budget=False)
            )
        if command == "run":
            return SessionResult([self.run(**self._with_optional_budget(words, command, options))])
        if command == "stop":
            self._ensure_no_extra(words, command)
            return SessionResult([self.stop()])
        if command == "blink":
            return self.blink(**self._with_optional_budget(words, command, options))
        if command in {"gpio-read", "gpio.read"}:
            return self.gpio_read(**self._with_gpio_read_options(words, command, options))
        raise ValueError(f"unknown Board VM session command: {command}")

    def decode_response(self, response: bytes) -> dict[str, Any]:
        return _native.decode_response(response)

    def _call_native(self, func: Any, *args: Any) -> bytes:
        result = func(self.next_request_id, *args)
        self.next_request_id = int(result["next_request_id"])
        return bytes(result["frame"])

    def _dispatch(self, command: str, frame: bytes) -> ProtocolResult:
        response = None
        decoded = None
        if self.transport is not None:
            if hasattr(self.transport, "transact"):
                response = self.transport.transact(frame, timeout_ms=self.timeout_ms)
            elif hasattr(self.transport, "write"):
                self.transport.write(frame)
            else:
                raise TypeError("Board VM transport must expose transact(frame, timeout_ms=...) or write(frame)")
        if response is not None:
            decoded = self.decode_response(response)
        return ProtocolResult(command=command, frame=frame, response=response, decoded_response=decoded)

    @staticmethod
    def _ensure_no_extra(words: Iterable[str], command: str) -> None:
        words = list(words)
        if words:
            raise ValueError(f"{command} got unexpected argument: {words[0]}")

    def _with_optional_budget(self, words: list[str], command: str, options: dict[str, Any]) -> dict[str, Any]:
        merged = dict(options)
        if words:
            merged["instruction_budget"] = int(words.pop(0))
        self._ensure_no_extra(words, command)
        return merged

    def _with_gpio_read_options(
        self,
        words: list[str],
        command: str,
        options: dict[str, Any],
        *,
        allow_budget: bool = True,
    ) -> dict[str, Any]:
        merged = dict(options)
        if words:
            merged["pin"] = int(words.pop(0))
        if words:
            mode_or_budget = words.pop(0)
            if allow_budget and mode_or_budget.isdecimal():
                merged["instruction_budget"] = int(mode_or_budget)
            else:
                merged["mode"] = mode_or_budget
        if allow_budget and words:
            merged["instruction_budget"] = int(words.pop(0))
        self._ensure_no_extra(words, command)
        if "pin" not in merged:
            raise ValueError(f"{command} requires pin")
        return merged

    @staticmethod
    def _gpio_read_mode(mode: str | int) -> int:
        if isinstance(mode, int):
            return mode
        normalized = str(mode).replace("-", "_")
        if normalized.isdecimal():
            return int(normalized)
        try:
            return GPIO_READ_MODES[normalized]
        except KeyError as exc:
            raise ValueError(f"unsupported GPIO read mode: {mode!r}") from exc


__all__ = [
    "BoardDescriptor",
    "Capability",
    "GPIO_READ_MODES",
    "ProtocolResult",
    "Session",
    "SessionResult",
]
