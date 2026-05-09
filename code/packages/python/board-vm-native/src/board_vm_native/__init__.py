"""Python sugar over Rust-owned Board VM protocol frames."""

from __future__ import annotations

import pathlib
import subprocess
import sys
import time
from dataclasses import dataclass
from typing import Any, Iterable

from . import board_vm_native as _native


DEFAULT_RUST_WORKSPACE = pathlib.Path(__file__).resolve().parents[4] / "rust"
DEFAULT_HOST_NAME = "python-board-vm"
DEFAULT_HOST_NONCE = 0xB0A2D001
DEFAULT_PROGRAM_ID = 1
DEFAULT_INSTRUCTION_BUDGET = 12
DEFAULT_PICO_RUNTIME_PORT_WAIT_MS = 5_000
DEFAULT_PICO_RUNTIME_PORT_POLL_MS = 250
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
    def wireless(self) -> list[dict[str, Any]]:
        return [dict(item) for item in self.raw.get("wireless", [])]

    @property
    def wireless_transports(self) -> list[str]:
        return [str(item["transport"]) for item in self.wireless]

    def supports_wireless_transport(self, transport: str) -> bool:
        return str(transport) in self.wireless_transports

    @property
    def supports_wifi(self) -> bool:
        return self.supports_wireless_transport("wifi")

    @property
    def supports_bluetooth(self) -> bool:
        return any(
            transport.startswith("bluetooth")
            for transport in self.wireless_transports
        )

    @property
    def supports_ota_update(self) -> bool:
        return any(bool(item.get("ota_update")) for item in self.wireless)

    @property
    def capabilities(self) -> list[str]:
        return [str(capability) for capability in self.raw.get("capabilities", [])]


@dataclass(frozen=True)
class EspUploadOptions:
    raw: dict[str, Any]

    @property
    def board_id(self) -> str:
        return str(self.raw["board_id"])

    @property
    def baud_rate(self) -> int:
        return int(self.raw["baud_rate"])

    @property
    def timeout_ms(self) -> int:
        return int(self.raw["timeout_ms"])

    @property
    def reset_into_bootloader(self) -> bool:
        return bool(self.raw["reset_into_bootloader"])

    @property
    def offset(self) -> int:
        return int(self.raw["offset"])

    @property
    def block_size(self) -> int:
        return int(self.raw["block_size"])

    @property
    def flash_size(self) -> int | None:
        value = self.raw.get("flash_size")
        return None if value is None else int(value)

    @property
    def verify_md5(self) -> bool:
        return bool(self.raw["verify_md5"])

    @property
    def stay_in_bootloader(self) -> bool:
        return bool(self.raw["stay_in_bootloader"])


@dataclass(frozen=True)
class PicoUf2UploadOptions:
    raw: dict[str, Any]

    @property
    def board_id(self) -> str:
        return str(self.raw["board_id"])

    @property
    def command(self) -> str:
        return str(self.raw["command"])

    @property
    def volume_label(self) -> str:
        return str(self.raw["volume_label"])

    @property
    def image_extension(self) -> str:
        return str(self.raw["image_extension"])

    @property
    def auto_detect_mount(self) -> bool:
        return bool(self.raw["auto_detect_mount"])


@dataclass(frozen=True)
class BoardDevice:
    raw: dict[str, Any]

    @property
    def id(self) -> str:
        return str(self.raw["id"])

    @property
    def port(self) -> str:
        return str(self.raw["port"])

    @property
    def transport(self) -> str:
        return str(self.raw["transport"])

    @property
    def display_name(self) -> str:
        return str(self.raw["display_name"])

    @property
    def target(self) -> BoardTarget | None:
        target = self.raw.get("target")
        return None if target is None else BoardTarget(target)

    @property
    def target_confidence(self) -> int:
        return int(self.raw.get("target_confidence", 0))

    @property
    def bootloader(self) -> bool:
        return bool(self.raw.get("bootloader", False))

    @property
    def tags(self) -> list[str]:
        return [str(tag) for tag in self.raw.get("tags", [])]


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


