"""Portable Executable decoder for CLR assemblies.

This module parses the parts of a .NET assembly that are needed to load and
execute CIL today while staying reusable for a fuller CLR later:

- PE headers and section layout
- CLI metadata root and streams
- key metadata tables such as TypeRef, TypeDef, MethodDef, MemberRef
- user strings and blob signatures
- method bodies located by RVA
"""

from __future__ import annotations

import struct
from collections.abc import Callable
from dataclasses import dataclass

USER_STRING_TOKEN_PREFIX = 0x70000000
METHOD_DEF_TOKEN_PREFIX = 0x06000000
MEMBER_REF_TOKEN_PREFIX = 0x0A000000
TYPE_REF_TOKEN_PREFIX = 0x01000000
TYPE_DEF_TOKEN_PREFIX = 0x02000000
ASSEMBLY_REF_TOKEN_PREFIX = 0x23000000
STANDALONE_SIG_TOKEN_PREFIX = 0x11000000

MODULE_TABLE = 0x00
TYPE_REF_TABLE = 0x01
TYPE_DEF_TABLE = 0x02
FIELD_TABLE = 0x04
METHOD_DEF_TABLE = 0x06
PARAM_TABLE = 0x08
MEMBER_REF_TABLE = 0x0A
STANDALONE_SIG_TABLE = 0x11
ASSEMBLY_TABLE = 0x20
ASSEMBLY_REF_TABLE = 0x23
TYPE_SPEC_TABLE = 0x1B

ELEMENT_TYPE_VOID = 0x01
ELEMENT_TYPE_BOOLEAN = 0x02
ELEMENT_TYPE_CHAR = 0x03
ELEMENT_TYPE_I1 = 0x04
ELEMENT_TYPE_U1 = 0x05
ELEMENT_TYPE_I2 = 0x06
ELEMENT_TYPE_U2 = 0x07
ELEMENT_TYPE_I4 = 0x08
ELEMENT_TYPE_U4 = 0x09
ELEMENT_TYPE_I8 = 0x0A
ELEMENT_TYPE_U8 = 0x0B
ELEMENT_TYPE_R4 = 0x0C
ELEMENT_TYPE_R8 = 0x0D
ELEMENT_TYPE_STRING = 0x0E
ELEMENT_TYPE_VALUETYPE = 0x11
ELEMENT_TYPE_CLASS = 0x12
ELEMENT_TYPE_OBJECT = 0x1C
ELEMENT_TYPE_SZARRAY = 0x1D

LOCAL_SIG = 0x07


@dataclass(frozen=True)
class CLRMethodSignature:
    """Minimal decoded representation of a CLR method signature."""

    has_this: bool
    parameter_types: tuple[str, ...]
    return_type: str


@dataclass(frozen=True)
class CLRMethodBodyHeader:
    """Decoded CLR method body header."""

    format: str
    max_stack: int
    code_size: int
    local_var_sig_token: int
    init_locals: bool


@dataclass(frozen=True)
class CLRTypeReference:
    """Decoded TypeRef row."""

    token: int
    full_name: str
    resolution_scope_name: str | None


@dataclass(frozen=True)
class CLRTypeDef:
    """Decoded TypeDef row."""

    token: int
    full_name: str
    method_tokens: tuple[int, ...]


@dataclass(frozen=True)
class CLRAssemblyRef:
    """Decoded AssemblyRef row."""

    token: int
    name: str


@dataclass(frozen=True)
class CLRMemberReference:
    """Decoded MemberRef row for external or metadata-based calls."""

    token: int
    declaring_type: str
    name: str
    signature: CLRMethodSignature


@dataclass(frozen=True)
class CLRMethodDef:
    """Decoded MethodDef row and extracted IL body."""

    token: int
    name: str
    declaring_type: str
    signature: CLRMethodSignature
    rva: int
    header: CLRMethodBodyHeader
    il_bytes: bytes
    local_count: int


@dataclass(frozen=True)
class CLRPEFile:
    """Version-aware CLR assembly representation."""

    metadata_version: str
    entry_point_token: int
    type_references: tuple[CLRTypeReference, ...]
    type_definitions: tuple[CLRTypeDef, ...]
    assembly_references: tuple[CLRAssemblyRef, ...]
    member_references: tuple[CLRMemberReference, ...]
    method_definitions: tuple[CLRMethodDef, ...]
    _user_strings: dict[int, str]
    _method_by_token: dict[int, CLRMethodDef]
    _member_ref_by_token: dict[int, CLRMemberReference]

    def resolve_user_string(self, token: int) -> str:
        """Resolve a UserString heap token."""
        if token == 0:
            return ""
        if token & 0xFF000000 != USER_STRING_TOKEN_PREFIX:
            msg = f"Token {token:#010x} is not a UserString token"
            raise ValueError(msg)
        if token not in self._user_strings:
            msg = f"Unknown UserString token {token:#010x}"
            raise KeyError(msg)
        return self._user_strings[token]

    def resolve_method_definition(self, token: int) -> CLRMethodDef:
        """Resolve a MethodDef token."""
        if token not in self._method_by_token:
            msg = f"Unknown MethodDef token {token:#010x}"
            raise KeyError(msg)
        return self._method_by_token[token]

    def resolve_member_reference(self, token: int) -> CLRMemberReference:
        """Resolve a MemberRef token."""
        if token not in self._member_ref_by_token:
            msg = f"Unknown MemberRef token {token:#010x}"
            raise KeyError(msg)
        return self._member_ref_by_token[token]

    def get_entry_point_method(self) -> CLRMethodDef:
        """Return the method marked as the assembly entry point."""
        return self.resolve_method_definition(self.entry_point_token)


