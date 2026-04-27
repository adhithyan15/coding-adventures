from beam_opcode_metadata import (
    OTP_24_PROFILE,
    OTP_28_PROFILE,
    OTP_29_PROFILE,
    get_profile,
)


def test_profile_opcode_boundaries() -> None:
    assert OTP_24_PROFILE.max_external_opcode == 176
    assert OTP_28_PROFILE.max_external_opcode == 184
    assert OTP_29_PROFILE.max_external_opcode == 191
    assert OTP_28_PROFILE.opcode_by_number(184).name == "debug_line"
    assert OTP_29_PROFILE.opcode_by_number(191).name == "get_record_field"


def test_profile_name_lookup() -> None:
    profile = get_profile("otp28")
    assert profile.opcode_by_name("move").number == 64
    assert profile.supports_opcode(64) is True
    assert profile.supports_opcode(185) is False