class Connection:
    def __init__(
        self,
        *,
        target: BoardTarget,
        port: str | None,
        transport: Any = None,
        runner: Any = None,
        cargo_workspace: str | pathlib.Path | None = None,
        firmware_image: str | pathlib.Path | None = None,
        device_discovery: Any = None,
        pico_uf2_mount: str | pathlib.Path | None = None,
        pico_uf2_roots: Iterable[str | pathlib.Path] | None = None,
        pico_runtime_port: bool = True,
        pico_runtime_port_wait_ms: int = DEFAULT_PICO_RUNTIME_PORT_WAIT_MS,
        pico_runtime_port_poll_ms: int = DEFAULT_PICO_RUNTIME_PORT_POLL_MS,
        esp_upload_options: dict[str, Any] | None = None,
        pico_uf2_upload_options: dict[str, Any] | None = None,
    ):
        self.target = target
        self.port = None if port is None else str(port)
        self.transport = transport
        self.runner = _default_runner if runner is None else runner
        self.cargo_workspace = pathlib.Path(cargo_workspace or DEFAULT_RUST_WORKSPACE)
        self.firmware_image = None if firmware_image is None else str(firmware_image)
        self.device_discovery = devices if device_discovery is None else device_discovery
        self.pico_uf2_mount = None if pico_uf2_mount is None else str(pico_uf2_mount)
        self.pico_uf2_roots = None if pico_uf2_roots is None else [str(root) for root in pico_uf2_roots]
        self.pico_runtime_port = pico_runtime_port
        self.pico_runtime_port_wait_ms = int(pico_runtime_port_wait_ms)
        self.pico_runtime_port_poll_ms = int(pico_runtime_port_poll_ms)
        self.esp_upload_options = dict(esp_upload_options or {})
        self.pico_uf2_upload_options = dict(pico_uf2_upload_options or {})

    @property
    def board_id(self) -> str:
        return self.target.board_id

    @property
    def family(self) -> str:
        return self.target.family

    def session(self, **options: Any) -> Session:
        options.setdefault("transport", self.transport)
        return Session(**options)

    def flash(self) -> Any:
        if self.firmware_image is None:
            raise ValueError("Board VM flash requires firmware_image")
        if self.family == "esp32":
            command = esp_upload_command(
                self.board_id,
                port=self.port,
                image=self.firmware_image,
                **self.esp_upload_options,
            )
            return self._run_board_vm(command)
        if self.family == "raspberry_pi_pico":
            command = pico_uf2_upload_command(
                self.board_id,
                image=self.firmware_image,
                mount=self.pico_uf2_mount,
                roots=self.pico_uf2_roots,
                **self.pico_uf2_upload_options,
            )
            result = self._run_board_vm(command)
            if self.pico_runtime_port:
                self.rediscover_runtime_port()
            return result
        raise ValueError(f"Python flash sugar does not support {self.board_id!r}")

    def rediscover_runtime_port(self) -> BoardDevice:
        deadline = time.monotonic() + (self.pico_runtime_port_wait_ms / 1000)
        last_error: ValueError | None = None
        while True:
            try:
                selected = select_runtime_device(
                    self.board_id,
                    device_candidates=self.device_discovery(),
                )
                self.port = selected.port
                return selected
            except ValueError as error:
                last_error = error

            if self.pico_runtime_port_wait_ms <= 0 or time.monotonic() >= deadline:
                break
            remaining = max(0, deadline - time.monotonic())
            poll_seconds = max(self.pico_runtime_port_poll_ms / 1000, 0.01)
            time.sleep(min(poll_seconds, remaining))

        detail = "" if last_error is None else f"\n{last_error}"
        raise ValueError(
            f"Pico UF2 upload finished, but no runtime serial device was found for {self.board_id!r}.{detail}"
        )

    def _run_board_vm(self, args: list[str]) -> Any:
        command = ["cargo", "run", "-p", "board-vm-cli", "--bin", "board-vm", "--", *args]
        return self.runner(command, cwd=str(self.cargo_workspace))


def _default_runner(command: list[str], *, cwd: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)


def known_targets() -> list[BoardTarget]:
    return [BoardTarget(raw) for raw in _native.known_targets()]


def detect_target(selector: str) -> BoardTarget | None:
    raw = _native.detect_target(str(selector))
    if raw is None:
        return None
    return BoardTarget(raw)


def find_target(board_id: str) -> BoardTarget | None:
    return detect_target(board_id)


def esp_upload_options(
    selector: str = "esp32-devkit-v1",
    **overrides: Any,
) -> EspUploadOptions | None:
    raw = _native.esp_upload_options(str(selector))
    if raw is None:
        return None
    merged = dict(raw)
    merged.update(overrides)
    return EspUploadOptions(merged)


def pico_uf2_upload_options(
    selector: str = "raspberry-pi-pico",
    **overrides: Any,
) -> PicoUf2UploadOptions | None:
    raw = _native.pico_uf2_upload_options(str(selector))
    if raw is None:
        return None
    merged = dict(raw)
    merged.update(overrides)
    return PicoUf2UploadOptions(merged)


