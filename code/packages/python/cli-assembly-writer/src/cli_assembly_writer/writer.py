"""Write minimal PE/CLI assemblies from CIL program artifacts."""

from __future__ import annotations

import struct
from dataclasses import dataclass

from ir_to_cil_bytecode import CILHelper, CILMethodArtifact, CILProgramArtifact

_FILE_ALIGNMENT = 0x200
_SECTION_ALIGNMENT = 0x2000
_HEADERS_SIZE = 0x200
_TEXT_RVA = 0x2000
_CLI_HEADER_SIZE = 0x48
_PE_OFFSET = 0x80

# Canonical 64-byte MS-DOS stub sitting between the MZ header and the
# PE header (file offsets 0x40..0x7F).  Real .NET's ``PEReader``
# validates that this slot matches the standard pattern before
# proceeding; without it, the loader rejects the file as corrupt
# (CLR01).  The bytes are: a tiny x86 prologue that prints "This
# program cannot be run in DOS mode.\r\r\n$" and exits, followed by
# eight padding zeros.  ECMA-335 §II.25.2.1 references this exact
# stub as the conventional content of the slot.
_DOS_STUB = bytes.fromhex(
    "0e1fba0e00b409cd21b8014ccd2154686973207072"
    "6f6772616d2063616e6e6f742062652072756e2069"
    "6e20444f53206d6f64652e0d0d0a2400000000000000"
)
assert len(_DOS_STUB) == _PE_OFFSET - 0x40, "DOS stub must fill 0x40..0x80"

_MODULE_TABLE = 0x00
_TYPE_REF_TABLE = 0x01
_TYPE_DEF_TABLE = 0x02
_METHOD_DEF_TABLE = 0x06
_MEMBER_REF_TABLE = 0x0A
_STANDALONE_SIG_TABLE = 0x11
_ASSEMBLY_TABLE = 0x20
_ASSEMBLY_REF_TABLE = 0x23

# ECMA public-key token for the standard library assemblies
# (mscorlib, System.Runtime, …).  Burnt into the runtime; treat as
# an opaque eight-byte literal.
_ECMA_PUBLIC_KEY_TOKEN = bytes.fromhex("b03f5f7f11d50a3a")

# net9.0's System.Runtime version.
_SYSTEM_RUNTIME_VERSION = (9, 0, 0, 0)

_METHOD_DEF_TOKEN_PREFIX = 0x06000000
_MEMBER_REF_TOKEN_PREFIX = 0x0A000000
_STANDALONE_SIG_TOKEN_PREFIX = 0x11000000

_ELEMENT_TYPES = {
    "void": 0x01,
    "bool": 0x02,
    "char": 0x03,
    "int8": 0x04,
    "uint8": 0x05,
    "int16": 0x06,
    "uint16": 0x07,
    "int32": 0x08,
    "uint32": 0x09,
    "int64": 0x0A,
    "uint64": 0x0B,
    "float32": 0x0C,
    "float64": 0x0D,
    "string": 0x0E,
    "object": 0x1C,
}


class CLIAssemblyWriterError(ValueError):
    """Raised when CIL artifacts cannot be written as a CLI assembly."""


@dataclass(frozen=True)
class CLIAssemblyConfig:
    """Configuration for minimal PE/CLI assembly writing."""

    assembly_name: str = "GeneratedAssembly"
    module_name: str = "GeneratedAssembly.dll"
    type_name: str = "Program"
    type_namespace: str = ""
    helper_type_name: str = "CodingAdventures.Runtime.Helpers"
    metadata_version: str = "v4.0.30319"


@dataclass(frozen=True)
class CLIAssemblyArtifact:
    """Serialized CLI assembly bytes and token maps."""

    assembly_bytes: bytes
    entry_point_token: int
    method_tokens: dict[str, int]
    helper_tokens: dict[CILHelper, int]


@dataclass(frozen=True)
class _MethodLayout:
    method: CILMethodArtifact
    rva: int
    body: bytes
    local_sig_token: int


