"""Python sugar over Rust-owned Board VM protocol frames."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable

from . import board_vm_native as _native


DEFAULT_HOST_NAME = "python-board-vm"
DEFAULT_HOST_NONCE = 0xB0A2D001
DEFAULT_PROGRAM_ID = 1
DEFAULT_INSTRUCTION_BUDGET = 12
BOOT_POLICIES = {
    "store_only": 0,
    "store-only": 0,
    "run_at_boot": 1,
    "run-at-boot": 1,
    "run_if_no_host": 2,
    "run-if-no-host": 2,
}
RUN_FLAG_RESET_VM_BEFORE_RUN = 0x01
RUN_FLAG_KEEP_HANDLES_AFTER_RUN = 0x02
RUN_FLAG_BACKGROUND_RUN = 0x04
DEFAULT_RUN_FLAGS = RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN
RUN_FLAGS = {
    "reset_vm_before_run": RUN_FLAG_RESET_VM_BEFORE_RUN,
    "reset-vm-before-run": RUN_FLAG_RESET_VM_BEFORE_RUN,
    "keep_handles_after_run": RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
    "keep-handles-after-run": RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
    "background_run": RUN_FLAG_BACKGROUND_RUN,
    "background-run": RUN_FLAG_BACKGROUND_RUN,
}
GPIO_READ_MODES = {
    "input": 0,
    "input_pullup": 2,
    "pullup": 2,
    "input_pulldown": 3,
    "pulldown": 3,
}
GPIO_MODES = {
    **GPIO_READ_MODES,
    "output": 1,
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
class BoardTarget:
    raw: dict[str, Any]

    @property
    def board_id(self) -> str:
        return str(self.raw["board_id"])

    @property
    def display_name(self) -> str:
        return str(self.raw["display_name"])

    @property
    def family(self) -> str:
        return str(self.raw["family"])

    @property
    def runtime_id(self) -> str:
        return str(self.raw["runtime_id"])

    @property
    def mcu(self) -> str:
        return str(self.raw["mcu"])

    @property
    def core(self) -> str:
        return str(self.raw["core"])

    @property
    def rust_target(self) -> str:
        return str(self.raw["rust_target"])

    @property
    def clock_hz(self) -> int:
        return int(self.raw["clock_hz"])

    @property
    def operating_voltage_mv(self) -> int:
        return int(self.raw["operating_voltage_mv"])

    @property
    def onboard_led(self) -> dict[str, Any] | None:
        led = self.raw.get("onboard_led")
        return None if led is None else dict(led)

    @property
    def onboard_led_pin(self) -> int | None:
        led = self.onboard_led
        if led is None:
            return None
        return int(led["pin"])

    @property
    def digital_pin_count(self) -> int:
        return int(self.raw["digital_pin_count"])

    @property
    def capabilities(self) -> list[str]:
        return [str(capability) for capability in self.raw.get("capabilities", [])]


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

    def gpio_write_module(self, *, pin: int, value: bool, max_stack: int = 3) -> bytes:
        return _native.gpio_write_module(pin, 1 if value else 0, max_stack)

    def gpio_open_module(self, *, pin: int, mode: str | int = "output", max_stack: int = 2) -> bytes:
        return _native.gpio_open_module(pin, self._gpio_mode(mode), max_stack)

    def gpio_handle_read_module(self, max_stack: int = 2) -> bytes:
        return _native.gpio_handle_read_module(max_stack)

    def gpio_handle_write_module(self, *, value: bool, max_stack: int = 3) -> bytes:
        return _native.gpio_handle_write_module(1 if value else 0, max_stack)

    def gpio_handle_close_module(self, max_stack: int = 1) -> bytes:
        return _native.gpio_handle_close_module(max_stack)

    def time_now_module(self, max_stack: int = 1) -> bytes:
        return _native.time_now_module(max_stack)

    def time_sleep_ms_module(self, duration_ms: int, max_stack: int = 1) -> bytes:
        return _native.time_sleep_ms_module(duration_ms, max_stack)

    def raw_module(
        self,
        *,
        code: bytes | bytearray,
        max_stack: int,
        flags: int = 0,
        const_pool: bytes | bytearray = b"",
    ) -> bytes:
        return _native.raw_module(flags, max_stack, bytes(code), bytes(const_pool))

    module = raw_module

    def upload(self, *, program_id: int = DEFAULT_PROGRAM_ID, module_bytes: bytes) -> SessionResult:
        return SessionResult([
            self._dispatch("program_begin", self._call_native(_native.program_begin_wire, program_id, module_bytes)),
            self._dispatch("program_chunk", self._call_native(_native.program_chunk_wire, program_id, 0, module_bytes)),
            self._dispatch("program_end", self._call_native(_native.program_end_wire, program_id)),
        ])

    def store_program(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        slot: int = 0,
        boot_policy: str | int = "run_if_no_host",
    ) -> ProtocolResult:
        frame = self._call_native(
            _native.store_program_wire,
            program_id,
            slot,
            self._boot_policy(boot_policy),
        )
        return self._dispatch("store_program", frame)

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

    def upload_gpio_write(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        pin: int,
        value: bool,
        max_stack: int = 3,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.gpio_write_module(pin=pin, value=value, max_stack=max_stack),
        )

    def upload_gpio_open(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        pin: int,
        mode: str | int = "output",
        max_stack: int = 2,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.gpio_open_module(pin=pin, mode=mode, max_stack=max_stack),
        )

    def upload_gpio_handle_read(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        max_stack: int = 2,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.gpio_handle_read_module(max_stack=max_stack),
        )

    def upload_gpio_handle_write(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        value: bool,
        max_stack: int = 3,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.gpio_handle_write_module(value=value, max_stack=max_stack),
        )

    def upload_gpio_handle_close(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        max_stack: int = 1,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.gpio_handle_close_module(max_stack=max_stack),
        )

    def upload_time_now(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        max_stack: int = 1,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.time_now_module(max_stack=max_stack),
        )

    def upload_time_sleep_ms(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        duration_ms: int,
        max_stack: int = 1,
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.time_sleep_ms_module(duration_ms, max_stack=max_stack),
        )

    def upload_raw_module(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        code: bytes | bytearray,
        max_stack: int,
        flags: int = 0,
        const_pool: bytes | bytearray = b"",
    ) -> SessionResult:
        return self.upload(
            program_id=program_id,
            module_bytes=self.raw_module(
                code=code,
                max_stack=max_stack,
                flags=flags,
                const_pool=const_pool,
            ),
        )

    def run(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        flags: int | None = None,
        reset_vm: bool = True,
        keep_handles: bool = False,
        background: bool = True,
        time_budget_ms: int = 0,
    ) -> ProtocolResult:
        frame = self._call_native(
            _native.run_wire,
            program_id,
            self._run_flags(
                flags=flags,
                reset_vm=reset_vm,
                keep_handles=keep_handles,
                background=background,
            ),
            instruction_budget,
            time_budget_ms,
        )
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

    def gpio_write(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        handshake: bool = False,
        query_caps: bool = False,
        pin: int,
        value: bool,
        max_stack: int = 3,
    ) -> SessionResult:
        results: list[ProtocolResult] = []
        if handshake:
            results.append(self.hello())
        if query_caps:
            results.append(self.capabilities())
        results.extend(
            self.upload_gpio_write(
                program_id=program_id,
                pin=pin,
                value=value,
                max_stack=max_stack,
            ).results
        )
        results.append(self.run(program_id=program_id, instruction_budget=instruction_budget))
        return SessionResult(results)

    def gpio_high(self, *, pin: int, **options: Any) -> SessionResult:
        return self.gpio_write(pin=pin, value=True, **options)

    def gpio_low(self, *, pin: int, **options: Any) -> SessionResult:
        return self.gpio_write(pin=pin, value=False, **options)

    def gpio_open(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        handshake: bool = False,
        query_caps: bool = False,
        pin: int,
        mode: str | int = "output",
        max_stack: int = 2,
    ) -> SessionResult:
        results: list[ProtocolResult] = []
        if handshake:
            results.append(self.hello())
        if query_caps:
            results.append(self.capabilities())
        results.extend(
            self.upload_gpio_open(
                program_id=program_id,
                pin=pin,
                mode=mode,
                max_stack=max_stack,
            ).results
        )
        results.append(
            self.run(
                program_id=program_id,
                instruction_budget=instruction_budget,
                keep_handles=True,
                background=False,
            )
        )
        return SessionResult(results)

    def gpio_handle_read(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        max_stack: int = 2,
    ) -> SessionResult:
        results = self.upload_gpio_handle_read(program_id=program_id, max_stack=max_stack).results
        results.append(
            self.run(
                program_id=program_id,
                instruction_budget=instruction_budget,
                reset_vm=False,
                keep_handles=True,
                background=False,
            )
        )
        return SessionResult(results)

    def gpio_handle_write(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        value: bool,
        max_stack: int = 3,
    ) -> SessionResult:
        results = self.upload_gpio_handle_write(
            program_id=program_id,
            value=value,
            max_stack=max_stack,
        ).results
        results.append(
            self.run(
                program_id=program_id,
                instruction_budget=instruction_budget,
                reset_vm=False,
                keep_handles=True,
                background=False,
            )
        )
        return SessionResult(results)

    def gpio_handle_close(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        max_stack: int = 1,
    ) -> SessionResult:
        results = self.upload_gpio_handle_close(program_id=program_id, max_stack=max_stack).results
        results.append(
            self.run(
                program_id=program_id,
                instruction_budget=instruction_budget,
                reset_vm=False,
                background=False,
            )
        )
        return SessionResult(results)

    def time_now(
        self,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        handshake: bool = False,
        query_caps: bool = False,
        max_stack: int = 1,
    ) -> SessionResult:
        results: list[ProtocolResult] = []
        if handshake:
            results.append(self.hello())
        if query_caps:
            results.append(self.capabilities())
        results.extend(self.upload_time_now(program_id=program_id, max_stack=max_stack).results)
        results.append(self.run(program_id=program_id, instruction_budget=instruction_budget))
        return SessionResult(results)

    def time_sleep_ms(
        self,
        duration_ms: int,
        *,
        program_id: int = DEFAULT_PROGRAM_ID,
        instruction_budget: int = DEFAULT_INSTRUCTION_BUDGET,
        handshake: bool = False,
        query_caps: bool = False,
        max_stack: int = 1,
    ) -> SessionResult:
        results: list[ProtocolResult] = []
        if handshake:
            results.append(self.hello())
        if query_caps:
            results.append(self.capabilities())
        results.extend(
            self.upload_time_sleep_ms(
                program_id=program_id,
                duration_ms=duration_ms,
                max_stack=max_stack,
            ).results
        )
        results.append(self.run(program_id=program_id, instruction_budget=instruction_budget))
        return SessionResult(results)

    sleep_ms = time_sleep_ms

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
        if command in {"upload-gpio-write", "upload-gpio.write"}:
            return self.upload_gpio_write(
                **self._with_gpio_write_options(words, command, options, allow_budget=False)
            )
        if command in {"upload-gpio-open", "upload-gpio.open"}:
            return self.upload_gpio_open(
                **self._with_gpio_open_options(words, command, options, allow_budget=False)
            )
        if command in {"upload-gpio-handle-read", "upload-gpio.handle-read"}:
            self._ensure_no_extra(words, command)
            return self.upload_gpio_handle_read(**options)
        if command in {"upload-gpio-handle-write", "upload-gpio.handle-write"}:
            return self.upload_gpio_handle_write(
                **self._with_gpio_handle_write_options(words, command, options, allow_budget=False)
            )
        if command in {"upload-gpio-handle-close", "upload-gpio.handle-close"}:
            self._ensure_no_extra(words, command)
            return self.upload_gpio_handle_close(**options)
        if command in {"upload-time-now", "upload-time.now"}:
            self._ensure_no_extra(words, command)
            return self.upload_time_now(**options)
        if command in {"upload-time-sleep-ms", "upload-time.sleep_ms", "upload-sleep-ms"}:
            return self.upload_time_sleep_ms(
                **self._with_time_sleep_ms_options(words, command, options, allow_budget=False)
            )
        if command in {"store-program", "store.program"}:
            return SessionResult([self.store_program(**self._with_store_program_options(words, command, options))])
        if command == "run":
            return SessionResult([self.run(**self._with_optional_budget(words, command, options))])
        if command == "stop":
            self._ensure_no_extra(words, command)
            return SessionResult([self.stop()])
        if command == "blink":
            return self.blink(**self._with_optional_budget(words, command, options))
        if command in {"gpio-read", "gpio.read"}:
            return self.gpio_read(**self._with_gpio_read_options(words, command, options))
        if command in {"gpio-write", "gpio.write"}:
            return self.gpio_write(**self._with_gpio_write_options(words, command, options))
        if command in {"gpio-high", "gpio.high"}:
            return self.gpio_write(**self._with_gpio_level_options(words, command, options, value=True))
        if command in {"gpio-low", "gpio.low"}:
            return self.gpio_write(**self._with_gpio_level_options(words, command, options, value=False))
        if command in {"gpio-open", "gpio.open"}:
            return self.gpio_open(**self._with_gpio_open_options(words, command, options))
        if command in {"gpio-handle-read", "gpio.handle-read"}:
            return self.gpio_handle_read(**self._with_optional_budget(words, command, options))
        if command in {"gpio-handle-write", "gpio.handle-write"}:
            return self.gpio_handle_write(
                **self._with_gpio_handle_write_options(words, command, options)
            )
        if command in {"gpio-handle-close", "gpio.handle-close"}:
            return self.gpio_handle_close(**self._with_optional_budget(words, command, options))
        if command in {"time-now", "time.now", "now"}:
            return self.time_now(**self._with_optional_budget(words, command, options))
        if command in {"time-sleep-ms", "time.sleep_ms", "sleep-ms"}:
            return self.time_sleep_ms(
                **self._with_time_sleep_ms_options(words, command, options)
            )
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

    @staticmethod
    def _run_flags(
        *,
        flags: int | None,
        reset_vm: bool,
        keep_handles: bool,
        background: bool,
    ) -> int:
        if flags is not None:
            return int(flags)
        value = 0
        if reset_vm:
            value |= RUN_FLAG_RESET_VM_BEFORE_RUN
        if keep_handles:
            value |= RUN_FLAG_KEEP_HANDLES_AFTER_RUN
        if background:
            value |= RUN_FLAG_BACKGROUND_RUN
        return value

    def _with_store_program_options(
        self,
        words: list[str],
        command: str,
        options: dict[str, Any],
    ) -> dict[str, Any]:
        merged = dict(options)
        if words:
            merged["program_id"] = int(words.pop(0))
        if words:
            merged["slot"] = int(words.pop(0))
        if words:
            merged["boot_policy"] = words.pop(0)
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

    def _with_time_sleep_ms_options(
        self,
        words: list[str],
        command: str,
        options: dict[str, Any],
        *,
        allow_budget: bool = True,
    ) -> dict[str, Any]:
        merged = dict(options)
        if words:
            merged["duration_ms"] = int(words.pop(0))
        if allow_budget and words:
            merged["instruction_budget"] = int(words.pop(0))
        self._ensure_no_extra(words, command)
        if "duration_ms" not in merged:
            raise ValueError(f"{command} requires duration_ms")
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

    @staticmethod
    def _gpio_mode(mode: str | int) -> int:
        if isinstance(mode, int):
            return mode
        normalized = str(mode).replace("-", "_")
        if normalized.isdecimal():
            return int(normalized)
        try:
            return GPIO_MODES[normalized]
        except KeyError as exc:
            raise ValueError(f"unsupported GPIO mode: {mode!r}") from exc

    @staticmethod
    def _boot_policy(policy: str | int) -> int:
        if isinstance(policy, int):
            return policy
        normalized = str(policy).replace("-", "_")
        if normalized.isdecimal():
            return int(normalized)
        try:
            return BOOT_POLICIES[normalized]
        except KeyError as exc:
            raise ValueError(f"unsupported boot policy: {policy!r}") from exc

    def _with_gpio_write_options(
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
            merged["value"] = self._gpio_write_value(words.pop(0))
        if allow_budget and words:
            merged["instruction_budget"] = int(words.pop(0))
        self._ensure_no_extra(words, command)
        if "pin" not in merged:
            raise ValueError(f"{command} requires pin")
        if "value" not in merged:
            raise ValueError(f"{command} requires value")
        return merged

    def _with_gpio_open_options(
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

    def _with_gpio_handle_write_options(
        self,
        words: list[str],
        command: str,
        options: dict[str, Any],
        *,
        allow_budget: bool = True,
    ) -> dict[str, Any]:
        merged = dict(options)
        if words:
            merged["value"] = self._gpio_write_value(words.pop(0))
        if allow_budget and words:
            merged["instruction_budget"] = int(words.pop(0))
        self._ensure_no_extra(words, command)
        if "value" not in merged:
            raise ValueError(f"{command} requires value")
        return merged

    def _with_gpio_level_options(
        self,
        words: list[str],
        command: str,
        options: dict[str, Any],
        *,
        value: bool,
    ) -> dict[str, Any]:
        merged = dict(options)
        if words:
            merged["pin"] = int(words.pop(0))
        if words:
            merged["instruction_budget"] = int(words.pop(0))
        self._ensure_no_extra(words, command)
        if "pin" not in merged:
            raise ValueError(f"{command} requires pin")
        merged["value"] = value
        return merged

    @staticmethod
    def _gpio_write_value(value: str | int | bool) -> bool:
        if isinstance(value, bool):
            return value
        if isinstance(value, int):
            return value != 0
        normalized = str(value).strip().lower().replace("-", "_")
        if normalized in {"1", "true", "high", "on"}:
            return True
        if normalized in {"0", "false", "low", "off"}:
            return False
        raise ValueError(f"unsupported GPIO write value: {value!r}")


def known_targets() -> list[BoardTarget]:
    return [BoardTarget(raw) for raw in _native.known_targets()]


def detect_target(selector: str) -> BoardTarget | None:
    raw = _native.detect_target(str(selector))
    if raw is None:
        return None
    return BoardTarget(raw)


def find_target(board_id: str) -> BoardTarget | None:
    return detect_target(board_id)


__all__ = [
    "BoardDescriptor",
    "BoardTarget",
    "BOOT_POLICIES",
    "Capability",
    "DEFAULT_RUN_FLAGS",
    "GPIO_MODES",
    "GPIO_READ_MODES",
    "ProtocolResult",
    "RUN_FLAG_BACKGROUND_RUN",
    "RUN_FLAG_KEEP_HANDLES_AFTER_RUN",
    "RUN_FLAG_RESET_VM_BEFORE_RUN",
    "RUN_FLAGS",
    "Session",
    "SessionResult",
    "detect_target",
    "find_target",
    "known_targets",
]
