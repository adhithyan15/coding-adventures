"""Write minimal PE/CLI assemblies from CIL program artifacts."""

from __future__ import annotations

import struct
from dataclasses import dataclass

from ir_to_cil_bytecode import (
    CILHelper,
    CILMethodArtifact,
    CILProgramArtifact,
    CILTypeArtifact,
)

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
_FIELD_TABLE = 0x04
_METHOD_DEF_TABLE = 0x06
_INTERFACE_IMPL_TABLE = 0x09
_MEMBER_REF_TABLE = 0x0A
_STANDALONE_SIG_TABLE = 0x11
_ASSEMBLY_TABLE = 0x20
_ASSEMBLY_REF_TABLE = 0x23

# Type flags (ECMA-335 §II.23.1.15).  See ``CILTypeArtifact`` docs.
# Public + AutoLayout + Class + AnsiClass + BeforeFieldInit.
_TYPE_FLAGS_PUBLIC_CLASS = 0x00100001
# Public + Interface + Abstract + AnsiClass.
_TYPE_FLAGS_PUBLIC_INTERFACE = 0x000000A1

# Method flags (ECMA-335 §II.23.1.10).  Existing static methods use
# 0x0016 = Public(0x6) + Static(0x10).  Instance methods drop Static
# and add HideBySig.  Constructors add SpecialName + RTSpecialName.
_METHOD_FLAGS_PUBLIC_STATIC = 0x0016
# Public + Virtual + HideBySig + NewSlot + Final.  Used for
# concrete instance methods on closure classes that *implement*
# an interface method.  ``NewSlot`` puts the method into its own
# vtable slot (rather than trying to override a parent class
# slot, which doesn't exist for closure ``Apply``); ``Final``
# prevents further overriding (closures are leaf types).
_METHOD_FLAGS_PUBLIC_INSTANCE = 0x01E6
# Public + HideBySig + SpecialName + RTSpecialName.
_METHOD_FLAGS_PUBLIC_CTOR = 0x1886
# Public + Virtual + HideBySig + NewSlot + Abstract.
_METHOD_FLAGS_INTERFACE_ABSTRACT = 0x05C6

# Method calling-convention bits (ECMA-335 §II.23.2.1).
_SIG_CALLCONV_DEFAULT = 0x00
_SIG_CALLCONV_HASTHIS = 0x20