class _StringHeap:
    def __init__(self) -> None:
        self._data = bytearray(b"\x00")
        self._indices: dict[str, int] = {"": 0}

    def add(self, value: str) -> int:
        existing = self._indices.get(value)
        if existing is not None:
            return existing
        index = len(self._data)
        self._data.extend(value.encode("utf-8") + b"\x00")
        self._indices[value] = index
        return index

    def bytes(self) -> bytes:
        return bytes(self._data)


class _BlobHeap:
    def __init__(self) -> None:
        self._data = bytearray(b"\x00")
        self._indices: dict[bytes, int] = {b"": 0}

    def add(self, payload: bytes) -> int:
        existing = self._indices.get(payload)
        if existing is not None:
            return existing
        index = len(self._data)
        self._data.extend(_compressed_uint(len(payload)) + payload)
        self._indices[payload] = index
        return index

    def bytes(self) -> bytes:
        return bytes(self._data)


class _GuidHeap:
    """ECMA-335 §II.24.2.5 — fixed 16-byte entries, 1-indexed.

    Index 0 means "no GUID" (some columns accept that).  Real GUIDs
    start at index 1 and lookup is ``offset = (index - 1) * 16``.
    """

    def __init__(self) -> None:
        self._entries: list[bytes] = []

    def add(self, guid: bytes) -> int:
        if len(guid) != 16:
            msg = f"GUID must be 16 bytes, got {len(guid)}"
            raise CLIAssemblyWriterError(msg)
        self._entries.append(guid)
        return len(self._entries)  # 1-based

    def bytes(self) -> bytes:
        return b"".join(self._entries)


