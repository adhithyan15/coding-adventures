"""Binary serialisation for IIRModule.

Format (little-endian throughout)
----------------------------------

Header (12 bytes):
    4 bytes  magic      0x49 0x49 0x52 0x00  (b"IIR\\x00")
    2 bytes  version    0x01 0x00
    4 bytes  fn_count   number of IIRFunction records
    2 bytes  name_len   length of module name in bytes
    N bytes  name       module name (UTF-8)
    2 bytes  lang_len   length of language string
    M bytes  language
    2 bytes  ep_len     length of entry_point string (0 = no entry point)
    P bytes  entry_point

For each IIRFunction:
    2 bytes  name_len, N bytes name
    2 bytes  return_type_len, N bytes return_type
    1 byte   param_count
    For each param:
        2 bytes name_len, N bytes param_name
        2 bytes type_len, N bytes type_hint
    4 bytes  instr_count
    1 byte   register_count
    1 byte   type_status  (0=UNTYPED, 1=PARTIALLY_TYPED, 2=FULLY_TYPED)
    For each IIRInstr:
        2 bytes  op_len, N bytes op
        1 byte   has_dest (0 or 1)
        If has_dest: 2 bytes dest_len, N bytes dest
        2 bytes  type_hint_len, N bytes type_hint
        1 byte   src_count
        For each src operand:
            1 byte  kind  (0=str, 1=int, 2=float, 3=bool)
            If kind==0: 2 bytes len, N bytes str
            If kind==1: 8 bytes signed int64
            If kind==2: 8 bytes IEEE 754 float64
            If kind==3: 1 byte (0=False, 1=True)

Note: observation fields (observed_type, observation_count, deopt_anchor)
are NOT serialised — they are runtime state that accumulates fresh each run.

Usage::

    raw = serialise(module)
    module2 = deserialise(raw)
    assert module == module2
"""

from __future__ import annotations

import struct

from interpreter_ir.function import FunctionTypeStatus, IIRFunction
from interpreter_ir.instr import IIRInstr
from interpreter_ir.module import IIRModule

_MAGIC = b"IIR\x00"
_VERSION = (1, 0)

_STATUS_TO_BYTE = {
    FunctionTypeStatus.UNTYPED: 0,
    FunctionTypeStatus.PARTIALLY_TYPED: 1,
    FunctionTypeStatus.FULLY_TYPED: 2,
}
_BYTE_TO_STATUS = {v: k for k, v in _STATUS_TO_BYTE.items()}


# ---------------------------------------------------------------------------
# Serialise
# ---------------------------------------------------------------------------

class _Writer:
    def __init__(self) -> None:
        self._buf: list[bytes] = []

    def u8(self, v: int) -> None:
        self._buf.append(struct.pack("<B", v))

    def u16(self, v: int) -> None:
        self._buf.append(struct.pack("<H", v))

    def u32(self, v: int) -> None:
        self._buf.append(struct.pack("<I", v))

    def i64(self, v: int) -> None:
        self._buf.append(struct.pack("<q", v))

    def f64(self, v: float) -> None:
        self._buf.append(struct.pack("<d", v))

    def str_(self, s: str) -> None:
        encoded = s.encode("utf-8")
        self.u16(len(encoded))
        self._buf.append(encoded)

    def bytes_(self) -> bytes:
        return b"".join(self._buf)


def serialise(module: IIRModule) -> bytes:
    """Serialise an ``IIRModule`` to bytes."""
    w = _Writer()
    w._buf.append(_MAGIC)
    w.u8(_VERSION[0])
    w.u8(_VERSION[1])
    w.u32(len(module.functions))
    w.str_(module.name)
    w.str_(module.language)
    ep = module.entry_point or ""
    w.str_(ep)

    for fn in module.functions:
        _write_function(w, fn)

    return w.bytes_()


