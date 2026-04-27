"""Argon2i test suite — mirrors RFC 9106 §5 and parameter-edge vectors."""

from __future__ import annotations

import pytest

from coding_adventures_argon2i import argon2i, argon2i_hex


# --- RFC 9106 §5.2 canonical test vector -----------------------------------

RFC_PASSWORD = b"\x01" * 32
RFC_SALT = b"\x02" * 16
RFC_KEY = b"\x03" * 8
RFC_AD = b"\x04" * 12
RFC_PARAMS = dict(time_cost=3, memory_cost=32, parallelism=4, tag_length=32)
RFC_EXPECTED = bytes.fromhex(
    "c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8"
)


def test_rfc_9106_vector() -> None:
    """The RFC 9106 §5.2 test vector is the gold standard."""
    tag = argon2i(
        RFC_PASSWORD, RFC_SALT,
        key=RFC_KEY, associated_data=RFC_AD,
        **RFC_PARAMS,
    )
    assert tag == RFC_EXPECTED


def test_hex_form_matches_bytes_form() -> None:
    tag = argon2i(RFC_PASSWORD, RFC_SALT, **RFC_PARAMS)
    tag_hex = argon2i_hex(RFC_PASSWORD, RFC_SALT, **RFC_PARAMS)
    assert tag_hex == tag.hex()


# --- Parameter validation ---------------------------------------------------


def test_short_salt_rejected() -> None:
    with pytest.raises(ValueError, match="salt"):
        argon2i(b"pw", b"short", 1, 8, 1, 32)


def test_tag_length_too_small_rejected() -> None:
    with pytest.raises(ValueError, match="tag_length"):
        argon2i(b"pw", b"saltsalt", 1, 8, 1, 3)


def test_memory_below_minimum_rejected() -> None:
    with pytest.raises(ValueError, match="memory_cost"):
        argon2i(b"pw", b"saltsalt", 1, 1, 1, 32)


def test_zero_time_cost_rejected() -> None:
    with pytest.raises(ValueError, match="time_cost"):
        argon2i(b"pw", b"saltsalt", 0, 8, 1, 32)


def test_zero_parallelism_rejected() -> None:
    with pytest.raises(ValueError, match="parallelism"):
        argon2i(b"pw", b"saltsalt", 1, 8, 0, 32)


def test_unsupported_version_rejected() -> None:
    with pytest.raises(ValueError, match="Argon2 v1.3"):
        argon2i(b"pw", b"saltsalt", 1, 8, 1, 32, version=0x10)


def test_non_bytes_password_rejected() -> None:
    with pytest.raises(TypeError, match="password"):
        argon2i("not bytes", b"saltsalt", 1, 8, 1, 32)  # type: ignore[arg-type]


# --- Small deterministic sanity checks -------------------------------------
#
# These vectors are for behaviour parity: if you change the
# implementation, verify the byte values stay stable.  They will
# additionally be cross-validated against the PHC reference
# implementation once the shared KAT file lands.


def test_single_lane_minimum_memory_deterministic() -> None:
    tag_a = argon2i(b"password", b"somesalt", 1, 8, 1, 32)
    tag_b = argon2i(b"password", b"somesalt", 1, 8, 1, 32)
    assert tag_a == tag_b
    assert len(tag_a) == 32


def test_different_passwords_differ() -> None:
    tag_a = argon2i(b"password1", b"somesalt", 1, 8, 1, 32)
    tag_b = argon2i(b"password2", b"somesalt", 1, 8, 1, 32)
    assert tag_a != tag_b


def test_different_salts_differ() -> None:
    tag_a = argon2i(b"password", b"saltsalt", 1, 8, 1, 32)
    tag_b = argon2i(b"password", b"saltsal2", 1, 8, 1, 32)
    assert tag_a != tag_b


def test_key_binds_output() -> None:
    tag_a = argon2i(b"password", b"saltsalt", 1, 8, 1, 32)
    tag_b = argon2i(b"password", b"saltsalt", 1, 8, 1, 32, key=b"secret!!")
    assert tag_a != tag_b


def test_associated_data_binds_output() -> None:
    tag_a = argon2i(b"password", b"saltsalt", 1, 8, 1, 32)
    tag_b = argon2i(
        b"password", b"saltsalt", 1, 8, 1, 32, associated_data=b"ad",
    )
    assert tag_a != tag_b


# --- Tag-length variability ------------------------------------------------


@pytest.mark.parametrize("T", [4, 16, 32, 64, 65, 128])
def test_tag_length_variants(T: int) -> None:
    tag = argon2i(b"password", b"saltsalt", 1, 8, 1, T)
    assert len(tag) == T


# --- Multi-lane / parameter edges ------------------------------------------


def test_multi_lane_parameters() -> None:
    # p=4, m=32: matches the RFC vector shape but with empty key/AD.
    tag = argon2i(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32)
    assert len(tag) == 32


def test_multiple_passes() -> None:
    tag_t1 = argon2i(b"password", b"saltsalt", 1, 8, 1, 32)
    tag_t2 = argon2i(b"password", b"saltsalt", 2, 8, 1, 32)
    tag_t3 = argon2i(b"password", b"saltsalt", 3, 8, 1, 32)
    assert len({tag_t1, tag_t2, tag_t3}) == 3
