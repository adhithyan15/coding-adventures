"""Language-neutral portable conformance for generic X.509 Extension values."""

from __future__ import annotations

import json
import sys
from dataclasses import FrozenInstanceError
from pathlib import Path
from typing import Any, cast

import pytest
from der_asn1 import Asn1Decoder, Asn1Element, Asn1Limits, ObjectIdentifier
from der_tlv import DerLimits

from x509_extension import X509Extension, X509ExtensionError, decode_x509_extension

FIXTURE_ROOT = Path(__file__).resolve().parents[4] / "specs/fixtures"
FIXTURE = FIXTURE_ROOT / "x509-extension-v1/cases.json"
UPSTREAM = FIXTURE_ROOT / "der-asn1-v1/cases.json"


def load(path: Path) -> dict[str, Any]:
    return cast(dict[str, Any], json.loads(path.read_text("utf-8")))


def materialize(segments: list[dict[str, Any]]) -> bytes:
    result = bytearray()
    for segment in segments:
        if "hex" in segment:
            result.extend(bytes.fromhex(segment["hex"]))
        else:
            result.extend(bytes.fromhex(segment["repeat_hex"]) * segment["count"])
    return bytes(result)


def configured_limits(upstream: dict[str, Any], case: dict[str, Any]) -> Asn1Limits:
    values = json.loads(json.dumps(upstream["defaults"]))
    override = case.get("limits", {})
    values.update({key: value for key, value in override.items() if key != "der"})
    values["der"].update(override.get("der", {}))
    if values["der"]["max_value_len"] == "host-max":
        values["der"]["max_value_len"] = sys.maxsize
    return Asn1Limits(der=DerLimits(**values.pop("der")), **values)


def attempt(decoder: Asn1Decoder, root: Asn1Element) -> dict[str, Any]:
    try:
        extension = decode_x509_extension(decoder, root)
        return {
            "outcome": "value",
            "extension_id_arcs_decimal": [
                str(arc) for arc in extension.extension_id.arcs
            ],
            "critical": extension.critical,
            "extension_value_hex": bytes(extension.extension_value).hex(),
            "elements_read": decoder.elements_read,
        }
    except X509ExtensionError as error:
        result: dict[str, Any] = {
            "outcome": "error",
            "error_id": error.kind.value,
            "offset": error.offset,
            "offset_scope": "extension-element",
            "elements_read": decoder.elements_read,
        }
        if error.asn1_kind is not None:
            result["asn1_error_id"] = error.asn1_kind.value
        if error.framing_kind is not None:
            result["framing_error_id"] = error.framing_kind.value
        return result


def test_consumes_every_closed_x509_extension_case() -> None:
    document = load(FIXTURE)
    upstream = load(UPSTREAM)
    assert len(document["cases"]) == 48
    for case in document["cases"]:
        decoder = Asn1Decoder(configured_limits(upstream, case))
        root = decoder.decode_exact(materialize(case["input"]))
        if case["operation"] == "extension-script":
            actual = {
                "outcome": "script",
                "events": [attempt(decoder, root) for _ in case["actions"]],
            }
        else:
            actual = attempt(decoder, root)
        assert actual == case["expected"], case["id"]
        hostile = case.get("redacted_input_hex")
        if hostile is not None:
            assert hostile not in json.dumps(actual)


def test_error_text_is_payload_blind() -> None:
    decoder = Asn1Decoder()
    root = decoder.decode_exact(bytes.fromhex("30080601800403deadbe"))
    with pytest.raises(X509ExtensionError) as caught:
        decode_x509_extension(decoder, root)
    assert "deadbe" not in str(caught.value).lower()
    assert "deadbe" not in repr(caught.value).lower()


def test_values_are_private_detached_and_immutable() -> None:
    source = bytearray.fromhex("30090603551d1104023000")
    decoder = Asn1Decoder()
    root = decoder.decode_exact(source)
    extension = decode_x509_extension(decoder, root)

    source[4] = 0x2A
    source[-1] = 0xFF
    assert extension.extension_value == bytes.fromhex("3000")
    assert extension.extension_id.encoded == bytes.fromhex("551d11")
    assert isinstance(extension.extension_value, bytes)
    assert isinstance(extension.extension_id.encoded, bytes)

    with pytest.raises(TypeError, match="created by decode_x509_extension"):
        X509Extension(extension.extension_id, False, b"")  # type: ignore[call-arg]
    with pytest.raises(TypeError, match="created by DER decoding"):
        ObjectIdentifier(b"\x55\x1d\x11", (2, 5, 29, 17))  # type: ignore[call-arg]
    with pytest.raises(FrozenInstanceError):
        extension.critical = True  # type: ignore[misc]
