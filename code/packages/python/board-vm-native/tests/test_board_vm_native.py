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
