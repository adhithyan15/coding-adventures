"""Direct CR03 Vigenere conformance regressions."""

import pytest

from vigenere_cipher import decrypt, encrypt, find_key, find_key_length


def test_ascii_transform_key_validation_and_analysis() -> None:
    plaintext = "Hello, 😀Wörld!"
    ciphertext = "Rijvs, 😀Uöbpb!"
    assert encrypt(plaintext, "kEy") == ciphertext
    assert decrypt(ciphertext, "kEy") == plaintext
    with pytest.raises(ValueError):
        decrypt("", "KÉY")
    assert find_key_length("AéA😀AЖAéABB", 4) == 2
    assert find_key("Eé😀Ж", 40) == "A" * 40


def test_analysis_limits_and_preflight_order() -> None:
    at_limit = "😀" * 8192
    over_limit = at_limit + "😀"
    assert find_key_length(at_limit, 40) == 1
    with pytest.raises(ValueError):
        find_key_length(over_limit, 20)
    with pytest.raises(ValueError):
        find_key_length(over_limit, 41)
    assert find_key(over_limit, 0) == ""
    with pytest.raises(ValueError):
        find_key(over_limit, 1)
    with pytest.raises(ValueError):
        find_key(over_limit, 41)