@dataclass(frozen=True)
class _Section:
    name: str
    virtual_address: int
    virtual_size: int
    raw_pointer: int
    raw_size: int


@dataclass(frozen=True)
class _TypeRefRow:
    resolution_scope: int
    type_name: str
    type_namespace: str


@dataclass(frozen=True)
class _TypeDefRow:
    type_name: str
    type_namespace: str
    method_list: int


@dataclass(frozen=True)
class _MethodDefRow:
    rva: int
    name: str
    signature_index: int


@dataclass(frozen=True)
class _MemberRefRow:
    parent: int
    name: str
    signature_index: int


def decode_clr_pe_file(data: bytes) -> CLRPEFile:
    """Decode a CLR Portable Executable assembly."""
    if len(data) < 0x40:
        msg = "Expected PE signature in CLR assembly"
        raise ValueError(msg)
    pe_offset = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe_offset : pe_offset + 4] != b"PE\x00\x00":
        msg = "Expected PE signature in CLR assembly"
        raise ValueError(msg)

    coff_offset = pe_offset + 4
    number_of_sections = struct.unpack_from("<H", data, coff_offset + 2)[0]
    optional_header_size = struct.unpack_from("<H", data, coff_offset + 16)[0]
    optional_offset = coff_offset + 20
    optional_magic = struct.unpack_from("<H", data, optional_offset)[0]
    if optional_magic == 0x10B:
        data_directory_offset = optional_offset + 96
    elif optional_magic == 0x20B:
        data_directory_offset = optional_offset + 112
    else:
        msg = f"Unsupported PE optional header magic {optional_magic:#x}"
        raise ValueError(msg)

    cli_directory_offset = data_directory_offset + (14 * 8)
    cli_header_rva, _ = struct.unpack_from("<II", data, cli_directory_offset)

    section_table_offset = optional_offset + optional_header_size
    sections: list[_Section] = []
    for index in range(number_of_sections):
        offset = section_table_offset + (40 * index)
        raw_name = data[offset : offset + 8]
        name = raw_name.split(b"\x00", 1)[0].decode("ascii")
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from(
            "<IIII", data, offset + 8
        )
        sections.append(
            _Section(
                name=name,
                virtual_address=virtual_address,
                virtual_size=virtual_size,
                raw_pointer=raw_pointer,
                raw_size=raw_size,
            )
        )

    def rva_to_offset(rva: int) -> int:
        for section in sections:
            end = section.virtual_address + max(section.virtual_size, section.raw_size)
            if section.virtual_address <= rva < end:
                return section.raw_pointer + (rva - section.virtual_address)
        msg = f"RVA {rva:#x} does not map to any PE section"
        raise ValueError(msg)

    cli_header_offset = rva_to_offset(cli_header_rva)
    metadata_rva = struct.unpack_from("<I", data, cli_header_offset + 8)[0]
    entry_point_token = struct.unpack_from("<I", data, cli_header_offset + 20)[0]

    metadata_offset = rva_to_offset(metadata_rva)
    if data[metadata_offset : metadata_offset + 4] != b"BSJB":
        msg = "Expected CLR metadata root signature"
        raise ValueError(msg)

    version_length = struct.unpack_from("<I", data, metadata_offset + 12)[0]
    version_start = metadata_offset + 16
    version_end = version_start + version_length
    metadata_version = data[version_start:version_end].rstrip(b"\x00").decode("ascii")
    stream_count = struct.unpack_from("<H", data, version_end + 2)[0]
    stream_header_offset = version_end + 4
    streams: dict[str, bytes] = {}
    for _ in range(stream_count):
        stream_offset, stream_size = struct.unpack_from(
            "<II",
            data,
            stream_header_offset,
        )
        name_start = stream_header_offset + 8
        name_end = data.index(b"\x00", name_start)
        stream_name = data[name_start:name_end].decode("ascii")
        stream_header_offset = _align(name_end + 1, 4)
        start = metadata_offset + stream_offset
        streams[stream_name] = data[start : start + stream_size]

    tables_stream = streams.get("#~") or streams.get("#-")
    if tables_stream is None:
        msg = "CLR metadata tables stream (#~ or #-) not found"
        raise ValueError(msg)

    strings_heap = streams.get("#Strings", b"")
    blob_heap = streams.get("#Blob", b"")
    user_string_heap = streams.get("#US", b"")

    row_counts, table_offsets, index_sizes = _parse_table_layout(tables_stream)

    type_ref_rows = _parse_type_ref_rows(
        tables_stream,
        table_offsets,
        row_counts,
        index_sizes,
        strings_heap,
    )
    type_def_rows = _parse_type_def_rows(
        tables_stream,
        table_offsets,
        row_counts,
        index_sizes,
        strings_heap,
    )
    method_def_rows = _parse_method_def_rows(
        tables_stream,
        table_offsets,
        row_counts,
        index_sizes,
        strings_heap,
    )
    member_ref_rows = _parse_member_ref_rows(
        tables_stream,
        table_offsets,
        row_counts,
        index_sizes,
        strings_heap,
    )
    assembly_refs = _parse_assembly_ref_rows(
        tables_stream,
        table_offsets,
        row_counts,
        index_sizes,
        strings_heap,
    )

    assembly_ref_names = {
        ASSEMBLY_REF_TOKEN_PREFIX | row_number: name
        for row_number, name in assembly_refs.items()
    }
    type_references = _build_type_references(type_ref_rows, assembly_ref_names)
    type_definitions, method_owner_names = _build_type_definitions(
        type_def_rows,
        row_counts.get(METHOD_DEF_TABLE, 0),
    )

    member_references = _build_member_references(
        member_ref_rows,
        blob_heap,
        type_references,
        type_definitions,
    )
    method_definitions = _build_method_definitions(
        data,
        blob_heap,
        tables_stream,
        table_offsets,
        row_counts,
        index_sizes["blob"],
        method_def_rows,
        method_owner_names,
        rva_to_offset,
        type_references,
        type_definitions,
    )
    user_strings = _read_user_strings(user_string_heap)

    return CLRPEFile(
        metadata_version=metadata_version,
        entry_point_token=entry_point_token,
        type_references=tuple(type_references.values()),
        type_definitions=tuple(type_definitions.values()),
        assembly_references=tuple(
            CLRAssemblyRef(token=token, name=name)
            for token, name in assembly_ref_names.items()
        ),
        member_references=tuple(member_references.values()),
        method_definitions=tuple(method_definitions.values()),
        _user_strings=user_strings,
        _method_by_token=method_definitions,
        _member_ref_by_token=member_references,
    )