class CLIAssemblyWriter:
    """Serialize CIL program artifacts into a minimal managed PE image."""

    def __init__(self, config: CLIAssemblyConfig | None = None) -> None:
        self.config = config or CLIAssemblyConfig()

    def write(self, program: CILProgramArtifact) -> CLIAssemblyArtifact:
        """Write ``program`` into PE/CLI assembly bytes."""
        _validate_config(self.config)
        _validate_program(program)

        method_tokens = {
            method.name: _METHOD_DEF_TOKEN_PREFIX | (index + 1)
            for index, method in enumerate(program.methods)
        }
        helper_tokens = {
            spec.helper: _MEMBER_REF_TOKEN_PREFIX | (index + 1)
            for index, spec in enumerate(program.helper_specs)
        }
        entry_point_token = method_tokens[program.entry_label]

        method_layouts, method_blob = self._layout_methods(program)
        metadata = self._build_metadata(
            program,
            method_layouts,
        )
        cli_header_offset = _align(len(method_blob), 4)
        metadata_offset = cli_header_offset + _CLI_HEADER_SIZE
        text = bytearray(method_blob)
        text.extend(b"\x00" * (cli_header_offset - len(text)))
        text.extend(
            self._build_cli_header(
                metadata_rva=_TEXT_RVA + metadata_offset,
                metadata_size=len(metadata),
                entry_point_token=entry_point_token,
            )
        )
        text.extend(metadata)

        assembly_bytes = _build_pe_image(bytes(text))
        return CLIAssemblyArtifact(
            assembly_bytes=assembly_bytes,
            entry_point_token=entry_point_token,
            method_tokens=method_tokens,
            helper_tokens=helper_tokens,
        )

    def _layout_methods(
        self,
        program: CILProgramArtifact,
    ) -> tuple[tuple[_MethodLayout, ...], bytes]:
        layouts: list[_MethodLayout] = []
        out = bytearray()
        standalone_sig_row = 0
        for method in program.methods:
            out.extend(b"\x00" * (_align(len(out), 4) - len(out)))
            if method.local_types:
                standalone_sig_row += 1
                local_sig_token = _STANDALONE_SIG_TOKEN_PREFIX | standalone_sig_row
            else:
                local_sig_token = 0
            body = _encode_method_body(method, local_sig_token)
            layouts.append(
                _MethodLayout(
                    method=method,
                    rva=_TEXT_RVA + len(out),
                    body=body,
                    local_sig_token=local_sig_token,
                )
            )
            out.extend(body)
        return tuple(layouts), bytes(out)

    def _build_cli_header(
        self,
        *,
        metadata_rva: int,
        metadata_size: int,
        entry_point_token: int,
    ) -> bytes:
        return struct.pack(
            "<IHH" + ("I" * 16),
            _CLI_HEADER_SIZE,
            2,
            5,
            metadata_rva,
            metadata_size,
            0x00000001,
            entry_point_token,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        )

    def _build_metadata(
        self,
        program: CILProgramArtifact,
        method_layouts: tuple[_MethodLayout, ...],
    ) -> bytes:
        strings = _StringHeap()
        blobs = _BlobHeap()
        guids = _GuidHeap()
        # Module Mvid — a deterministic-but-stable GUID derived from
        # the assembly name keeps two writes of the same program byte
        # identical (useful for tests) while still satisfying real
        # .NET's "Module.Mvid must point at a real GUID" rule.
        mvid_index = guids.add(_derive_module_mvid(self.config.assembly_name))
        module_name_index = strings.add(self.config.module_name)
        type_name_index = strings.add(self.config.type_name)
        type_namespace_index = strings.add(self.config.type_namespace)
        assembly_name_index = strings.add(self.config.assembly_name)

        helper_namespace, helper_name = _split_type_name(self.config.helper_type_name)
        helper_name_index = strings.add(helper_name)
        helper_namespace_index = strings.add(helper_namespace)

        # CLR01 chunk 2: ECMA-335 §II.22.37 mandates that TypeDef row 1
        # be the special ``<Module>`` pseudo-type that owns module-level
        # fields and global functions.  Real .NET's metadata loader
        # rejects any TypeDef table that doesn't begin with this row.
        # Empty namespace, no fields, no methods (FieldList/MethodList
        # both pointing at row 1 means the user TypeDef on row 2 owns
        # everything starting from index 1).
        module_pseudo_name_index = strings.add("<Module>")

        # CLR01 chunk 3: AssemblyRef table + System.Object TypeRef.
        # The user's TypeDef must extend something — real .NET refuses
        # to load assemblies whose user types have ``Extends = 0``.
        # Real types extend ``System.Object``, which lives in
        # ``System.Runtime`` for net9.0.  We add (a) one AssemblyRef
        # row pointing at System.Runtime, (b) one TypeRef row for
        # System.Object resolving against that AssemblyRef, then
        # point the user TypeDef's ``Extends`` column at the
        # System.Object TypeRef.  See spec §II.22.5 / §II.24.2.6.
        system_runtime_name_index = strings.add("System.Runtime")
        empty_string_index = strings.add("")
        ecma_token_blob = blobs.add(_ECMA_PUBLIC_KEY_TOKEN)
        empty_blob = blobs.add(b"")
        system_object_name_index = strings.add("Object")
        system_namespace_index = strings.add("System")

        method_name_indices = {
            layout.method.name: strings.add(layout.method.name)
            for layout in method_layouts
        }
        helper_name_indices = {
            spec.helper: strings.add(spec.name)
            for spec in program.helper_specs
        }

        method_sig_indices = {
            layout.method.name: blobs.add(
                _method_signature(
                    layout.method.return_type,
                    layout.method.parameter_types,
                )
            )
            for layout in method_layouts
        }
        helper_sig_indices = {
            spec.helper: blobs.add(
                _method_signature(spec.return_type, spec.parameter_types)
            )
            for spec in program.helper_specs
        }
        local_sig_indices = {
            layout.local_sig_token: blobs.add(
                _local_signature(layout.method.local_types)
            )
            for layout in method_layouts
            if layout.local_sig_token
        }

        tables = self._build_tables_stream(
            program,
            method_layouts,
            module_name_index,
            module_pseudo_name_index,
            type_name_index,
            type_namespace_index,
            assembly_name_index,
            helper_name_index,
            helper_namespace_index,
            method_name_indices,
            method_sig_indices,
            helper_name_indices,
            helper_sig_indices,
            local_sig_indices,
            mvid_index=mvid_index,
            system_runtime_name_index=system_runtime_name_index,
            empty_string_index=empty_string_index,
            ecma_token_blob=ecma_token_blob,
            empty_blob=empty_blob,
            system_object_name_index=system_object_name_index,
            system_namespace_index=system_namespace_index,
        )
        # Stream order matches what real C# emits: #~, #Strings, #US,
        # #GUID, #Blob.  The order isn't load-bearing per ECMA-335 but
        # some loader paths (notably System.Reflection.Metadata) cache
        # offsets aggressively and behave poorly with surprising
        # arrangements.  Match the conventional layout.
        return _metadata_root(
            self.config.metadata_version,
            (
                ("#~", tables),
                ("#Strings", strings.bytes()),
                ("#US", b"\x00"),
                ("#GUID", guids.bytes()),
                ("#Blob", blobs.bytes()),
            ),
        )

    def _build_tables_stream(
        self,
        program: CILProgramArtifact,
        method_layouts: tuple[_MethodLayout, ...],
        module_name_index: int,
        module_pseudo_name_index: int,
        type_name_index: int,
        type_namespace_index: int,
        assembly_name_index: int,
        helper_name_index: int,
        helper_namespace_index: int,
        method_name_indices: dict[str, int],
        method_sig_indices: dict[str, int],
        helper_name_indices: dict[CILHelper, int],
        helper_sig_indices: dict[CILHelper, int],
        local_sig_indices: dict[int, int],
        *,
        mvid_index: int,
        system_runtime_name_index: int,
        empty_string_index: int,
        ecma_token_blob: int,
        empty_blob: int,
        system_object_name_index: int,
        system_namespace_index: int,
    ) -> bytes:
        tables: dict[int, list[bytes]] = {
            _MODULE_TABLE: [
                # Generation, Name, Mvid, EncId, EncBaseId.  Mvid is
                # a #GUID-heap index pointing at the real GUID we just
                # added; EncId/EncBaseId stay 0 (no edit-and-continue).
                struct.pack("<HHHHH", 0, module_name_index, mvid_index, 0, 0),
            ],
            _TYPE_REF_TABLE: [
                # Row 1: the existing helper TypeRef.  ResolutionScope
                # rewired (CLR01) from 0 (= dangling) to AssemblyRef row
                # 1.  ResolutionScope is a coded index with the table
                # tag in the low 2 bits; AssemblyRef tag = 2, row 1 →
                # ``(1 << 2) | 2 = 6``.
                struct.pack(
                    "<HHH",
                    6,
                    helper_name_index,
                    helper_namespace_index,
                ),
                # Row 2: ``System.Object`` in System.Runtime.  Same
                # AssemblyRef-row-1 coded index in ResolutionScope.
                struct.pack(
                    "<HHH",
                    6,
                    system_object_name_index,
                    system_namespace_index,
                ),
            ],
            _TYPE_DEF_TABLE: [
                # Row 1: the ``<Module>`` pseudo-TypeDef.  Flags=0,
                # empty namespace, Extends=0, FieldList=1, MethodList=1.
                # Owns no methods because the row 2 user TypeDef has
                # the same MethodList=1 (and no rows follow).
                struct.pack(
                    "<IHHHHH",
                    0,
                    module_pseudo_name_index,
                    0,
                    0,
                    1,
                    1,
                ),
                # Row 2: the user-supplied TypeDef.  Owns all real
                # methods starting at MethodDef row 1.  ``Extends`` is
                # a TypeDefOrRef coded index pointing at TypeRef row
                # 2 (System.Object); TypeRef tag = 1, row 2 →
                # ``(2 << 2) | 1 = 9``.
                struct.pack(
                    "<IHHHHH",
                    0x00100001,
                    type_name_index,
                    type_namespace_index,
                    9,
                    1,
                    1,
                ),
            ],
            _METHOD_DEF_TABLE: [
                struct.pack(
                    "<IHHHHH",
                    layout.rva,
                    0,
                    0x0016,
                    method_name_indices[layout.method.name],
                    method_sig_indices[layout.method.name],
                    1,
                )
                for layout in method_layouts
            ],
            _MEMBER_REF_TABLE: [
                struct.pack(
                    "<HHH",
                    (1 << 3) | 1,
                    helper_name_indices[spec.helper],
                    helper_sig_indices[spec.helper],
                )
                for spec in program.helper_specs
            ],
            _STANDALONE_SIG_TABLE: [
                struct.pack("<H", local_sig_indices[layout.local_sig_token])
                for layout in method_layouts
                if layout.local_sig_token
            ],
            _ASSEMBLY_TABLE: [
                # ECMA-335 §II.22.2 — Assembly row has 9 columns:
                # HashAlgId(U32), MajorVersion(U16), MinorVersion(U16),
                # BuildNumber(U16), RevisionNumber(U16), Flags(U32),
                # PublicKey(Blob idx), Name(String idx), Culture(String idx).
                # Total 22 bytes with narrow heaps.
                #
                # The pre-CLR01 row used format ``<IHHHHIHH`` (20 bytes,
                # 8 fields) — missing the Culture column entirely, AND
                # mis-mapped: ``assembly_name_index`` ended up in the
                # PublicKey slot instead of the Name slot.  Real .NET's
                # AssemblyRefTableReader reads past end-of-stream when
                # the Assembly table is 2 bytes short, producing the
                # cryptic ``BadImageFormatException: File is corrupt``
                # we hit before this fix.
                struct.pack(
                    "<IHHHHIHHH",
                    0,                        # HashAlgId
                    1, 0, 0, 0,               # 1.0.0.0
                    0,                        # Flags
                    0,                        # PublicKey blob (none)
                    assembly_name_index,      # Name (string idx)
                    0,                        # Culture (empty string idx)
                ),
            ],
            # CLR01 chunk 3: AssemblyRef row pointing at
            # ``System.Runtime`` (net9.0) with the standard ECMA
            # public-key token.  Without this row, every TypeRef row's
            # ResolutionScope dangles and real .NET refuses to load.
            # Row layout per ECMA-335 §II.22.5:
            #   USHORT MajorVersion, MinorVersion, BuildNumber,
            #          RevisionNumber
            #   ULONG  Flags
            #   BLOB   PublicKeyOrToken
            #   STRING Name, Culture
            #   BLOB   HashValue
            _ASSEMBLY_REF_TABLE: [
                struct.pack(
                    "<HHHHIHHHH",
                    _SYSTEM_RUNTIME_VERSION[0],
                    _SYSTEM_RUNTIME_VERSION[1],
                    _SYSTEM_RUNTIME_VERSION[2],
                    _SYSTEM_RUNTIME_VERSION[3],
                    0,
                    ecma_token_blob,
                    system_runtime_name_index,
                    empty_string_index,
                    empty_blob,
                ),
            ],
        }
        # ECMA-335: a table with zero rows must NOT have its bit set
        # in the valid mask, otherwise real .NET rejects the file as
        # corrupt.  Drop empties.
        for empty_table in (_STANDALONE_SIG_TABLE, _MEMBER_REF_TABLE):
            if not tables.get(empty_table):
                tables.pop(empty_table, None)

        valid_mask = 0
        for table in tables:
            valid_mask |= 1 << table

        out = bytearray(
            struct.pack(
                "<IBBBBQQ",
                0,
                2,
                0,
                0,
                1,
                valid_mask,
                valid_mask,
            )
        )
        for table in sorted(tables):
            out.extend(struct.pack("<I", len(tables[table])))
        for table in sorted(tables):
            for row in tables[table]:
                out.extend(row)
        return bytes(out)


