from __future__ import annotations

import pytest

from forme_plugin_runner.wire import FrameDecoder, ProtocolError, decode_wire_value, encode_frame


def test_wire_values_round_trip_bytes_and_reserved_objects() -> None:
    value = {"bytes": bytes([0, 255]), "reserved": {"$forme": "bytes", "base64": "plain"}}
    frame = encode_frame({"jsonrpc": "2.0", "id": 1, "result": value}, 4096)
    decoder = FrameDecoder(4096, 512)
    assert decoder.push(frame[:7]) == []
    assert decoder.push(frame[7:]) == [{"jsonrpc": "2.0", "id": 1, "result": value}]


@pytest.mark.parametrize(
    "payload",
    [
        b"X: 1\r\n\r\n{}",
        b"Content-Length: 01\r\n\r\n{}",
        b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        b"Content-Length: 4\r\n\r\nnull",
    ],
)
def test_decoder_rejects_malformed_frames(payload: bytes) -> None:
    with pytest.raises(ProtocolError):
        FrameDecoder(128, 64).push(payload)


def test_wire_rejects_invalid_envelopes_and_bounds() -> None:
    with pytest.raises(ProtocolError, match="canonical"):
        decode_wire_value({"$forme": "bytes", "base64": "!"})
    with pytest.raises(ProtocolError, match="escaped-object"):
        decode_wire_value({"$forme": "escaped-object", "entries": [["x", 1], ["x", 2]]})
    with pytest.raises(ProtocolError, match="bound"):
        encode_frame({"value": bytes(4096)}, 4096)


def test_wire_preflights_large_values_before_json_or_base64_allocation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def unexpected(*_args: object, **_kwargs: object) -> object:
        raise AssertionError("amplifying encoder was called before the bound check")

    monkeypatch.setattr("forme_plugin_runner.wire.json.dumps", unexpected)
    with pytest.raises(ProtocolError, match="bound"):
        encode_frame({"value": "x" * 4096}, 64)

    monkeypatch.setattr("forme_plugin_runner.wire.base64.b64encode", unexpected)
    with pytest.raises(ProtocolError, match="bound"):
        encode_frame({"value": bytes(4096)}, 64)


def test_wire_serializes_one_bounded_private_snapshot() -> None:
    class ChangingMapping(dict[str, object]):
        calls = 0

        def items(self) -> object:
            self.calls += 1
            if self.calls == 1:
                return {"x": 1}.items()
            return {"x": "x" * 1_000_000}.items()

    changing = ChangingMapping()
    frame = encode_frame({"value": changing}, 128)
    assert changing.calls == 1
    assert b'"value":{"x":1}' in frame
