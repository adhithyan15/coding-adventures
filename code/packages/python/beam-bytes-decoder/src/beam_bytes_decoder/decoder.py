"""Low-level BEAM bytes decoding utilities."""

from dataclasses import dataclass
from io import BytesIO

from beam_opcode_metadata import OTP_28_PROFILE, BeamProfile


@dataclass(frozen=True)
class BeamChunk:
    """One IFF chunk inside a BEAM container."""

    chunk_id: str
    data: bytes
    offset: int


@dataclass(frozen=True)
class BeamContainer:
    """Parsed BEAM IFF container."""

    form_id: str
    chunks: tuple[BeamChunk, ...]

    def chunk_map(self) -> dict[str, BeamChunk]:
        """Return a mapping from chunk id to chunk."""
        return {chunk.chunk_id: chunk for chunk in self.chunks}


@dataclass(frozen=True)
class BeamCodeHeader:
    """Decoded metadata from the Code chunk header."""

    sub_size: int
    format_number: int
    max_opcode: int
    label_count: int
    function_count: int
    code: bytes


@dataclass(frozen=True)
class BeamImportEntry:
    """One ImpT entry."""

    module: str
    function: str
    arity: int


@dataclass(frozen=True)
class BeamExportEntry:
    """One ExpT or LocT entry."""

    function: str
    arity: int
    label: int


@dataclass(frozen=True)
class DecodedBeamModule:
    """Reusable low-level decoded BEAM module representation."""

    profile: BeamProfile
    module_name: str
    atoms: tuple[str | None, ...]
    code_header: BeamCodeHeader
    imports: tuple[BeamImportEntry, ...]
    exports: tuple[BeamExportEntry, ...]
    locals: tuple[BeamExportEntry, ...]
    chunks: tuple[BeamChunk, ...]
    literal_chunk: bytes | None = None


def _read_u32(stream: BytesIO) -> int:
    data = stream.read(4)
    if len(data) != 4:
        msg = "Unexpected end of stream while reading u32"
        raise ValueError(msg)
    return int.from_bytes(data, byteorder="big", signed=False)


def _read_i32(stream: BytesIO) -> int:
    data = stream.read(4)
    if len(data) != 4:
        msg = "Unexpected end of stream while reading i32"
        raise ValueError(msg)
    return int.from_bytes(data, byteorder="big", signed=True)