def write_cli_assembly(
    program: CILProgramArtifact,
    config: CLIAssemblyConfig | None = None,
) -> CLIAssemblyArtifact:
    """Write a CIL program artifact into a minimal PE/CLI assembly."""
    return CLIAssemblyWriter(config).write(program)


def _validate_config(config: CLIAssemblyConfig) -> None:
    for field_name in (
        "assembly_name",
        "module_name",
        "type_name",
        "helper_type_name",
        "metadata_version",
    ):
        if not getattr(config, field_name):
            msg = f"{field_name} must not be empty"
            raise CLIAssemblyWriterError(msg)


def _validate_program(program: CILProgramArtifact) -> None:
    if not program.methods:
        msg = "program must contain at least one method"
        raise CLIAssemblyWriterError(msg)
    names = [method.name for method in program.methods]
    if len(set(names)) != len(names):
        msg = "method names must be unique"
        raise CLIAssemblyWriterError(msg)
    if program.entry_label not in names:
        msg = f"entry label {program.entry_label!r} is not an emitted method"
        raise CLIAssemblyWriterError(msg)


def _encode_method_body(method: CILMethodArtifact, local_sig_token: int) -> bytes:
    if not method.local_types and method.max_stack <= 8 and len(method.body) < 64:
        return bytes([(len(method.body) << 2) | 0x2]) + method.body
    flags_and_size = (3 << 12) | 0x03
    if method.local_types:
        flags_and_size |= 0x10
    return (
        struct.pack(
            "<HHII",
            flags_and_size,
            method.max_stack,
            len(method.body),
            local_sig_token,
        )
        + method.body
    )