def _parse_table_layout(
    tables_stream: bytes,
) -> tuple[dict[int, int], dict[int, int], dict[str, int]]:
    heap_sizes = tables_stream[6]
    valid_mask = struct.unpack_from("<Q", tables_stream, 8)[0]
    offset = 24
    present_tables = [table for table in range(64) if valid_mask & (1 << table)]

    row_counts: dict[int, int] = {}
    for table in present_tables:
        row_counts[table] = struct.unpack_from("<I", tables_stream, offset)[0]
        offset += 4

    string_index_size = 4 if heap_sizes & 0x01 else 2
    guid_index_size = 4 if heap_sizes & 0x02 else 2
    blob_index_size = 4 if heap_sizes & 0x04 else 2

    table_offsets: dict[int, int] = {}
    cursor = offset
    for table in present_tables:
        table_offsets[table] = cursor
        cursor += _table_row_size(
            table,
            row_counts,
            string_index_size,
            guid_index_size,
            blob_index_size,
        ) * row_counts[table]

    return row_counts, table_offsets, {
        "string": string_index_size,
        "guid": guid_index_size,
        "blob": blob_index_size,
    }


def _parse_type_ref_rows(
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    index_sizes: dict[str, int],
    strings_heap: bytes,
) -> dict[int, _TypeRefRow]:
    row_count = row_counts.get(TYPE_REF_TABLE, 0)
    if row_count == 0:
        return {}
    row_size = _table_row_size(
        TYPE_REF_TABLE,
        row_counts,
        index_sizes["string"],
        index_sizes["guid"],
        index_sizes["blob"],
    )
    scope_size = _coded_index_size("ResolutionScope", row_counts)
    rows: dict[int, _TypeRefRow] = {}
    for row_number in range(1, row_count + 1):
        offset = table_offsets[TYPE_REF_TABLE] + ((row_number - 1) * row_size)
        resolution_scope = _read_index(tables_stream, offset, scope_size)
        cursor = offset + scope_size
        name_index = _read_index(tables_stream, cursor, index_sizes["string"])
        cursor += index_sizes["string"]
        namespace_index = _read_index(tables_stream, cursor, index_sizes["string"])
        rows[row_number] = _TypeRefRow(
            resolution_scope=resolution_scope,
            type_name=_read_string(strings_heap, name_index),
            type_namespace=_read_string(strings_heap, namespace_index),
        )
    return rows