# Coded-index tag widths (ECMA-335 §II.24.2.6).  All packed in 2 bytes
# while every referenced table stays under the relevant threshold.
_TYPEDEFORREF_TAG_TYPEDEF = 0
_TYPEDEFORREF_TAG_TYPEREF = 1
_TYPEDEFORREF_TAG_TYPESPEC = 2
_TYPEDEFORREF_TAG_BITS = 2

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
        """Lay out main-type methods first, then each extra-type's
        methods in declaration order.  The output order corresponds
        directly to MethodDef table row order, which the TypeDef
        ``MethodList`` column uses as a 1-based starting index.
        """
        layouts: list[_MethodLayout] = []
        out = bytearray()
        standalone_sig_row = 0
        ordered_methods: list[CILMethodArtifact] = list(program.methods)
        for extra in program.extra_types:
            ordered_methods.extend(extra.methods)
        for method in ordered_methods:
            if method.is_abstract:
                # Abstract methods have no body; the MethodDef row's
                # RVA column must be 0.  We still emit a layout entry
                # so the row index lines up with the ordered method
                # list.
                layouts.append(
                    _MethodLayout(
                        method=method,
                        rva=0,
                        body=b"",
                        local_sig_token=0,
                    )
                )
                continue
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

        # Index every extra type's name + namespace in the string
        # heap up-front.  Used both for the TypeDef rows themselves
        # and (via a name → TypeDef-row map) for resolving the
        # ``implements`` / ``extends`` columns later.
        extra_type_name_indices = [
            (strings.add(t.namespace), strings.add(t.name))
            for t in program.extra_types
        ]
        # Map "Namespace.Name" → 1-based TypeDef row.  Row 1 is the
        # ``<Module>`` pseudo, row 2 is the main user type, rows 3+
        # are the extras in declaration order.
        typedef_row_for_name: dict[str, int] = {
            _qualified_name(self.config.type_namespace, self.config.type_name): 2,
        }
        for offset, t in enumerate(program.extra_types):
            typedef_row_for_name[_qualified_name(t.namespace, t.name)] = 3 + offset

        # CLR02 Phase 2c: when any extra type extends ``System.Object``
        # (i.e. a concrete closure class), the closure's .ctor body
        # chains into ``Object::.ctor()``.  That requires a MemberRef
        # row pointing at the existing System.Object TypeRef.  We
        # always emit it at MemberRef row ``len(helper_specs) + 1``
        # whenever the trigger fires, so the lowerer's deterministic
        # token computation matches the actual emitted row.
        needs_object_ctor_memberref = any(
            (not t.is_interface) and t.extends == "System.Object"
            for t in program.extra_types
        )
        ctor_string_index = strings.add(".ctor") if needs_object_ctor_memberref else 0
        # ``HASTHIS | paramcount(0) | retType(void)`` per ECMA-335 §II.23.2.1
        object_ctor_sig_index = (
            blobs.add(bytes([_SIG_CALLCONV_HASTHIS, 0x00, 0x01]))
            if needs_object_ctor_memberref
            else 0
        )

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
        # TW03 Phase 3 follow-up: System.Int32 TypeRef so the box
        # opcode can wrap int32 closure returns into ``object`` for
        # the IClosure.Apply contract.  Always emitted (small cost
        # — 6 bytes of TypeRef + 6 bytes of string heap) so the
        # token assignment stays deterministic regardless of whether
        # any specific compilation needs box.
        system_int32_name_index = strings.add("Int32")

        # Same keying decision as the sig table: MethodDef row (1-based).
        # ``Apply`` / ``.ctor`` repeat across closure types and would
        # collide on name.
        method_name_indices: dict[int, int] = {
            (idx + 1): strings.add(layout.method.name)
            for idx, layout in enumerate(method_layouts)
        }
        helper_name_indices = {
            spec.helper: strings.add(spec.name)
            for spec in program.helper_specs
        }

        # Method-sig blobs are keyed by MethodDef row (1-based) rather
        # than method name because extra closure types can each have a
        # method called ``Apply`` or ``.ctor`` — name-keying would
        # silently fold them onto a single shared sig blob.
        method_sig_indices: dict[int, int] = {
            (idx + 1): blobs.add(
                _method_signature(
                    layout.method.return_type,
                    layout.method.parameter_types,
                    has_this=layout.method.is_instance,
                )
            )
            for idx, layout in enumerate(method_layouts)
        }
        helper_sig_indices = {
            spec.helper: blobs.add(
                _method_signature(spec.return_type, spec.parameter_types)
            )
            for spec in program.helper_specs
        }
        # Field-sig blobs, keyed similarly by Field-table row (1-based).
        # Walks extra_types in declaration order so the row indices
        # match what the writer assigns when building the Field table.
        field_sig_indices: dict[int, int] = {}
        field_name_indices: dict[int, int] = {}
        field_row = 0
        for extra in program.extra_types:
            for fld in extra.fields:
                field_row += 1
                field_sig_indices[field_row] = blobs.add(_field_signature(fld.type))
                field_name_indices[field_row] = strings.add(fld.name)
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
            extra_type_name_indices=tuple(extra_type_name_indices),
            typedef_row_for_name=typedef_row_for_name,
            field_name_indices=field_name_indices,
            field_sig_indices=field_sig_indices,
            ctor_string_index=ctor_string_index,
            object_ctor_sig_index=object_ctor_sig_index,
            needs_object_ctor_memberref=needs_object_ctor_memberref,
            mvid_index=mvid_index,
            system_runtime_name_index=system_runtime_name_index,
            empty_string_index=empty_string_index,
            ecma_token_blob=ecma_token_blob,
            empty_blob=empty_blob,
            system_object_name_index=system_object_name_index,
            system_int32_name_index=system_int32_name_index,
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
        method_name_indices: dict[int, int],
        method_sig_indices: dict[int, int],
        helper_name_indices: dict[CILHelper, int],
        helper_sig_indices: dict[CILHelper, int],
        local_sig_indices: dict[int, int],
        *,
        extra_type_name_indices: tuple[tuple[int, int], ...],
        typedef_row_for_name: dict[str, int],
        field_name_indices: dict[int, int],
        field_sig_indices: dict[int, int],
        ctor_string_index: int,
        object_ctor_sig_index: int,
        needs_object_ctor_memberref: bool,
        mvid_index: int,
        system_runtime_name_index: int,
        empty_string_index: int,
        ecma_token_blob: int,
        empty_blob: int,
        system_object_name_index: int,
        system_int32_name_index: int,
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
                # Row 3: ``System.Int32``.  Used by closure subclasses'
                # Apply method when boxing int32 returns into the
                # ``object`` IClosure.Apply contract (TW03 Phase 3).
                struct.pack(
                    "<HHH",
                    6,
                    system_int32_name_index,
                    system_namespace_index,
                ),
            ],
            _TYPE_DEF_TABLE: _build_typedef_rows(
                program=program,
                module_pseudo_name_index=module_pseudo_name_index,
                type_name_index=type_name_index,
                type_namespace_index=type_namespace_index,
                extra_type_name_indices=extra_type_name_indices,
                typedef_row_for_name=typedef_row_for_name,
            ),
            _METHOD_DEF_TABLE: [
                struct.pack(
                    "<IHHHHH",
                    layout.rva,
                    0,
                    _method_def_flags(layout.method),
                    method_name_indices[idx + 1],
                    method_sig_indices[idx + 1],
                    1,  # ParamList — params table omitted (always row 1)
                )
                for idx, layout in enumerate(method_layouts)
            ],
            _FIELD_TABLE: _build_field_rows(
                program=program,
                field_name_indices=field_name_indices,
                field_sig_indices=field_sig_indices,
            ),
            _INTERFACE_IMPL_TABLE: _build_interfaceimpl_rows(
                program=program,
                typedef_row_for_name=typedef_row_for_name,
            ),
            _MEMBER_REF_TABLE: _build_memberref_rows(
                program=program,
                helper_name_indices=helper_name_indices,
                helper_sig_indices=helper_sig_indices,
                ctor_string_index=ctor_string_index,
                object_ctor_sig_index=object_ctor_sig_index,
                needs_object_ctor_memberref=needs_object_ctor_memberref,
            ),
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
        for empty_table in (
            _STANDALONE_SIG_TABLE,
            _MEMBER_REF_TABLE,
            _FIELD_TABLE,
            _INTERFACE_IMPL_TABLE,
        ):
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