def _build_pe_image(text: bytes) -> bytes:
    raw_text_size = _align(len(text), _FILE_ALIGNMENT)
    image_size = _align(_TEXT_RVA + len(text), _SECTION_ALIGNMENT)
    headers = bytearray(_HEADERS_SIZE)

    headers[0:2] = b"MZ"
    struct.pack_into("<I", headers, 0x3C, _PE_OFFSET)
    headers[0x40 : 0x40 + len(_DOS_STUB)] = _DOS_STUB
    headers[_PE_OFFSET : _PE_OFFSET + 4] = b"PE\x00\x00"

    coff_offset = _PE_OFFSET + 4
    optional_offset = coff_offset + 20
    # COFF Characteristics: real C# emits 0x22 = EXECUTABLE_IMAGE
    # (0x02) | LARGE_ADDRESS_AWARE (0x20).  We previously emitted
    # 0x102 with IMAGE_FILE_32BIT_MACHINE (0x100) instead, which is
    # the legacy "32-bit-only" flag and breaks under modern 64-bit
    # CoreCLR.  Match the real-C# value.
    struct.pack_into(
        "<HHIIIHH",
        headers,
        coff_offset,
        0x014C,
        1,
        0,
        0,
        0,
        0x00E0,
        0x0022,
    )

    optional = bytearray(0xE0)
    struct.pack_into("<HBBIII", optional, 0, 0x010B, 8, 0, raw_text_size, 0, 0)
    struct.pack_into("<III", optional, 16, 0, _TEXT_RVA, 0)
    struct.pack_into("<I", optional, 28, 0x00400000)
    struct.pack_into("<II", optional, 32, _SECTION_ALIGNMENT, _FILE_ALIGNMENT)
    struct.pack_into("<HHHHHH", optional, 40, 4, 0, 0, 0, 4, 0)
    struct.pack_into("<I", optional, 56, image_size)
    struct.pack_into("<I", optional, 60, _HEADERS_SIZE)
    struct.pack_into("<H", optional, 68, 3)
    struct.pack_into(
        "<IIIIII",
        optional,
        72,
        0x100000,
        0x1000,
        0x100000,
        0x1000,
        0,
        16,
    )
    cli_directory_offset = 96 + (14 * 8)
    struct.pack_into(
        "<II",
        optional,
        cli_directory_offset,
        _TEXT_RVA + _cli_offset(text),
        _CLI_HEADER_SIZE,
    )
    headers[optional_offset : optional_offset + len(optional)] = optional

    section_offset = optional_offset + len(optional)
    headers[section_offset : section_offset + 8] = b".text\x00\x00\x00"
    struct.pack_into(
        "<IIIIIIHHI",
        headers,
        section_offset + 8,
        len(text),
        _TEXT_RVA,
        raw_text_size,
        _HEADERS_SIZE,
        0,
        0,
        0,
        0,
        0x60000020,
    )

    return bytes(headers) + text + (b"\x00" * (raw_text_size - len(text)))