def pico_uf2_mounts(roots: Iterable[str] | None = None) -> list[str]:
    if roots is None:
        return [str(mount) for mount in _native.pico_uf2_mounts()]
    return [str(mount) for mount in _native.pico_uf2_mounts([str(root) for root in roots])]


def _pico_uf2_mount_list(mounts: Iterable[str]) -> str:
    return "\n".join(f"{index}. {mount}" for index, mount in enumerate(mounts, start=1))


def pico_uf2_mount(roots: Iterable[str] | None = None) -> str:
    mounts = pico_uf2_mounts(roots)
    if len(mounts) == 1:
        return mounts[0]
    if not mounts:
        raise ValueError(
            "No Pico BOOTSEL UF2 mount found. "
            "Hold BOOTSEL while plugging in the Pico/Pico W."
        )
    raise ValueError(
        "Multiple Pico BOOTSEL UF2 mounts found; choose one.\n"
        f"{_pico_uf2_mount_list(mounts)}"
    )


DeviceReference = BoardDevice | dict[str, Any] | str | int


def select_device(
    selector: str = "auto",
    *,
    device: DeviceReference | None = None,
    device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None,
) -> BoardDevice:
    candidates = [
        item if isinstance(item, BoardDevice) else BoardDevice(item)
        for item in (devices() if device_candidates is None else device_candidates)
    ]

    if isinstance(device, BoardDevice):
        return device
    if isinstance(device, dict):
        return BoardDevice(device)
    if isinstance(device, int):
        try:
            return candidates[device]
        except IndexError as error:
            raise ValueError(f"No Board VM device at index {device}.") from error
    if device is not None:
        needle = str(device)
        for candidate in candidates:
            if candidate.id == needle or candidate.port == needle:
                return candidate
        raise ValueError(f"No Board VM device named {needle!r}.\n{device_list(candidates)}")

    target = None if selector == "auto" else detect_target(selector)
    if selector != "auto" and target is None:
        raise ValueError(f"unsupported board: {selector!r}")

    if target is None:
        matches = [candidate for candidate in candidates if candidate.target is not None]
    else:
        exact_matches = [
            candidate
            for candidate in candidates
            if candidate.target is not None and candidate.target.board_id == target.board_id
        ]
        matches = exact_matches or [
            candidate for candidate in candidates if candidate.target is None
        ]

    if target is None and not matches and len(candidates) == 1:
        matches = candidates
    if len(matches) == 1:
        return matches[0]

    if not candidates:
        raise ValueError("No Board VM devices found. Plug in a board or pass an explicit device.")

    if not matches and target is None:
        reason = "Multiple Board VM devices found; choose one"
    elif not matches:
        reason = "No matching Board VM device found"
    else:
        reason = "Multiple Board VM devices match"
    raise ValueError(f"{reason}.\n{device_list(candidates)}")


def runtime_devices(
    selector: str = "auto",
    *,
    device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None,
) -> list[BoardDevice]:
    candidates = [
        item if isinstance(item, BoardDevice) else BoardDevice(item)
        for item in (devices() if device_candidates is None else device_candidates)
        if not (item.bootloader if isinstance(item, BoardDevice) else item.get("bootloader", False))
    ]
    target = None if selector == "auto" else detect_target(selector)
    if selector != "auto" and target is None:
        raise ValueError(f"unsupported board: {selector!r}")

    if target is None:
        matches = [candidate for candidate in candidates if candidate.target is not None]
        if not matches and len(candidates) == 1:
            return candidates
        return matches

    exact_matches = [
        candidate
        for candidate in candidates
        if candidate.target is not None and candidate.target.board_id == target.board_id
    ]
    if exact_matches:
        return exact_matches
    return [candidate for candidate in candidates if candidate.target is None]


def select_runtime_device(
    selector: str = "auto",
    *,
    device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None,
) -> BoardDevice:
    candidates = [
        item if isinstance(item, BoardDevice) else BoardDevice(item)
        for item in (devices() if device_candidates is None else device_candidates)
    ]
    matches = runtime_devices(selector, device_candidates=candidates)
    if len(matches) == 1:
        return matches[0]

    if not candidates:
        raise ValueError(
            "No Board VM runtime serial devices found. "
            "Plug in a board or pass an explicit port."
        )

    reason = (
        "No matching runtime serial device found"
        if not matches
        else "Multiple runtime serial devices match"
    )
    raise ValueError(f"{reason}.\n{device_list(candidates)}")


