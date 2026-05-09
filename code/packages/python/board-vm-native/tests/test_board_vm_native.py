from board_vm_native import (
    BoardDevice,
    BoardDescriptor,
    BoardTarget,
    EspUploadOptions,
    ProtocolResult,
    Session,
    device_list,
    devices,
    detect_target,
    esp_upload_command,
    esp_upload_options,
    find_target,
    known_targets,
)


class FakeWriteTransport:
    def __init__(self):
        self.frames = []

    def write(self, frame):
        self.frames.append(frame)


def test_native_session_builds_protocol_bytes_in_rust():
    session = Session()

    hello = session.hello(host_nonce=0x1234_ABCD)
    assert isinstance(hello.frame, bytes)
    assert len(hello.frame) > 0
    assert session.next_request_id == 2

    caps = session.capabilities()
    assert isinstance(caps.frame, bytes)
    assert len(caps.frame) > 0
    assert session.next_request_id == 3

    module = session.blink_module(pin=13, high_ms=250, low_ms=250, max_stack=4)
    assert isinstance(module, bytes)
    assert len(module) > 0

    gpio_module = session.gpio_read_module(pin=13, mode="pullup", max_stack=2)
    assert isinstance(gpio_module, bytes)
    assert len(gpio_module) > 0

    gpio_write_module = session.gpio_write_module(pin=13, value=True, max_stack=3)
    assert isinstance(gpio_write_module, bytes)
    assert len(gpio_write_module) > 0

    gpio_open_module = session.gpio_open_module(pin=13, mode="output", max_stack=2)
    assert isinstance(gpio_open_module, bytes)
    assert len(gpio_open_module) > 0

    gpio_handle_read_module = session.gpio_handle_read_module(max_stack=2)
    assert isinstance(gpio_handle_read_module, bytes)
    assert len(gpio_handle_read_module) > 0

    gpio_handle_write_module = session.gpio_handle_write_module(value=True, max_stack=3)
    assert isinstance(gpio_handle_write_module, bytes)
    assert len(gpio_handle_write_module) > 0

    gpio_handle_close_module = session.gpio_handle_close_module(max_stack=1)
    assert isinstance(gpio_handle_close_module, bytes)
    assert len(gpio_handle_close_module) > 0

    time_module = session.time_now_module(max_stack=1)
    assert isinstance(time_module, bytes)
    assert len(time_module) > 0

    sleep_module = session.time_sleep_ms_module(250, max_stack=1)
    assert isinstance(sleep_module, bytes)
    assert len(sleep_module) > 0

    raw_module = session.raw_module(code=b"\x00", max_stack=1, const_pool=b"\xAA\x55")
    assert raw_module.startswith(b"BVM1")
    assert raw_module.endswith(b"\xAA\x55")

    stop = session.stop()
    assert isinstance(stop.frame, bytes)
    assert len(stop.frame) > 0
    assert session.next_request_id == 4


def test_known_targets_are_exposed_from_rust_registry():
    targets = known_targets()
    esp32 = find_target("esp32-devkit-v1")
    pico_w = find_target("raspberry-pi-pico-w")

    assert all(isinstance(target, BoardTarget) for target in targets)
    assert esp32 is not None
    assert esp32.family == "esp32"
    assert esp32.runtime_id == "board-vm-esp32"
    assert esp32.onboard_led_pin == 2
    assert "gpio.open" in esp32.capabilities
    assert pico_w is not None
    assert pico_w.onboard_led == {"kind": "wireless_chip_gpio", "pin": 0}


def test_targets_are_detected_from_rust_owned_aliases():
    esp32 = detect_target("esp32")
    pico = detect_target("Raspberry Pi Pico")
    pico_w = find_target("pico-w")

    assert esp32 is not None
    assert esp32.board_id == "esp32-devkit-v1"
    assert esp32.rust_target == "xtensa-esp32-none-elf"
    assert pico is not None
    assert pico.board_id == "raspberry-pi-pico"
    assert pico_w is not None
    assert pico_w.board_id == "raspberry-pi-pico-w"
    assert detect_target("not-a-board") is None


def test_esp_upload_options_are_exposed_from_rust_language_core():
    options = esp_upload_options("esp32")

    assert isinstance(options, EspUploadOptions)
    assert options.board_id == "esp32-devkit-v1"
    assert options.baud_rate == 115_200
    assert options.timeout_ms == 1_000
    assert options.reset_into_bootloader is True
    assert options.offset == 0x1000
    assert options.block_size == 0x400
    assert options.flash_size == 4 * 1024 * 1024
    assert options.verify_md5 is True
    assert options.stay_in_bootloader is False
    assert esp_upload_options("pico") is None

    overridden = esp_upload_options("esp32", offset=0x2000, verify_md5=False)
    assert overridden is not None
    assert overridden.offset == 0x2000
    assert overridden.verify_md5 is False


def test_esp_upload_command_uses_rust_owned_options():
    command = esp_upload_command(
        "esp32",
        port="/dev/cu.usbserial-110",
        image="/tmp/board-vm-esp32.bin",
        offset=0x2000,
        verify_md5=False,
        stay_in_bootloader=True,
    )

    assert command == [
        "esp-upload",
        "--port",
        "/dev/cu.usbserial-110",
        "--image",
        "/tmp/board-vm-esp32.bin",
        "--baud",
        "115200",
        "--timeout-ms",
        "1000",
        "--offset",
        "8192",
        "--block-size",
        "1024",
        "--flash-size",
        "4194304",
        "--no-verify",
        "--stay-in-bootloader",
    ]


