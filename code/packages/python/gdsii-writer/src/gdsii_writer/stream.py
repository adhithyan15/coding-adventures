"""GDSII binary stream format writer.

The Calma Stream Format from 1978: every record has a 2-byte big-endian
length, a 1-byte record type, a 1-byte data type, then payload. Reals are
fixed-point 8-byte big-endian. We implement the subset needed for digital
ASIC layout: HEADER, BGNLIB, LIBNAME, UNITS, BGNSTR, STRNAME, BOUNDARY,
PATH, SREF, AREF, TEXT, ENDEL, ENDSTR, ENDLIB, plus LAYER/DATATYPE/XY/WIDTH/
SNAME/STRANS/MAG/ANGLE.
"""

from __future__ import annotations

import struct
import time as _time
from io import BytesIO

# ----------------------------------------------------------------------------
# Record types (Calma table 6 in the 1985 spec; match KLayout conventions)
# ----------------------------------------------------------------------------

HEADER     = 0x0002
BGNLIB     = 0x0102
LIBNAME    = 0x0206
UNITS      = 0x0305
ENDLIB     = 0x0400
BGNSTR     = 0x0502
STRNAME    = 0x0606
ENDSTR     = 0x0700
BOUNDARY   = 0x0800
PATH       = 0x0900
SREF       = 0x0A00
AREF       = 0x0B00
TEXT       = 0x0C00
LAYER      = 0x0D02
DATATYPE   = 0x0E02
WIDTH      = 0x0F03
XY         = 0x1003
ENDEL      = 0x1100
SNAME      = 0x1206
COLROW     = 0x1302
TEXTNODE   = 0x1400
NODE       = 0x1500
TEXTTYPE   = 0x1602
PRESENTATION = 0x1701
STRING     = 0x1906
STRANS     = 0x1A01
MAG        = 0x1B05
ANGLE      = 0x1C05


# ----------------------------------------------------------------------------
# Real number conversion (8-byte fixed-point)
# ----------------------------------------------------------------------------


def _double_to_gds_real(value: float) -> bytes:
    """Convert a Python float to GDSII 8-byte real format (signed fraction +
    7-bit excess-64 exponent base 16)."""
    if value == 0.0:
        return b"\x00" * 8

    sign = 0
    if value < 0:
        sign = 1
        value = -value

    # Find the exponent so that 1/16 <= mantissa < 1.
    # GDSII uses base-16 exponent + excess-64 bias.
    exponent = 64
    while value >= 1.0:
        value /= 16.0
        exponent += 1
    while value < 1.0 / 16.0:
        value *= 16.0
        exponent -= 1

    # 56-bit mantissa: value × 2^56
    mantissa = int(value * (1 << 56))
    if mantissa >= (1 << 56):
        # rounded up; renormalize
        mantissa >>= 4
        exponent += 1

    sign_exp = (sign << 7) | (exponent & 0x7F)
    return bytes([sign_exp]) + mantissa.to_bytes(7, "big")


# ----------------------------------------------------------------------------
# Record writer
# ----------------------------------------------------------------------------


def _record(rec_type: int, payload: bytes = b"") -> bytes:
    """Build a single GDSII record."""
    length = 4 + len(payload)
    if length > 0xFFFF:
        raise ValueError(f"record too long: {length} bytes")
    rt = (rec_type >> 8) & 0xFF
    dt = rec_type & 0xFF
    return struct.pack(">HBB", length, rt, dt) + payload


def _string_padded(s: str) -> bytes:
    """Pad a string to even length (GDSII requirement)."""
    b = s.encode("ascii", errors="replace")
    if len(b) % 2:
        b += b"\x00"
    return b


def _int2(*values: int) -> bytes:
    # GDSII 2-byte ints are big-endian. We use signed to match struct's `h`,
    # but values that overflow the signed range get wrapped to two's-complement
    # negative (e.g., STRANS bitfield 0x8000 -> -32768). The bit pattern is
    # the same.
    wrapped = tuple(v - 0x10000 if v >= 0x8000 else v for v in values)
    return struct.pack(f">{len(values)}h", *wrapped)


def _int4(*values: int) -> bytes:
    return struct.pack(f">{len(values)}i", *values)


def _real8(*values: float) -> bytes:
    return b"".join(_double_to_gds_real(v) for v in values)


# ----------------------------------------------------------------------------
# Public API
# ----------------------------------------------------------------------------


