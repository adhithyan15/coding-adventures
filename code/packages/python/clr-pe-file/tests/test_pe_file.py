from __future__ import annotations

import pytest

from clr_pe_file import decode_clr_pe_file
from clr_pe_file.pe_file import (
    CLRTypeDef,
    CLRTypeReference,
    _parse_method_signature,
    _read_compressed_uint,
    _read_method_body_header,
    _resolve_typedef_or_ref_name,
)
from clr_pe_file.testing import hello_world_dll_bytes


def test_decode_hello_world_clr_assembly() -> None:
    assembly = decode_clr_pe_file(hello_world_dll_bytes())

    entry_point = assembly.get_entry_point_method()

    assert assembly.metadata_version == "v4.0.30319"
    assert assembly.entry_point_token == 0x06000001
    assert entry_point.declaring_type == "Program"
    assert entry_point.name == "Main"
    assert entry_point.signature.return_type == "void"
    assert entry_point.signature.parameter_types == ("string[]",)
    assert entry_point.il_bytes.hex() == "7201000070280d00000a2a"
    assert entry_point.header.format == "tiny"

    member = assembly.resolve_member_reference(0x0A00000D)
    assert member.declaring_type == "System.Console"
    assert member.name == "WriteLine"
    assert member.signature.parameter_types == ("string",)
    assert member.signature.return_type == "void"

    assert assembly.resolve_user_string(0x70000001) == "Hello, world!"


def test_signature_and_header_helpers_cover_common_cases() -> None:
    signature = _parse_method_signature(
        bytes([0x00, 0x01, 0x01, 0x0E]),
        {},
        {},
    )
    assert signature.parameter_types == ("string",)
    assert signature.return_type == "void"

    typedefs = {0x02000001: CLRTypeDef(0x02000001, "Program", ())}
    typerefs = {
        0x01000001: CLRTypeReference(0x01000001, "System.Console", "System.Runtime")
    }
    resolved_type = _resolve_typedef_or_ref_name(0x00000005, typerefs, typedefs)
    assert resolved_type == "System.Console"
    assert _resolve_typedef_or_ref_name(0x00000004, typerefs, typedefs) == "Program"

    tiny_header, tiny_size = _read_method_body_header(bytes([0x2E]), 0)
    assert tiny_header.format == "tiny"
    assert tiny_header.code_size == 11
    assert tiny_size == 1

    fat_bytes = bytes.fromhex("133008000300000011000000")
    fat_header, fat_size = _read_method_body_header(fat_bytes, 0)
    assert fat_header.format == "fat"
    assert fat_header.max_stack == 8
    assert fat_header.code_size == 3
    assert fat_header.local_var_sig_token == 0x00000011
    assert fat_header.init_locals
    assert fat_size == 12


def test_compressed_uint_and_invalid_pe_cases() -> None:
    assert _read_compressed_uint(bytes([0x7F]), 0) == (127, 1)
    assert _read_compressed_uint(bytes([0x80, 0x80]), 0) == (128, 2)
    assert _read_compressed_uint(bytes([0xC0, 0x00, 0x01, 0x00]), 0) == (256, 4)

    with pytest.raises(ValueError, match="Expected PE signature"):
        decode_clr_pe_file(b"not-a-pe")