def _write_function(w: _Writer, fn: IIRFunction) -> None:
    w.str_(fn.name)
    w.str_(fn.return_type)
    w.u8(len(fn.params))
    for param_name, param_type in fn.params:
        w.str_(param_name)
        w.str_(param_type)
    w.u32(len(fn.instructions))
    w.u8(fn.register_count)
    w.u8(_STATUS_TO_BYTE[fn.type_status])
    for instr in fn.instructions:
        _write_instr(w, instr)


def _write_instr(w: _Writer, instr: IIRInstr) -> None:
    w.str_(instr.op)
    if instr.dest is not None:
        w.u8(1)
        w.str_(instr.dest)
    else:
        w.u8(0)
    w.str_(instr.type_hint)
    w.u8(len(instr.srcs))
    for src in instr.srcs:
        if isinstance(src, bool):
            w.u8(3)
            w.u8(1 if src else 0)
        elif isinstance(src, int):
            w.u8(1)
            w.i64(src)
        elif isinstance(src, float):
            w.u8(2)
            w.f64(src)
        else:
            w.u8(0)
            w.str_(src)


# ---------------------------------------------------------------------------
# Deserialise
# ---------------------------------------------------------------------------

class _Reader:
    def __init__(self, data: bytes) -> None:
        self._data = data
        self._pos = 0

    def _read(self, n: int) -> bytes:
        if self._pos + n > len(self._data):
            raise ValueError(
                f"unexpected end of data at offset {self._pos} "
                f"(need {n} bytes, have {len(self._data) - self._pos})"
            )
        chunk = self._data[self._pos : self._pos + n]
        self._pos += n
        return chunk

    def u8(self) -> int:
        return struct.unpack("<B", self._read(1))[0]

    def u16(self) -> int:
        return struct.unpack("<H", self._read(2))[0]

    def u32(self) -> int:
        return struct.unpack("<I", self._read(4))[0]

    def i64(self) -> int:
        return struct.unpack("<q", self._read(8))[0]

    def f64(self) -> float:
        return struct.unpack("<d", self._read(8))[0]

    def str_(self) -> str:
        length = self.u16()
        return self._read(length).decode("utf-8")


def deserialise(data: bytes) -> IIRModule:
    """Deserialise bytes produced by :func:`serialise` back to an ``IIRModule``."""
    r = _Reader(data)

    magic = r._read(4)
    if magic != _MAGIC:
        raise ValueError(f"invalid magic bytes: {magic!r} (expected {_MAGIC!r})")

    major = r.u8()
    minor = r.u8()
    if (major, minor) != _VERSION:
        raise ValueError(f"unsupported version {major}.{minor}")

    fn_count = r.u32()
    name = r.str_()
    language = r.str_()
    ep_raw = r.str_()
    entry_point = ep_raw if ep_raw else None

    functions = [_read_function(r) for _ in range(fn_count)]

    return IIRModule(
        name=name,
        functions=functions,
        entry_point=entry_point,
        language=language,
    )


def _read_function(r: _Reader) -> IIRFunction:
    name = r.str_()
    return_type = r.str_()
    param_count = r.u8()
    params = [(r.str_(), r.str_()) for _ in range(param_count)]
    instr_count = r.u32()
    register_count = r.u8()
    type_status = _BYTE_TO_STATUS[r.u8()]
    instructions = [_read_instr(r) for _ in range(instr_count)]
    return IIRFunction(
        name=name,
        params=params,
        return_type=return_type,
        instructions=instructions,
        register_count=register_count,
        type_status=type_status,
    )


def _read_instr(r: _Reader) -> IIRInstr:
    op = r.str_()
    has_dest = r.u8()
    dest = r.str_() if has_dest else None
    type_hint = r.str_()
    src_count = r.u8()
    srcs: list = []
    for _ in range(src_count):
        kind = r.u8()
        if kind == 0:
            srcs.append(r.str_())
        elif kind == 1:
            srcs.append(r.i64())
        elif kind == 2:
            srcs.append(r.f64())
        elif kind == 3:
            srcs.append(bool(r.u8()))
        else:
            raise ValueError(f"unknown operand kind byte: {kind}")
    return IIRInstr(op=op, dest=dest, srcs=srcs, type_hint=type_hint)