def _parse_type_def_rows(
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    index_sizes: dict[str, int],
    strings_heap: bytes,
) -> dict[int, _TypeDefRow]:
    row_count = row_counts.get(TYPE_DEF_TABLE, 0)
    if row_count == 0:
        return {}
    row_size = _table_row_size(
        TYPE_DEF_TABLE,
        row_counts,
        index_sizes["string"],
        index_sizes["guid"],
        index_sizes["blob"],
    )
    field_index_size = _simple_index_size(row_counts.get(FIELD_TABLE, 0))
    method_index_size = _simple_index_size(row_counts.get(METHOD_DEF_TABLE, 0))
    extends_size = _coded_index_size("TypeDefOrRef", row_counts)
    rows: dict[int, _TypeDefRow] = {}
    for row_number in range(1, row_count + 1):
        offset = table_offsets[TYPE_DEF_TABLE] + ((row_number - 1) * row_size)
        cursor = offset + 4
        name_index = _read_index(tables_stream, cursor, index_sizes["string"])
        cursor += index_sizes["string"]
        namespace_index = _read_index(tables_stream, cursor, index_sizes["string"])
        cursor += index_sizes["string"] + extends_size + field_index_size
        method_list = _read_index(tables_stream, cursor, method_index_size)
        rows[row_number] = _TypeDefRow(
            type_name=_read_string(strings_heap, name_index),
            type_namespace=_read_string(strings_heap, namespace_index),
            method_list=method_list,
        )
    return rows


def _parse_method_def_rows(
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    index_sizes: dict[str, int],
    strings_heap: bytes,
) -> dict[int, _MethodDefRow]:
    row_count = row_counts.get(METHOD_DEF_TABLE, 0)
    if row_count == 0:
        return {}
    row_size = _table_row_size(
        METHOD_DEF_TABLE,
        row_counts,
        index_sizes["string"],
        index_sizes["guid"],
        index_sizes["blob"],
    )
    rows: dict[int, _MethodDefRow] = {}
    for row_number in range(1, row_count + 1):
        offset = table_offsets[METHOD_DEF_TABLE] + ((row_number - 1) * row_size)
        rva = struct.unpack_from("<I", tables_stream, offset)[0]
        name_index = _read_index(tables_stream, offset + 8, index_sizes["string"])
        signature_index = _read_index(
            tables_stream,
            offset + 8 + index_sizes["string"],
            index_sizes["blob"],
        )
        rows[row_number] = _MethodDefRow(
            rva=rva,
            name=_read_string(strings_heap, name_index),
            signature_index=signature_index,
        )
    return rows


def _parse_member_ref_rows(
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    index_sizes: dict[str, int],
    strings_heap: bytes,
) -> dict[int, _MemberRefRow]:
    row_count = row_counts.get(MEMBER_REF_TABLE, 0)
    if row_count == 0:
        return {}
    row_size = _table_row_size(
        MEMBER_REF_TABLE,
        row_counts,
        index_sizes["string"],
        index_sizes["guid"],
        index_sizes["blob"],
    )
    parent_size = _coded_index_size("MemberRefParent", row_counts)
    rows: dict[int, _MemberRefRow] = {}
    for row_number in range(1, row_count + 1):
        offset = table_offsets[MEMBER_REF_TABLE] + ((row_number - 1) * row_size)
        parent = _read_index(tables_stream, offset, parent_size)
        cursor = offset + parent_size
        name_index = _read_index(tables_stream, cursor, index_sizes["string"])
        cursor += index_sizes["string"]
        signature_index = _read_index(tables_stream, cursor, index_sizes["blob"])
        rows[row_number] = _MemberRefRow(
            parent=parent,
            name=_read_string(strings_heap, name_index),
            signature_index=signature_index,
        )
    return rows


def _parse_assembly_ref_rows(
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    index_sizes: dict[str, int],
    strings_heap: bytes,
) -> dict[int, str]:
    row_count = row_counts.get(ASSEMBLY_REF_TABLE, 0)
    if row_count == 0:
        return {}
    row_size = _table_row_size(
        ASSEMBLY_REF_TABLE,
        row_counts,
        index_sizes["string"],
        index_sizes["guid"],
        index_sizes["blob"],
    )
    rows: dict[int, str] = {}
    for row_number in range(1, row_count + 1):
        offset = table_offsets[ASSEMBLY_REF_TABLE] + ((row_number - 1) * row_size)
        cursor = offset + 12 + index_sizes["blob"]
        name_index = _read_index(tables_stream, cursor, index_sizes["string"])
        rows[row_number] = _read_string(strings_heap, name_index)
    return rows


def _build_type_references(
    rows: dict[int, _TypeRefRow],
    assembly_ref_names: dict[int, str],
) -> dict[int, CLRTypeReference]:
    built: dict[int, CLRTypeReference] = {}
    for row_number, row in rows.items():
        token = TYPE_REF_TOKEN_PREFIX | row_number
        full_name = _join_type_name(row.type_namespace, row.type_name)
        scope_name = _resolve_resolution_scope_name(
            row.resolution_scope,
            rows,
            assembly_ref_names,
        )
        built[token] = CLRTypeReference(
            token=token,
            full_name=full_name,
            resolution_scope_name=scope_name,
        )
    return built