def _cli_offset(text: bytes) -> int:
    if len(text) < _CLI_HEADER_SIZE:
        msg = "text section is too small to contain a CLI header"
        raise CLIAssemblyWriterError(msg)
    marker = struct.pack("<IHH", _CLI_HEADER_SIZE, 2, 5)
    offset = text.find(marker)
    if offset < 0:
        msg = "CLI header marker not found"
        raise CLIAssemblyWriterError(msg)
    return offset


def _metadata_root(version: str, streams: tuple[tuple[str, bytes], ...]) -> bytes:
    version_bytes = version.encode("ascii") + b"\x00"
    version_bytes += b"\x00" * (_align(len(version_bytes), 4) - len(version_bytes))
    header_size = 16 + len(version_bytes) + 4
    stream_headers_size = sum(_stream_header_size(name) for name, _ in streams)
    stream_offset = _align(header_size + stream_headers_size, 4)

    stream_headers = bytearray()
    stream_payloads = bytearray()
    cursor = stream_offset
    for name, payload in streams:
        cursor = _align(cursor, 4)
        padding = _align(len(stream_payloads), 4) - len(stream_payloads)
        stream_payloads.extend(b"\x00" * padding)
        stream_headers.extend(struct.pack("<II", cursor, len(payload)))
        raw_name = name.encode("ascii") + b"\x00"
        raw_name += b"\x00" * (_align(len(raw_name), 4) - len(raw_name))
        stream_headers.extend(raw_name)
        stream_payloads.extend(payload)
        cursor += len(payload)

    return (
        b"BSJB"
        + struct.pack("<HHI", 1, 1, 0)
        + struct.pack("<I", len(version_bytes))
        + version_bytes
        + struct.pack("<HH", 0, len(streams))
        + bytes(stream_headers)
        + bytes(stream_payloads)
    )