class GdsWriter:
    """Streaming GDSII binary writer.

    Open the file, write the library header, write structures (cells), close
    the library, write the file. Each structure contains polygons (BOUNDARY),
    paths (PATH), references (SREF/AREF), and text (TEXT)."""

    def __init__(
        self,
        path: str,
        library_name: str = "design",
        user_unit: float = 1e-6,
        db_unit: float = 1e-9,
    ) -> None:
        # NOTE: store as `_filepath` to avoid colliding with the `path()` method below.
        self._filepath = path
        self.library_name = library_name
        self.user_unit = user_unit
        self.db_unit = db_unit
        self._buf = BytesIO()
        self._closed = False
        self._in_struct = False

    def __enter__(self) -> GdsWriter:
        self.write_header()
        return self

    def __exit__(self, *exc: object) -> None:
        if not self._closed:
            self.write_footer()
        self.flush()

    # ----- Library header / footer -----

    def write_header(self) -> None:
        # HEADER (version 600 = "5.0", widely accepted)
        self._buf.write(_record(HEADER, _int2(600)))
        # BGNLIB: 12 int2 values for last-modified + last-accessed timestamps
        now = _time.localtime()
        ts = _int2(
            now.tm_year, now.tm_mon, now.tm_mday,
            now.tm_hour, now.tm_min, now.tm_sec,
        )
        self._buf.write(_record(BGNLIB, ts + ts))
        # LIBNAME
        self._buf.write(_record(LIBNAME, _string_padded(self.library_name)))
        # UNITS: user_unit (relative to db_unit) + db_unit (in meters)
        self._buf.write(
            _record(UNITS, _real8(self.db_unit / self.user_unit, self.db_unit))
        )

    def write_footer(self) -> None:
        # Auto-close any open structure (graceful shutdown).
        if self._in_struct:
            self.end_structure()
        self._buf.write(_record(ENDLIB))
        self._closed = True

    def flush(self) -> None:
        with open(self._filepath, "wb") as f:
            f.write(self._buf.getvalue())

    # ----- Structures (cells) -----

    def begin_structure(self, name: str) -> None:
        if self._in_struct:
            raise RuntimeError("cannot begin_structure inside another structure")
        now = _time.localtime()
        ts = _int2(
            now.tm_year, now.tm_mon, now.tm_mday,
            now.tm_hour, now.tm_min, now.tm_sec,
        )
        self._buf.write(_record(BGNSTR, ts + ts))
        self._buf.write(_record(STRNAME, _string_padded(name)))
        self._in_struct = True

    def end_structure(self) -> None:
        if not self._in_struct:
            raise RuntimeError("end_structure without matching begin_structure")
        self._buf.write(_record(ENDSTR))
        self._in_struct = False

    # ----- Elements -----

    def boundary(
        self,
        layer: int,
        datatype: int,
        points: list[tuple[float, float]],
    ) -> None:
        """A polygon. Points in user units; converted to DBU via user/db ratio.
        Polygon must close: first point == last point."""
        if not self._in_struct:
            raise RuntimeError("boundary outside structure")
        if len(points) < 4:
            raise ValueError("BOUNDARY requires >= 4 points (polygon must close)")
        if points[0] != points[-1]:
            points = [*points, points[0]]
        scale = self.user_unit / self.db_unit
        coords: list[int] = []
        for x, y in points:
            coords.append(int(round(x * scale)))
            coords.append(int(round(y * scale)))
        self._buf.write(_record(BOUNDARY))
        self._buf.write(_record(LAYER, _int2(layer)))
        self._buf.write(_record(DATATYPE, _int2(datatype)))
        self._buf.write(_record(XY, _int4(*coords)))
        self._buf.write(_record(ENDEL))

    def path(
        self,
        layer: int,
        datatype: int,
        width: float,
        points: list[tuple[float, float]],
    ) -> None:
        """A wire (polyline + width)."""
        if not self._in_struct:
            raise RuntimeError("path outside structure")
        if len(points) < 2:
            raise ValueError("PATH requires >= 2 points")
        scale = self.user_unit / self.db_unit
        coords = []
        for x, y in points:
            coords.append(int(round(x * scale)))
            coords.append(int(round(y * scale)))
        width_dbu = int(round(width * scale))
        self._buf.write(_record(PATH))
        self._buf.write(_record(LAYER, _int2(layer)))
        self._buf.write(_record(DATATYPE, _int2(datatype)))
        self._buf.write(_record(WIDTH, _int4(width_dbu)))
        self._buf.write(_record(XY, _int4(*coords)))
        self._buf.write(_record(ENDEL))

    def sref(
        self,
        cell_name: str,
        x: float,
        y: float,
        angle_deg: float = 0.0,
        mag: float = 1.0,
        reflect: bool = False,
    ) -> None:
        """A reference (instance) of another cell."""
        if not self._in_struct:
            raise RuntimeError("sref outside structure")
        scale = self.user_unit / self.db_unit
        self._buf.write(_record(SREF))
        self._buf.write(_record(SNAME, _string_padded(cell_name)))
        if angle_deg != 0.0 or mag != 1.0 or reflect:
            strans = 0x8000 if reflect else 0
            self._buf.write(_record(STRANS, _int2(strans)))
            if mag != 1.0:
                self._buf.write(_record(MAG, _real8(mag)))
            if angle_deg != 0.0:
                self._buf.write(_record(ANGLE, _real8(angle_deg)))
        self._buf.write(
            _record(XY, _int4(int(round(x * scale)), int(round(y * scale))))
        )
        self._buf.write(_record(ENDEL))

    def text(
        self,
        layer: int,
        text_type: int,
        x: float,
        y: float,
        text: str,
    ) -> None:
        """A text label (typically used for pin labels)."""
        if not self._in_struct:
            raise RuntimeError("text outside structure")
        scale = self.user_unit / self.db_unit
        self._buf.write(_record(TEXT))
        self._buf.write(_record(LAYER, _int2(layer)))
        self._buf.write(_record(TEXTTYPE, _int2(text_type)))
        self._buf.write(
            _record(XY, _int4(int(round(x * scale)), int(round(y * scale))))
        )
        self._buf.write(_record(STRING, _string_padded(text)))
        self._buf.write(_record(ENDEL))