def _decode_compact_u(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode the compact unsigned integer encoding used in modern AtU8."""
    first = data[offset]
    tag = first & 0b111
    if tag != 0:
        msg = f"Expected compact unsigned integer tag, got tag {tag}"
        raise ValueError(msg)
    if (first & 0x08) == 0:
        return first >> 4, offset + 1
    if (first & 0x10) == 0:
        if offset + 1 >= len(data):
            msg = "Truncated compact unsigned integer"
            raise ValueError(msg)
        value = ((first & 0xE0) << 3) | data[offset + 1]
        return value, offset + 2

    length_code = first >> 5
    if length_code == 7:
        nested_length, next_offset = _decode_compact_u(data, offset + 1)
        length = nested_length + 9
    else:
        next_offset = offset + 1
        length = length_code + 2

    end_offset = next_offset + length
    if end_offset > len(data):
        msg = "Truncated big compact unsigned integer"
        raise ValueError(msg)
    return (
        int.from_bytes(data[next_offset:end_offset], byteorder="big", signed=False),
        end_offset,
    )


def parse_beam_container(data: bytes) -> BeamContainer:
    """Parse a raw `.beam` file into an IFF chunk table."""
    stream = BytesIO(data)
    magic = stream.read(4)
    if magic != b"FOR1":
        msg = "Not a BEAM file: missing FOR1 header"
        raise ValueError(msg)

    declared_size = _read_u32(stream)
    if declared_size + 8 != len(data):
        msg = (
            f"Invalid FOR1 size: declared {declared_size}, "
            f"actual payload {len(data) - 8}"
        )
        raise ValueError(msg)

    form_id = stream.read(4).decode("ascii")
    if form_id != "BEAM":
        msg = f"Unsupported IFF form {form_id!r}; expected 'BEAM'"
        raise ValueError(msg)

    chunks: list[BeamChunk] = []
    while stream.tell() < len(data):
        offset = stream.tell()
        chunk_id_bytes = stream.read(4)
        if not chunk_id_bytes:
            break
        if len(chunk_id_bytes) != 4:
            msg = "Truncated chunk id"
            raise ValueError(msg)
        chunk_id = chunk_id_bytes.decode("ascii")
        chunk_size = _read_u32(stream)
        chunk_data = stream.read(chunk_size)
        if len(chunk_data) != chunk_size:
            msg = f"Truncated chunk {chunk_id!r}"
            raise ValueError(msg)
        chunks.append(BeamChunk(chunk_id=chunk_id, data=chunk_data, offset=offset))
        padding = (4 - (chunk_size % 4)) % 4
        if padding:
            stream.read(padding)

    return BeamContainer(form_id=form_id, chunks=tuple(chunks))


def _decode_atoms(chunk_data: bytes) -> tuple[str | None, ...]:
    stream = BytesIO(chunk_data)
    raw_count = _read_i32(stream)
    long_counts = raw_count < 0
    count = abs(raw_count)
    atoms: list[str | None] = [None]
    for _ in range(count):
        if long_counts:
            remaining = stream.read()
            length, consumed = _decode_compact_u(remaining)
            stream.seek(stream.tell() - len(remaining) + consumed)
        else:
            length_raw = stream.read(1)
            if len(length_raw) != 1:
                msg = "Truncated atom length"
                raise ValueError(msg)
            length = length_raw[0]

        atom_bytes = stream.read(length)
        if len(atom_bytes) != length:
            msg = "Truncated atom text"
            raise ValueError(msg)
        atoms.append(atom_bytes.decode("utf-8"))
    return tuple(atoms)


def _decode_code_header(chunk_data: bytes) -> BeamCodeHeader:
    stream = BytesIO(chunk_data)
    sub_size = _read_u32(stream)
    format_number = _read_u32(stream)
    max_opcode = _read_u32(stream)
    label_count = _read_u32(stream)
    function_count = _read_u32(stream)
    stream.seek(sub_size + 4)
    code = stream.read()
    return BeamCodeHeader(
        sub_size=sub_size,
        format_number=format_number,
        max_opcode=max_opcode,
        label_count=label_count,
        function_count=function_count,
        code=code,
    )


def _decode_imports(
    chunk_data: bytes,
    atoms: tuple[str | None, ...],
) -> tuple[BeamImportEntry, ...]:
    stream = BytesIO(chunk_data)
    count = _read_u32(stream)
    imports: list[BeamImportEntry] = []
    for _ in range(count):
        module_index = _read_u32(stream)
        function_index = _read_u32(stream)
        arity = _read_u32(stream)
        imports.append(
            BeamImportEntry(
                module=atoms[module_index] or "[]",
                function=atoms[function_index] or "[]",
                arity=arity,
            )
        )
    return tuple(imports)


def _decode_exports(
    chunk_data: bytes,
    atoms: tuple[str | None, ...],
) -> tuple[BeamExportEntry, ...]:
    stream = BytesIO(chunk_data)
    count = _read_u32(stream)
    exports: list[BeamExportEntry] = []
    for _ in range(count):
        function_index = _read_u32(stream)
        arity = _read_u32(stream)
        label = _read_u32(stream)
        exports.append(
            BeamExportEntry(
                function=atoms[function_index] or "[]",
                arity=arity,
                label=label,
            )
        )
    return tuple(exports)


def decode_beam_module(
    data: bytes,
    profile: BeamProfile = OTP_28_PROFILE,
) -> DecodedBeamModule:
    """Decode the reusable chunk-level representation of a BEAM module."""
    container = parse_beam_container(data)
    chunks = container.chunk_map()
    required = ("AtU8", "Code", "ImpT", "ExpT")
    for chunk_id in required:
        if chunk_id not in chunks:
            msg = f"Missing required chunk {chunk_id!r}"
            raise ValueError(msg)

    atoms = _decode_atoms(chunks["AtU8"].data)
    code_header = _decode_code_header(chunks["Code"].data)
    imports = _decode_imports(chunks["ImpT"].data, atoms)
    exports = _decode_exports(chunks["ExpT"].data, atoms)
    locals_table = (
        _decode_exports(chunks["LocT"].data, atoms) if "LocT" in chunks else ()
    )
    literal_chunk = chunks["LitT"].data if "LitT" in chunks else None

    return DecodedBeamModule(
        profile=profile,
        module_name=atoms[1] or "[]",
        atoms=atoms,
        code_header=code_header,
        imports=imports,
        exports=exports,
        locals=locals_table,
        chunks=container.chunks,
        literal_chunk=literal_chunk,
    )