def _build_type_definitions(
    rows: dict[int, _TypeDefRow],
    method_row_count: int,
) -> tuple[dict[int, CLRTypeDef], dict[int, str]]:
    built: dict[int, CLRTypeDef] = {}
    method_owner_names: dict[int, str] = {}
    ordered_rows = list(rows.items())
    for index, (row_number, row) in enumerate(ordered_rows):
        next_method_start = (
            ordered_rows[index + 1][1].method_list
            if index + 1 < len(ordered_rows)
            else method_row_count + 1
        )
        method_tokens = tuple(
            METHOD_DEF_TOKEN_PREFIX | method_row
            for method_row in range(row.method_list, next_method_start)
            if method_row != 0
        )
        full_name = _join_type_name(row.type_namespace, row.type_name)
        token = TYPE_DEF_TOKEN_PREFIX | row_number
        built[token] = CLRTypeDef(
            token=token,
            full_name=full_name,
            method_tokens=method_tokens,
        )
        for method_token in method_tokens:
            method_owner_names[method_token] = full_name
    return built, method_owner_names


def _build_member_references(
    rows: dict[int, _MemberRefRow],
    blob_heap: bytes,
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> dict[int, CLRMemberReference]:
    built: dict[int, CLRMemberReference] = {}
    for row_number, row in rows.items():
        token = MEMBER_REF_TOKEN_PREFIX | row_number
        blob = _read_blob(blob_heap, row.signature_index)
        if blob and blob[0] == 0x06:
            continue
        declaring_type = _resolve_member_ref_parent_name(
            row.parent,
            type_references,
            type_definitions,
        )
        signature = _parse_method_signature(
            blob,
            type_references,
            type_definitions,
        )
        built[token] = CLRMemberReference(
            token=token,
            declaring_type=declaring_type,
            name=row.name,
            signature=signature,
        )
    return built


def _build_method_definitions(
    data: bytes,
    blob_heap: bytes,
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    blob_index_size: int,
    rows: dict[int, _MethodDefRow],
    method_owner_names: dict[int, str],
    rva_to_offset: Callable[[int], int],
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> dict[int, CLRMethodDef]:
    built: dict[int, CLRMethodDef] = {}
    local_counts = _read_standalone_signature_local_counts(
        tables_stream,
        table_offsets,
        row_counts,
        blob_heap,
        blob_index_size,
        type_references,
        type_definitions,
    )
    for row_number, row in rows.items():
        token = METHOD_DEF_TOKEN_PREFIX | row_number
        body_offset = rva_to_offset(row.rva)
        header, header_size = _read_method_body_header(data, body_offset)
        il_start = body_offset + header_size
        il_end = il_start + header.code_size
        signature = _parse_method_signature(
            _read_blob(blob_heap, row.signature_index),
            type_references,
            type_definitions,
        )
        built[token] = CLRMethodDef(
            token=token,
            name=row.name,
            declaring_type=method_owner_names.get(token, ""),
            signature=signature,
            rva=row.rva,
            header=header,
            il_bytes=data[il_start:il_end],
            local_count=local_counts.get(header.local_var_sig_token, 0),
        )
    return built


def _read_standalone_signature_local_counts(
    tables_stream: bytes,
    table_offsets: dict[int, int],
    row_counts: dict[int, int],
    blob_heap: bytes,
    blob_index_size: int,
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> dict[int, int]:
    row_count = row_counts.get(STANDALONE_SIG_TABLE, 0)
    if row_count == 0:
        return {}
    row_size = _table_row_size(
        STANDALONE_SIG_TABLE,
        row_counts,
        2,
        2,
        blob_index_size,
    )
    counts: dict[int, int] = {}
    for row_number in range(1, row_count + 1):
        offset = table_offsets[STANDALONE_SIG_TABLE] + ((row_number - 1) * row_size)
        blob_index = _read_index(tables_stream, offset, blob_index_size)
        blob = _read_blob(blob_heap, blob_index)
        if not blob or blob[0] != LOCAL_SIG:
            continue
        local_count, read_bytes = _read_compressed_uint(blob, 1)
        cursor = 1 + read_bytes
        for _ in range(local_count):
            _, cursor = _parse_type_name(
                blob,
                cursor,
                type_references,
                type_definitions,
            )
        counts[STANDALONE_SIG_TOKEN_PREFIX | row_number] = local_count
    return counts


def _read_method_body_header(
    data: bytes,
    offset: int,
) -> tuple[CLRMethodBodyHeader, int]:
    first_byte = data[offset]
    format_bits = first_byte & 0x3
    if format_bits == 0x2:
        return CLRMethodBodyHeader(
            format="tiny",
            max_stack=8,
            code_size=first_byte >> 2,
            local_var_sig_token=0,
            init_locals=False,
        ), 1
    if format_bits == 0x3:
        flags_and_size = struct.unpack_from("<H", data, offset)[0]
        header_size = (flags_and_size >> 12) * 4
        return CLRMethodBodyHeader(
            format="fat",
            max_stack=struct.unpack_from("<H", data, offset + 2)[0],
            code_size=struct.unpack_from("<I", data, offset + 4)[0],
            local_var_sig_token=struct.unpack_from("<I", data, offset + 8)[0],
            init_locals=bool(flags_and_size & 0x10),
        ), header_size
    msg = f"Unsupported CLR method body header format {format_bits:#x}"
    raise ValueError(msg)


def _read_user_strings(user_string_heap: bytes) -> dict[int, str]:
    strings: dict[int, str] = {}
    offset = 1
    while offset < len(user_string_heap):
        length, length_size = _read_compressed_uint(user_string_heap, offset)
        start = offset + length_size
        end = start + length
        if end > len(user_string_heap):
            break
        payload = user_string_heap[start:end]
        if payload:
            strings[USER_STRING_TOKEN_PREFIX | offset] = payload[:-1].decode("utf-16le")
        offset = end
    return strings


def _parse_method_signature(
    blob: bytes,
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> CLRMethodSignature:
    if not blob:
        msg = "CLR method signature blob is empty"
        raise ValueError(msg)
    has_this = bool(blob[0] & 0x20)
    param_count, read_bytes = _read_compressed_uint(blob, 1)
    cursor = 1 + read_bytes
    return_type, cursor = _parse_type_name(
        blob,
        cursor,
        type_references,
        type_definitions,
    )
    parameter_types: list[str] = []
    for _ in range(param_count):
        parameter_type, cursor = _parse_type_name(
            blob,
            cursor,
            type_references,
            type_definitions,
        )
        parameter_types.append(parameter_type)
    return CLRMethodSignature(
        has_this=has_this,
        parameter_types=tuple(parameter_types),
        return_type=return_type,
    )


def _parse_type_name(
    blob: bytes,
    offset: int,
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> tuple[str, int]:
    element = blob[offset]
    cursor = offset + 1
    if element == ELEMENT_TYPE_VOID:
        return "void", cursor
    if element == ELEMENT_TYPE_BOOLEAN:
        return "bool", cursor
    if element == ELEMENT_TYPE_CHAR:
        return "char", cursor
    if element == ELEMENT_TYPE_I1:
        return "int8", cursor
    if element == ELEMENT_TYPE_U1:
        return "uint8", cursor
    if element == ELEMENT_TYPE_I2:
        return "int16", cursor
    if element == ELEMENT_TYPE_U2:
        return "uint16", cursor
    if element == ELEMENT_TYPE_I4:
        return "int32", cursor
    if element == ELEMENT_TYPE_U4:
        return "uint32", cursor
    if element == ELEMENT_TYPE_I8:
        return "int64", cursor
    if element == ELEMENT_TYPE_U8:
        return "uint64", cursor
    if element == ELEMENT_TYPE_R4:
        return "float32", cursor
    if element == ELEMENT_TYPE_R8:
        return "float64", cursor
    if element == ELEMENT_TYPE_STRING:
        return "string", cursor
    if element == ELEMENT_TYPE_OBJECT:
        return "object", cursor
    if element in {ELEMENT_TYPE_CLASS, ELEMENT_TYPE_VALUETYPE}:
        coded_index, read_bytes = _read_compressed_uint(blob, cursor)
        cursor += read_bytes
        return _resolve_typedef_or_ref_name(
            coded_index,
            type_references,
            type_definitions,
        ), cursor
    if element == ELEMENT_TYPE_SZARRAY:
        inner, cursor = _parse_type_name(
            blob,
            cursor,
            type_references,
            type_definitions,
        )
        return f"{inner}[]", cursor
    msg = f"Unsupported CLR signature element type {element:#x}"
    raise ValueError(msg)


def _resolve_typedef_or_ref_name(
    coded_index: int,
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> str:
    tag = coded_index & 0x3
    row_number = coded_index >> 2
    if tag == 0:
        token = TYPE_DEF_TOKEN_PREFIX | row_number
        return type_definitions[token].full_name
    if tag == 1:
        token = TYPE_REF_TOKEN_PREFIX | row_number
        return type_references[token].full_name
    if tag == 2:
        return "<typespec>"
    msg = f"Unsupported TypeDefOrRef tag {tag}"
    raise ValueError(msg)


def _resolve_member_ref_parent_name(
    coded_index: int,
    type_references: dict[int, CLRTypeReference],
    type_definitions: dict[int, CLRTypeDef],
) -> str:
    tag = coded_index & 0x7
    row_number = coded_index >> 3
    if tag == 0:
        return type_definitions[TYPE_DEF_TOKEN_PREFIX | row_number].full_name
    if tag == 1:
        return type_references[TYPE_REF_TOKEN_PREFIX | row_number].full_name
    if tag == 2:
        return "<moduleref>"
    if tag == 3:
        return "<methoddef>"
    if tag == 4:
        return "<typespec>"
    msg = f"Unsupported MemberRefParent tag {tag}"
    raise ValueError(msg)


def _resolve_resolution_scope_name(
    coded_index: int,
    type_ref_rows: dict[int, _TypeRefRow],
    assembly_ref_names: dict[int, str],
) -> str | None:
    if coded_index == 0:
        return None
    tag = coded_index & 0x3
    row_number = coded_index >> 2
    if tag == 0:
        return "<module>"
    if tag == 1:
        nested = type_ref_rows.get(row_number)
        if nested is None:
            return None
        return _join_type_name(nested.type_namespace, nested.type_name)
    if tag == 2:
        return assembly_ref_names.get(ASSEMBLY_REF_TOKEN_PREFIX | row_number)
    if tag == 3:
        return "<typeref>"
    return None


def _read_blob(heap: bytes, index: int) -> bytes:
    if index == 0:
        return b""
    length, read_bytes = _read_compressed_uint(heap, index)
    start = index + read_bytes
    return heap[start : start + length]


def _read_string(heap: bytes, index: int) -> str:
    if index == 0:
        return ""
    end = heap.index(b"\x00", index)
    return heap[index:end].decode("utf-8")


def _read_index(data: bytes, offset: int, size: int) -> int:
    if size == 2:
        return struct.unpack_from("<H", data, offset)[0]
    if size == 4:
        return struct.unpack_from("<I", data, offset)[0]
    msg = f"Unsupported metadata index size {size}"
    raise ValueError(msg)


def _read_compressed_uint(data: bytes, offset: int) -> tuple[int, int]:
    first = data[offset]
    if first & 0x80 == 0:
        return first, 1
    if first & 0xC0 == 0x80:
        return ((first & 0x3F) << 8) | data[offset + 1], 2
    if first & 0xE0 == 0xC0:
        return (
            ((first & 0x1F) << 24)
            | (data[offset + 1] << 16)
            | (data[offset + 2] << 8)
            | data[offset + 3],
            4,
        )
    msg = "Invalid compressed unsigned integer encoding"
    raise ValueError(msg)


def _table_row_size(
    table: int,
    row_counts: dict[int, int],
    string_index_size: int,
    guid_index_size: int,
    blob_index_size: int,
) -> int:
    simple = {
        MODULE_TABLE: 2 + string_index_size + (guid_index_size * 3),
        FIELD_TABLE: 2 + string_index_size + blob_index_size,
        PARAM_TABLE: 4 + string_index_size,
        STANDALONE_SIG_TABLE: blob_index_size,
        ASSEMBLY_TABLE: 16 + blob_index_size + (string_index_size * 2),
        TYPE_SPEC_TABLE: blob_index_size,
    }
    if table in simple:
        return simple[table]
    if table == TYPE_REF_TABLE:
        return _coded_index_size("ResolutionScope", row_counts) + (
            string_index_size * 2
        )
    if table == TYPE_DEF_TABLE:
        return (
            4
            + (string_index_size * 2)
            + _coded_index_size("TypeDefOrRef", row_counts)
            + _simple_index_size(row_counts.get(FIELD_TABLE, 0))
            + _simple_index_size(row_counts.get(METHOD_DEF_TABLE, 0))
        )
    if table == METHOD_DEF_TABLE:
        return 8 + string_index_size + blob_index_size + _simple_index_size(
            row_counts.get(PARAM_TABLE, 0)
        )
    if table == 0x09:
        return _simple_index_size(
            row_counts.get(TYPE_DEF_TABLE, 0)
        ) + _coded_index_size("TypeDefOrRef", row_counts)
    if table == MEMBER_REF_TABLE:
        return (
            _coded_index_size("MemberRefParent", row_counts)
            + string_index_size
            + blob_index_size
        )
    if table == 0x0B:
        return 2 + _coded_index_size("HasConstant", row_counts) + blob_index_size
    if table == 0x0C:
        return _coded_index_size("HasCustomAttribute", row_counts) + _coded_index_size(
            "CustomAttributeType",
            row_counts,
        ) + blob_index_size
    if table == 0x0D:
        return _coded_index_size("HasFieldMarshal", row_counts) + blob_index_size
    if table == 0x0E:
        return 2 + _coded_index_size("HasDeclSecurity", row_counts) + blob_index_size
    if table == 0x0F:
        return 6 + _simple_index_size(row_counts.get(TYPE_DEF_TABLE, 0))
    if table == 0x10:
        return 4 + _simple_index_size(row_counts.get(FIELD_TABLE, 0))
    if table == 0x12:
        return _simple_index_size(row_counts.get(TYPE_DEF_TABLE, 0)) + (
            _simple_index_size(row_counts.get(0x14, 0))
        )
    if table == 0x14:
        return 2 + string_index_size + _coded_index_size("TypeDefOrRef", row_counts)
    if table == 0x15:
        return _simple_index_size(row_counts.get(TYPE_DEF_TABLE, 0)) + (
            _simple_index_size(row_counts.get(0x17, 0))
        )
    if table == 0x17:
        return 2 + string_index_size + blob_index_size
    if table == 0x18:
        return 2 + _simple_index_size(
            row_counts.get(METHOD_DEF_TABLE, 0)
        ) + _coded_index_size("HasSemantics", row_counts)
    if table == 0x19:
        return _simple_index_size(row_counts.get(TYPE_DEF_TABLE, 0)) + (
            _coded_index_size("MethodDefOrRef", row_counts) * 2
        )
    if table == 0x1A:
        return string_index_size
    if table == 0x1C:
        return 2 + _coded_index_size("MemberForwarded", row_counts) + (
            string_index_size + _simple_index_size(row_counts.get(0x1A, 0))
        )
    if table == 0x1D:
        return 4 + _simple_index_size(row_counts.get(FIELD_TABLE, 0))
    if table == ASSEMBLY_REF_TABLE:
        return 12 + blob_index_size + (string_index_size * 2) + blob_index_size
    if table == 0x26:
        return 4 + string_index_size + blob_index_size
    if table == 0x27:
        return 8 + (string_index_size * 2) + _coded_index_size(
            "Implementation",
            row_counts,
        )
    if table == 0x28:
        return 8 + string_index_size + _coded_index_size("Implementation", row_counts)
    if table == 0x29:
        return _simple_index_size(row_counts.get(TYPE_DEF_TABLE, 0)) * 2
    if table == 0x2A:
        return 4 + _coded_index_size("TypeOrMethodDef", row_counts) + string_index_size
    if table == 0x2B:
        return _coded_index_size("MethodDefOrRef", row_counts) + blob_index_size
    if table == 0x2C:
        return _simple_index_size(row_counts.get(0x2A, 0)) + _coded_index_size(
            "TypeDefOrRef",
            row_counts,
        )
    msg = f"Unsupported metadata table {table:#x}"
    raise ValueError(msg)


def _simple_index_size(row_count: int) -> int:
    return 2 if row_count < (1 << 16) else 4


def _coded_index_size(name: str, row_counts: dict[int, int]) -> int:
    tables, tag_bits = _CODED_INDEX_TABLES[name]
    max_rows = max((row_counts.get(table, 0) for table in tables), default=0)
    return 2 if max_rows < (1 << (16 - tag_bits)) else 4


def _join_type_name(namespace: str, name: str) -> str:
    if namespace:
        return f"{namespace}.{name}"
    return name


def _align(value: int, alignment: int) -> int:
    remainder = value % alignment
    if remainder == 0:
        return value
    return value + (alignment - remainder)


_CODED_INDEX_TABLES: dict[str, tuple[tuple[int, ...], int]] = {
    "TypeDefOrRef": ((TYPE_DEF_TABLE, TYPE_REF_TABLE, TYPE_SPEC_TABLE), 2),
    "HasConstant": ((FIELD_TABLE, PARAM_TABLE, 0x17), 2),
    "HasCustomAttribute": (
        (
            METHOD_DEF_TABLE,
            FIELD_TABLE,
            TYPE_REF_TABLE,
            TYPE_DEF_TABLE,
            PARAM_TABLE,
            0x09,
            MEMBER_REF_TABLE,
            MODULE_TABLE,
            0x0E,
            0x17,
            0x14,
            STANDALONE_SIG_TABLE,
            0x1A,
            TYPE_SPEC_TABLE,
            ASSEMBLY_TABLE,
            ASSEMBLY_REF_TABLE,
            0x26,
            0x27,
            0x28,
            0x2A,
            0x2C,
            0x2B,
        ),
        5,
    ),
    "HasFieldMarshal": ((FIELD_TABLE, PARAM_TABLE), 1),
    "HasDeclSecurity": ((TYPE_DEF_TABLE, METHOD_DEF_TABLE, ASSEMBLY_TABLE), 2),
    "MemberRefParent": (
        (TYPE_DEF_TABLE, TYPE_REF_TABLE, 0x1A, METHOD_DEF_TABLE, TYPE_SPEC_TABLE),
        3,
    ),
    "HasSemantics": ((0x14, 0x17), 1),
    "MethodDefOrRef": ((METHOD_DEF_TABLE, MEMBER_REF_TABLE), 1),
    "MemberForwarded": ((FIELD_TABLE, METHOD_DEF_TABLE), 1),
    "Implementation": ((0x26, ASSEMBLY_REF_TABLE, 0x27), 2),
    "CustomAttributeType": ((METHOD_DEF_TABLE, MEMBER_REF_TABLE), 3),
    "ResolutionScope": ((MODULE_TABLE, 0x1A, ASSEMBLY_REF_TABLE, TYPE_REF_TABLE), 2),
    "TypeOrMethodDef": ((TYPE_DEF_TABLE, METHOD_DEF_TABLE), 1),
}