def pick_device(
    selector: str = "auto",
    *,
    device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None,
    input_func: Any = input,
    output: Any = None,
) -> BoardDevice:
    candidates = [
        item if isinstance(item, BoardDevice) else BoardDevice(item)
        for item in (devices() if device_candidates is None else device_candidates)
    ]
    if not candidates:
        raise ValueError("No Board VM devices found. Plug in a board.")

    try:
        return select_device(selector, device_candidates=candidates)
    except ValueError:
        pass

    output = sys.stdout if output is None else output
    output.write(device_list(candidates))
    output.write("\n")
    output.write(f"Select board [1-{len(candidates)}]: ")
    choice = input_func("")
    try:
        index = int(str(choice).strip())
    except ValueError as error:
        raise ValueError(f"Invalid Board VM device selection: {choice!r}") from error
    if not 1 <= index <= len(candidates):
        raise ValueError(f"Invalid Board VM device selection: {choice!r}")

    selected = candidates[index - 1]
    target = None if selector == "auto" else detect_target(selector)
    if target is not None and selected.target is not None and selected.target.board_id != target.board_id:
        raise ValueError(
            f"Selected {selected.port} is {selected.target.board_id}, not {target.board_id}."
        )
    return selected


def esp_upload_command(
    selector: str = "esp32-devkit-v1",
    *,
    port: str | None = None,
    device: DeviceReference | None = None,
    device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None,
    image: str,
    **overrides: Any,
) -> list[str]:
    options = esp_upload_options(selector, **overrides)
    if options is None:
        raise ValueError(f"ESP upload is not supported for {selector!r}")

    if port is None:
        port = select_device(
            selector,
            device=device,
            device_candidates=device_candidates,
        ).port

    command = [
        "esp-upload",
        "--port",
        str(port),
        "--image",
        str(image),
        "--baud",
        str(options.baud_rate),
        "--timeout-ms",
        str(options.timeout_ms),
        "--offset",
        str(options.offset),
        "--block-size",
        str(options.block_size),
    ]
    if options.flash_size is not None:
        command.extend(["--flash-size", str(options.flash_size)])
    if not options.reset_into_bootloader:
        command.append("--no-reset")
    if not options.verify_md5:
        command.append("--no-verify")
    if options.stay_in_bootloader:
        command.append("--stay-in-bootloader")
    return command


def pico_uf2_upload_command(
    selector: str = "raspberry-pi-pico",
    *,
    image: str,
    mount: str | None = None,
    roots: Iterable[str] | None = None,
    auto_mount: bool = True,
    **overrides: Any,
) -> list[str]:
    options = pico_uf2_upload_options(selector, **overrides)
    if options is None:
        raise ValueError(f"Pico UF2 upload is not supported for {selector!r}")

    selected_mount = mount
    if selected_mount is None and auto_mount:
        selected_mount = pico_uf2_mount(roots)

    command = [
        options.command,
        "--image",
        str(image),
    ]
    if selected_mount is not None:
        command.extend(["--mount", str(selected_mount)])
    return command


def connect(
    selector: str = "auto",
    *,
    port: str | None = None,
    device: DeviceReference | None = None,
    device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None,
    pick: bool = False,
    input_func: Any = input,
    output: Any = None,
    flash: bool = False,
    transport: Any = None,
    runner: Any = None,
    cargo_workspace: str | pathlib.Path | None = None,
    firmware_image: str | pathlib.Path | None = None,
    esp_image: str | pathlib.Path | None = None,
    device_discovery: Any = None,
    pico_uf2_mount: str | pathlib.Path | None = None,
    pico_uf2_roots: Iterable[str | pathlib.Path] | None = None,
    pico_runtime_port: bool = True,
    pico_runtime_port_wait_ms: int = DEFAULT_PICO_RUNTIME_PORT_WAIT_MS,
    pico_runtime_port_poll_ms: int = DEFAULT_PICO_RUNTIME_PORT_POLL_MS,
    esp_upload_options: dict[str, Any] | None = None,
    pico_uf2_upload_options: dict[str, Any] | None = None,
) -> Connection:
    selected_device = None
    if pick and port is None and device is None:
        selected_device = pick_device(
            selector,
            device_candidates=device_candidates,
            input_func=input_func,
            output=output,
        )
    elif device is not None:
        selected_device = select_device(
            selector,
            device=device,
            device_candidates=device_candidates,
        )
    elif port is None and not _flash_without_port(selector, flash):
        selected_device = select_device(selector, device_candidates=device_candidates)

    selected_port = port or (selected_device.port if selected_device is not None else None)
    target = _connection_target(selector, selected_device, selected_port)
    connection = Connection(
        target=target,
        port=selected_port,
        transport=transport,
        runner=runner,
        cargo_workspace=cargo_workspace,
        firmware_image=firmware_image or esp_image,
        device_discovery=device_discovery,
        pico_uf2_mount=pico_uf2_mount,
        pico_uf2_roots=pico_uf2_roots,
        pico_runtime_port=pico_runtime_port,
        pico_runtime_port_wait_ms=pico_runtime_port_wait_ms,
        pico_runtime_port_poll_ms=pico_runtime_port_poll_ms,
        esp_upload_options=esp_upload_options,
        pico_uf2_upload_options=pico_uf2_upload_options,
    )
    if flash:
        connection.flash()
    return connection