def _build_typedef_rows(
    *,
    program: CILProgramArtifact,
    module_pseudo_name_index: int,
    type_name_index: int,
    type_namespace_index: int,
    extra_type_name_indices: tuple[tuple[int, int], ...],
    typedef_row_for_name: dict[str, int],
) -> list[bytes]:
    """Emit the TypeDef table rows.

    Layout: ``Flags(U32), Name(StrIdx), Namespace(StrIdx),
    Extends(TypeDefOrRef), FieldList(FieldIdx), MethodList(MethodIdx)``.

    Row 1 is always ``<Module>`` (ECMA-335 §II.22.37).  Row 2 is the
    user's main type.  Rows 3+ are the extras in declaration order.

    ``FieldList`` and ``MethodList`` are running 1-based offsets into
    the Field / MethodDef tables — each TypeDef owns the consecutive
    rows starting at its declared offset, ending where the next
    TypeDef's offset begins (or at end-of-table for the last row).
    """
    method_offset = 1 + len(program.methods)
    field_offset = 1
    rows: list[bytes] = [
        # Row 1: <Module>.  No fields, no methods owned.
        struct.pack(
            "<IHHHHH",
            0,
            module_pseudo_name_index,
            0,
            0,
            1,
            1,
        ),
        # Row 2: the user's main type.  Owns MethodDef rows
        # 1..len(program.methods), no fields.  ``Extends`` is
        # System.Object — TypeRef row 2, TypeDefOrRef tag = 1
        # (TypeRef), encoded as ``(2 << 2) | 1 = 9``.
        struct.pack(
            "<IHHHHH",
            _TYPE_FLAGS_PUBLIC_CLASS,
            type_name_index,
            type_namespace_index,
            9,
            field_offset,
            1,  # MethodList — main type starts at row 1
        ),
    ]

    for (ns_idx, name_idx), extra in zip(
        extra_type_name_indices, program.extra_types, strict=True,
    ):
        flags = (
            _TYPE_FLAGS_PUBLIC_INTERFACE
            if extra.is_interface
            else _TYPE_FLAGS_PUBLIC_CLASS
        )
        extends_index = _resolve_extends(extra, typedef_row_for_name)
        rows.append(
            struct.pack(
                "<IHHHHH",
                flags,
                name_idx,
                ns_idx,
                extends_index,
                field_offset,
                method_offset,
            )
        )
        method_offset += len(extra.methods)
        field_offset += len(extra.fields)

    return rows


