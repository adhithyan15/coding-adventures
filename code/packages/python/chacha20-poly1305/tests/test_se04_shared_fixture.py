"""Portable SE04 fixture consumed by every D18 language implementation."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from coding_adventures_chacha20_poly1305 import (  # type: ignore[import-untyped]
    hchacha20_subkey,
    xchacha20_encrypt,
    xchacha20_poly1305_aead_decrypt,
    xchacha20_poly1305_aead_encrypt,
)

FIXTURE_PATH = (
    Path(__file__).resolve().parents[4]
    / "specs"
    / "fixtures"
    / "se04-xchacha20-poly1305-v1"
    / "cases.json"
)
FIXTURE: dict[str, Any] = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))


def from_hex(value: str) -> bytes:
    """Decode one closed-fixture byte string."""
    return bytes.fromhex(value)


def test_shared_fixture_metadata() -> None:
    assert FIXTURE["schema_version"] == 1
    assert FIXTURE["profile"] == "se04-xchacha20-poly1305-v1"
    assert FIXTURE["authentication_failure"] == "authentication_failed"
    assert len(FIXTURE["hchacha20_cases"]) == 1
    assert len(FIXTURE["xchacha20_cases"]) == 2
    assert len(FIXTURE["aead_cases"]) == 3
    assert len(FIXTURE["mutations"]) == 5


def test_shared_hchacha20_cases() -> None:
    for case in FIXTURE["hchacha20_cases"]:
        assert hchacha20_subkey(
            from_hex(case["key_hex"]),
            from_hex(case["nonce_hex"]),
        ) == from_hex(case["subkey_hex"]), case["id"]


def test_shared_raw_xchacha20_cases() -> None:
    for case in FIXTURE["xchacha20_cases"]:
        input_bytes = from_hex(case["input_hex"])
        output = xchacha20_encrypt(
            input_bytes,
            from_hex(case["key_hex"]),
            from_hex(case["nonce_hex"]),
            case["counter"],
        )
        assert output == from_hex(case["output_hex"]), case["id"]
        assert (
            xchacha20_encrypt(
                output,
                from_hex(case["key_hex"]),
                from_hex(case["nonce_hex"]),
                case["counter"],
            )
            == input_bytes
        ), case["id"]


def test_shared_aead_cases_encrypt_and_decrypt_byte_identically() -> None:
    for case in FIXTURE["aead_cases"]:
        key = from_hex(case["key_hex"])
        nonce = from_hex(case["nonce_hex"])
        aad = from_hex(case["aad_hex"])
        plaintext = from_hex(case["plaintext_hex"])
        expected_ciphertext = from_hex(case["ciphertext_hex"])
        expected_tag = from_hex(case["tag_hex"])

        assert xchacha20_poly1305_aead_encrypt(
            plaintext, key, nonce, aad,
        ) == (expected_ciphertext, expected_tag), case["id"]
        assert xchacha20_poly1305_aead_decrypt(
            expected_ciphertext, key, nonce, aad, expected_tag,
        ) == plaintext, case["id"]


def test_shared_mutations_have_one_authentication_failure() -> None:
    cases = {case["id"]: case for case in FIXTURE["aead_cases"]}

    for mutation in FIXTURE["mutations"]:
        source = cases[mutation["source_case"]]
        originals = {
            "ciphertext": from_hex(source["ciphertext_hex"]),
            "key": from_hex(source["key_hex"]),
            "nonce": from_hex(source["nonce_hex"]),
            "aad": from_hex(source["aad_hex"]),
            "tag": from_hex(source["tag_hex"]),
        }

        for byte_index in mutation["byte_indices"]:
            changed = dict(originals)
            value = bytearray(changed[mutation["target"]])
            value[byte_index] ^= int(mutation["xor_hex"], 16)
            changed[mutation["target"]] = bytes(value)

            with pytest.raises(ValueError, match="Authentication failed"):
                xchacha20_poly1305_aead_decrypt(
                    changed["ciphertext"],
                    changed["key"],
                    changed["nonce"],
                    changed["aad"],
                    changed["tag"],
                )