def uno_r4_wifi(**options: Any) -> Connection:
    return connect("arduino-uno-r4-wifi", **options)


def esp32_devkit_v1(**options: Any) -> Connection:
    return connect("esp32-devkit-v1", **options)


def esp32(**options: Any) -> Connection:
    return esp32_devkit_v1(**options)


def raspberry_pi_pico(**options: Any) -> Connection:
    return connect("raspberry-pi-pico", **options)


def pico(**options: Any) -> Connection:
    return raspberry_pi_pico(**options)


def raspberry_pi_pico_w(**options: Any) -> Connection:
    return connect("raspberry-pi-pico-w", **options)


def pico_w(**options: Any) -> Connection:
    return raspberry_pi_pico_w(**options)


def _connection_target(
    selector: str,
    selected_device: BoardDevice | None,
    selected_port: str | None,
) -> BoardTarget:
    if selector != "auto":
        target = detect_target(selector)
        if target is None:
            raise ValueError(f"unsupported board: {selector!r}")
        return target
    if selected_device is not None and selected_device.target is not None:
        return selected_device.target
    if selected_port is not None:
        target = detect_target("arduino-uno-r4-wifi")
        if target is not None:
            return target
    raise ValueError(f"Could not infer the board for {selected_port or 'the selected device'}.\n{device_list()}")


def _flash_without_port(selector: str, flash: bool) -> bool:
    if not flash:
        return False
    target = detect_target(selector)
    return target is not None and target.family == "raspberry_pi_pico"


def devices(paths: Iterable[str] | None = None) -> list[BoardDevice]:
    raw_devices = (
        _native.discover_devices()
        if paths is None
        else _native.classify_devices([str(path) for path in paths])
    )
    return [BoardDevice(raw) for raw in raw_devices]


def device_list(device_candidates: Iterable[BoardDevice | dict[str, Any]] | None = None) -> str:
    items = list(devices() if device_candidates is None else device_candidates)
    if not items:
        return "No Board VM devices found."

    lines = []
    for index, item in enumerate(items, start=1):
        device = item if isinstance(item, BoardDevice) else BoardDevice(item)
        target = device.target
        target_name = target.display_name if target is not None else "Unknown board"
        confidence = f", {device.target_confidence}% match" if device.target_confidence > 0 else ""
        tags = f" [{', '.join(device.tags)}]" if device.tags else ""
        lines.append(f"{index}. {target_name} - {device.port}{confidence}{tags}")
    return "\n".join(lines)


__all__ = [
    "BoardDevice",
    "BoardDescriptor",
    "BoardTarget",
    "BOOT_POLICIES",
    "Capability",
    "Connection",
    "DEFAULT_PICO_RUNTIME_PORT_POLL_MS",
    "DEFAULT_PICO_RUNTIME_PORT_WAIT_MS",
    "DEFAULT_RUN_FLAGS",
    "DEFAULT_RUST_WORKSPACE",
    "EspUploadOptions",
    "GPIO_MODES",
    "GPIO_READ_MODES",
    "PicoUf2UploadOptions",
    "ProtocolResult",
    "RUN_FLAG_BACKGROUND_RUN",
    "RUN_FLAG_KEEP_HANDLES_AFTER_RUN",
    "RUN_FLAG_RESET_VM_BEFORE_RUN",
    "RUN_FLAGS",
    "Session",
    "SessionResult",
    "connect",
    "device_list",
    "devices",
    "detect_target",
    "esp32",
    "esp32_devkit_v1",
    "esp_upload_command",
    "esp_upload_options",
    "find_target",
    "known_targets",
    "pico",
    "pico_w",
    "pico_uf2_mount",
    "pico_uf2_upload_command",
    "pico_uf2_mounts",
    "pico_uf2_upload_options",
    "pick_device",
    "raspberry_pi_pico",
    "raspberry_pi_pico_w",
    "runtime_devices",
    "select_device",
    "select_runtime_device",
    "uno_r4_wifi",
]
