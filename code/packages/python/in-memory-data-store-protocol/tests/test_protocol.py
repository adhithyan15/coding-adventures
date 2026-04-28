from in_memory_data_store_protocol import CommandFrame, EngineResponse, ascii_upper


def test_ascii_upper_normalizes_bytes() -> None:
    assert ascii_upper(b"get") == "GET"
    assert ascii_upper(bytearray(b"mset")) == "MSET"
    assert ascii_upper(memoryview(b"ping")) == "PING"


def test_command_frame_from_parts() -> None:
    frame = CommandFrame.from_parts([b"set", b"key", b"value"])

    assert frame == CommandFrame("SET", (b"key", b"value"))
    assert frame.to_parts() == [b"SET", b"key", b"value"]
    assert CommandFrame.from_parts([]) is None


def test_command_frame_new_copies_args() -> None:
    frame = CommandFrame.new("GET", [bytearray(b"key")])  # type: ignore[list-item]

    assert frame.command == "GET"
    assert frame.args == (b"key",)
    assert frame.to_parts() == [b"GET", b"key"]


def test_engine_response_constructors() -> None:
    assert EngineResponse.simple_string("PONG") == EngineResponse("simple_string", "PONG")
    assert EngineResponse.error("ERR") == EngineResponse("error", "ERR")
    assert EngineResponse.integer(42) == EngineResponse("integer", 42)
    assert EngineResponse.bulk_string(b"value") == EngineResponse("bulk_string", b"value")
    assert EngineResponse.bulk_string(bytearray(b"value")) == EngineResponse("bulk_string", b"value")
    assert EngineResponse.null() == EngineResponse("bulk_string", None)
    assert EngineResponse.ok() == EngineResponse("simple_string", "OK")
    assert EngineResponse.zero() == EngineResponse("integer", 0)
    assert EngineResponse.one() == EngineResponse("integer", 1)


def test_engine_response_arrays() -> None:
    response = EngineResponse.array([EngineResponse.ok(), EngineResponse.integer(3)])

    assert response.kind == "array"
    assert response.value == (EngineResponse.ok(), EngineResponse.integer(3))
    assert EngineResponse.array(None) == EngineResponse("array", None)