def _resolve_extends(
    extra: CILTypeArtifact,
    typedef_row_for_name: dict[str, int],
) -> int:
    """Encode the ``Extends`` column as a ``TypeDefOrRef`` coded index.

    Tag bits (low 2): 0 = TypeDef row, 1 = TypeRef row, 2 = TypeSpec
    row.  We only resolve to TypeRef row 2 (System.Object) or to a
    same-module TypeDef.  Interfaces have ``extends=None`` and the
    column must be 0.
    """
    if extra.is_interface or extra.extends is None:
        return 0
    if extra.extends == "System.Object":
        return (2 << _TYPEDEFORREF_TAG_BITS) | _TYPEDEFORREF_TAG_TYPEREF
    if extra.extends in typedef_row_for_name:
        row = typedef_row_for_name[extra.extends]
        return (row << _TYPEDEFORREF_TAG_BITS) | _TYPEDEFORREF_TAG_TYPEDEF
    msg = (
        f"unsupported ``extends`` reference {extra.extends!r} on "
        f"type {extra.name!r} — only ``System.Object`` and "
        "same-module TypeDefs are resolvable in CLR02 v1"
    )
    raise CLIAssemblyWriterError(msg)


def _build_field_rows(
    *,
    program: CILProgramArtifact,
    field_name_indices: dict[int, int],
    field_sig_indices: dict[int, int],
) -> list[bytes]:
    """Emit the Field table rows for every field on every extra type.

    Row layout: ``Flags(U16), Name(StrIdx), Signature(BlobIdx)`` =
    6 bytes with narrow heaps.  Public instance fields use flags
    ``0x0006`` (Public + InstanceContract — i.e. not Static).
    """
    rows: list[bytes] = []
    field_row = 0
    for extra in program.extra_types:
        for _fld in extra.fields:
            field_row += 1
            rows.append(
                struct.pack(
                    "<HHH",
                    0x0006,  # Public, instance (no Static bit)
                    field_name_indices[field_row],
                    field_sig_indices[field_row],
                )
            )
    return rows


def _build_interfaceimpl_rows(
    *,
    program: CILProgramArtifact,
    typedef_row_for_name: dict[str, int],
) -> list[bytes]:
    """Emit one InterfaceImpl row per ``(class, interface)`` pair.

    Row layout: ``Class(TypeDefIdx), Interface(TypeDefOrRef)`` =
    4 bytes.  ECMA-335 §II.22.23 mandates rows be sorted by Class
    then Interface — we iterate ``extra_types`` in declaration
    order, which gives ascending Class indices automatically; ties
    inside one type are sorted by interface name.
    """
    rows: list[bytes] = []
    for offset, extra in enumerate(program.extra_types):
        if not extra.implements:
            continue
        class_row = 3 + offset  # extras start at TypeDef row 3
        # Resolve and encode each interface, then sort within this
        # type's group to satisfy the table ordering invariant.
        encoded: list[int] = []
        for iface_name in extra.implements:
            if iface_name not in typedef_row_for_name:
                msg = (
                    f"interface {iface_name!r} on {extra.name!r} is not "
                    "declared in this assembly's extra_types — only "
                    "same-module interfaces are supported in CLR02 v1"
                )
                raise CLIAssemblyWriterError(msg)
            iface_row = typedef_row_for_name[iface_name]
            encoded.append(
                (iface_row << _TYPEDEFORREF_TAG_BITS)
                | _TYPEDEFORREF_TAG_TYPEDEF
            )
        for iface_coded in sorted(encoded):
            rows.append(struct.pack("<HH", class_row, iface_coded))
    return rows


