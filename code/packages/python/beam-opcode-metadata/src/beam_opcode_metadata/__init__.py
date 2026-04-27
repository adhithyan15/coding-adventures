"""Version-aware BEAM opcode metadata for Erlang/OTP tooling."""

from beam_opcode_metadata.catalog import (
    BEAM_FORMAT_NUMBER,
    OTP_24_PROFILE,
    OTP_28_PROFILE,
    OTP_29_PROFILE,
    BeamOpcode,
    BeamProfile,
    get_profile,
    list_profiles,
)

__all__ = [
    "BEAM_FORMAT_NUMBER",
    "BeamOpcode",
    "BeamProfile",
    "OTP_24_PROFILE",
    "OTP_28_PROFILE",
    "OTP_29_PROFILE",
    "get_profile",
    "list_profiles",
]
