from board_vm_native import BoardDescriptor, ProtocolResult, Session


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