def _stream_header_size(name: str) -> int:
    return 8 + _align(len(name.encode("ascii")) + 1, 4)


def _method_signature(return_type: str, parameter_types: tuple[str, ...]) -> bytes:
    out = bytearray([0x00])
    out.extend(_compressed_uint(len(parameter_types)))
    out.extend(_type_signature(return_type))
    for parameter_type in parameter_types:
        out.extend(_type_signature(parameter_type))
    return bytes(out)


def _local_signature(local_types: tuple[str, ...]) -> bytes:
    out = bytearray([0x07])
    out.extend(_compressed_uint(len(local_types)))
    for local_type in local_types:
        out.extend(_type_signature(local_type))
    return bytes(out)


def _type_signature(type_name: str) -> bytes:
    if type_name.endswith("[]"):
        return bytes([0x1D]) + _type_signature(type_name[:-2])
    element_type = _ELEMENT_TYPES.get(type_name)
    if element_type is None:
        msg = f"Unsupported CLI signature type: {type_name}"
        raise CLIAssemblyWriterError(msg)
    return bytes([element_type])


def _compressed_uint(value: int) -> bytes:
    if value < 0:
        msg = "compressed uint cannot be negative"
        raise CLIAssemblyWriterError(msg)
    if value <= 0x7F:
        return bytes([value])
    if value <= 0x3FFF:
        return bytes([(value >> 8) | 0x80, value & 0xFF])
    if value <= 0x1FFFFFFF:
        return bytes(
            [
                ((value >> 24) & 0x1F) | 0xC0,
                (value >> 16) & 0xFF,
                (value >> 8) & 0xFF,
                value & 0xFF,
            ]
        )
    msg = f"compressed uint is too large: {value}"
    raise CLIAssemblyWriterError(msg)


def _split_type_name(full_name: str) -> tuple[str, str]:
    if "." not in full_name:
        return "", full_name
    namespace, _, name = full_name.rpartition(".")
    if not name:
        msg = f"invalid type name: {full_name}"
        raise CLIAssemblyWriterError(msg)
    return namespace, name


def _derive_module_mvid(assembly_name: str) -> bytes:
    """Return a deterministic 16-byte GUID for the module's Mvid.

    Real C# emits a fresh random GUID per build, but we want byte-
    stable output for tests.  SHA-256(assembly_name) truncated to 16
    bytes gives us that, and the result is a valid GUID for any
    purpose — Mvid only has to be unique per module instance, not
    cryptographically strong.
    """
    import hashlib

    return hashlib.sha256(assembly_name.encode("utf-8")).digest()[:16]


def _align(value: int, alignment: int) -> int:
    remainder = value % alignment
    if remainder == 0:
        return value
    return value + (alignment - remainder)