def _build_memberref_rows(
    *,
    program: CILProgramArtifact,
    helper_name_indices: dict[CILHelper, int],
    helper_sig_indices: dict[CILHelper, int],
    ctor_string_index: int,
    object_ctor_sig_index: int,
    needs_object_ctor_memberref: bool,
) -> list[bytes]:
    """Emit the MemberRef table.

    Helper rows go first (so their token positions stay stable for
    CLR01 callers).  When any closure type is present (CLR02
    Phase 2c), one additional row is appended for
    ``[System.Runtime]System.Object::.ctor()`` so closure ctors can
    chain into it via a deterministic ``0x0A...`` token.

    Layout per row: ``Class(MemberRefParent), Name(StringIdx),
    Signature(BlobIdx)`` = 6 bytes with narrow heaps.

    ``MemberRefParent`` tag bits (low 3): TypeRef = 1.  Helper rows
    point at TypeRef row 1 (the helper's TypeRef) → ``(1 << 3) | 1
    = 9``.  System.Object::.ctor points at TypeRef row 2 →
    ``(2 << 3) | 1 = 17``.
    """
    rows: list[bytes] = [
        struct.pack(
            "<HHH",
            (1 << 3) | 1,
            helper_name_indices[spec.helper],
            helper_sig_indices[spec.helper],
        )
        for spec in program.helper_specs
    ]
    if needs_object_ctor_memberref:
        rows.append(
            struct.pack(
                "<HHH",
                (2 << 3) | 1,
                ctor_string_index,
                object_ctor_sig_index,
            )
        )
    return rows


def _method_def_flags(method: CILMethodArtifact) -> int:
    """Pick the ECMA-335 ``MethodAttributes`` value for ``method``.

    Default (``is_instance=False``, ``is_special_name=False``,
    ``is_abstract=False``) is the existing static-method shape
    (``0x0016``) so CLR01-era callers keep producing identical bytes.
    """
    if method.is_abstract:
        return _METHOD_FLAGS_INTERFACE_ABSTRACT
    if method.is_special_name:
        return _METHOD_FLAGS_PUBLIC_CTOR
    if not method.is_instance:
        return _METHOD_FLAGS_PUBLIC_STATIC
    return _METHOD_FLAGS_PUBLIC_INSTANCE


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


def _method_signature(
    return_type: str,
    parameter_types: tuple[str, ...],
    *,
    has_this: bool = False,
) -> bytes:
    """Encode a CIL method signature blob (ECMA-335 §II.23.2.1).

    The first byte is the calling convention.  ``HASTHIS`` (0x20)
    flags the method as instance-bound (the first arg passed by the
    runtime is the implicit ``this`` reference).  Existing static
    methods leave it clear (0x00).
    """
    callconv = _SIG_CALLCONV_HASTHIS if has_this else _SIG_CALLCONV_DEFAULT
    out = bytearray([callconv])
    out.extend(_compressed_uint(len(parameter_types)))
    out.extend(_type_signature(return_type))
    for parameter_type in parameter_types:
        out.extend(_type_signature(parameter_type))
    return bytes(out)


def _field_signature(field_type: str) -> bytes:
    """Encode a Field signature blob (ECMA-335 §II.23.2.4).

    Layout: ``0x06 || TypeSig``.  We only emit value-type and ref-
    type fields here; fancier modifiers (CMOD_REQD / CMOD_OPT) are
    out of scope.
    """
    return bytes([0x06]) + _type_signature(field_type)


def _qualified_name(namespace: str, name: str) -> str:
    return f"{namespace}.{name}" if namespace else name


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