def test_esp_upload_command_rejects_non_esp_targets():
    try:
        esp_upload_command("pico", port="/dev/cu.usbmodem", image="fw.bin")
    except ValueError as error:
        assert "ESP upload is not supported" in str(error)
    else:
        raise AssertionError("expected ValueError")


def test_devices_are_classified_by_rust_language_core():
    found = devices([
        "/dev/cu.usbmodem1101",
        "/dev/tty.usbserial-CP2102-esp32",
        "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00",
    ])

    assert all(isinstance(device, BoardDevice) for device in found)
    assert found[0].port == "/dev/cu.usbmodem1101"
    assert found[0].target is None
    assert "usb_cdc" in found[0].tags

    esp = next(device for device in found if "usbserial" in device.port)
    assert esp.target is not None
    assert esp.target.board_id == "esp32-devkit-v1"
    assert "uart" in esp.tags

    pico = next(device for device in found if "Raspberry_Pi_Pico" in device.port)
    assert pico.target is not None
    assert pico.target.board_id == "raspberry-pi-pico"
    assert pico.bootloader is True

    rendered = device_list(found)
    assert "ESP32 DevKit V1" in rendered
    assert "/dev/cu.usbmodem1101" in rendered


def test_session_dispatches_frames_through_write_transport():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.blink(program_id=7, instruction_budget=24, handshake=True, query_caps=True)

    assert [item.command for item in result.results] == [
        "hello",
        "capabilities",
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames
    assert all(isinstance(frame, bytes) and frame for frame in result.frames)
    assert result.responses == [None] * 6
    assert result.decoded_responses == [None] * 6


def test_run_command_accepts_repl_style_blink():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("blink 42", program_id=9)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_stop():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("stop")

    assert [item.command for item in result.results] == ["stop"]
    assert result.frames == transport.frames


def test_store_program_dispatches_rust_owned_wire_frame():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.store_program(program_id=9, slot=2, boot_policy="run-at-boot")
    command = session.run_command("store-program 10 3 store-only")

    assert result.command == "store_program"
    assert isinstance(result.frame, bytes)
    assert result.frame == transport.frames[0]
    assert [item.command for item in command.results] == ["store_program"]
    assert command.frames == transport.frames[1:]


def test_run_accepts_configurable_rust_owned_flags():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run(
        program_id=9,
        instruction_budget=42,
        keep_handles=True,
        background=False,
        time_budget_ms=250,
    )

    assert result.command == "run"
    assert isinstance(result.frame, bytes)
    assert result.frame == transport.frames[0]
    assert session.next_request_id == 2


def test_run_command_accepts_repl_style_gpio_read():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("gpio-read 13 pullup 24", program_id=9)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_gpio_write_and_levels():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("gpio-write 13 high 24", program_id=9)
    high = session.run_command("gpio-high 13 24", program_id=10)
    low = session.run_command("gpio-low 13 24", program_id=11)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in high.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in low.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames + high.frames + low.frames == transport.frames


def test_run_command_accepts_repl_style_gpio_handle_commands():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    open_result = session.run_command("gpio-open 13 output 24", program_id=9)
    read = session.run_command("gpio-handle-read 24", program_id=10)
    write = session.run_command("gpio-handle-write high 24", program_id=11)
    close = session.run_command("gpio-handle-close 24", program_id=12)

    assert [item.command for item in open_result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in read.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in write.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in close.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert open_result.frames + read.frames + write.frames + close.frames == transport.frames


def test_run_command_accepts_repl_style_time_now():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("time-now 42", program_id=9)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_time_sleep_ms():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("time-sleep-ms 250 42", program_id=9)
    upload = session.run_command("upload-time-sleep-ms 125", program_id=10)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in upload.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
    ]
    assert result.frames + upload.frames == transport.frames


def test_upload_raw_module_uses_rust_owned_module_builder():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.upload_raw_module(
        program_id=12,
        code=b"\x00",
        max_stack=1,
        const_pool=b"\xAA\x55",
    )

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
    ]
    assert result.frames == transport.frames


def test_board_descriptor_wraps_rust_decoded_capability_payload():
    descriptor = BoardDescriptor(
        {
            "board_id": "arduino-uno-r4-wifi",
            "runtime_id": "board-vm-uno-r4",
            "max_program_bytes": 1024,
            "max_stack_values": 16,
            "max_handles": 4,
            "supports_store_program": False,
            "capabilities": [
                {"id": 1, "version": 1, "flags": 1, "name": "gpio.open"},
                {
                    "id": 0x7001,
                    "version": 1,
                    "flags": 2,
                    "name": "program.ram_exec",
                    "protocol_feature": True,
                    "flag_names": ["protocol_feature"],
                },
            ],
        }
    )

    assert descriptor.board_id == "arduino-uno-r4-wifi"
    assert descriptor.runtime_id == "board-vm-uno-r4"
    assert descriptor.capability_names == ["gpio.open", "program.ram_exec"]
    assert descriptor.supports("gpio.open")
    assert descriptor.supports(0x7001)
    assert descriptor.capability("gpio.open").name == "gpio.open"
    assert descriptor.capability("gpio.open").bytecode_callable
    assert descriptor.capability("program.ram_exec").protocol_feature
    assert descriptor.capability("program.ram_exec").flag_names == ["protocol_feature"]

    result = ProtocolResult(
        command="capabilities",
        frame=b"frame",
        decoded_response={"kind": "caps_report", "payload": descriptor.raw},
    )
    assert result.board_descriptor.capability_names == descriptor.capability_names
