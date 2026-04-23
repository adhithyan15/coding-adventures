"""Intel HEX encoder — converts raw binary bytes to Intel HEX ROM image format.

=== What is Intel HEX? ===

Intel HEX is a text-based file format originally invented for loading programs
into EPROM chips using a "programmer" device (a piece of hardware that burns
bits into erasable ROM by applying high voltages to specific pins).

The format dates from the early 1970s — exactly the era of the Intel 8008
(1972).  When a customer wanted to store their 8008 firmware in ROM, they
would have used exactly this workflow:

  1. Write assembly code
  2. Assemble to binary
  3. Encode as Intel HEX
  4. Send the .hex file to a ROM programmer service
  5. Receive burned EPROM chips in the mail

Today you can burn an EPROM yourself with a $30 TL866 programmer and the
minipro command-line tool. The Intel HEX format is understood by every EPROM
programmer on the market — it has survived 50+ years unchanged.

=== Record Format ===

Each line in an Intel HEX file is a "record" with this structure:

  :LLAAAATTDDDDDD...CC

  :     — start code (literal colon, marks the start of a record)
  LL    — byte count (hex), number of data bytes in this record (0–255)
  AAAA  — load address (hex, 16-bit big-endian), where in ROM these bytes go
  TT    — record type (hex):
          00 = Data
          01 = End Of File
          02 = Extended Segment Address (not used — 8008 ROM fits in 16 KB)
  DD... — data bytes (LL × 2 hex chars)
  CC    — checksum (hex), two's complement of the sum of all bytes in the
          record (LL + AAAA_high + AAAA_low + TT + all DD bytes)

=== Checksum Calculation ===

The checksum is designed so that summing ALL bytes in the record (including
the checksum byte itself) yields 0x00 (mod 256). This makes verification
trivial for ROM programmer firmware: sum every byte, check for zero.

  checksum = (0x100 - (sum_of_all_data_bytes % 256)) % 256

Example for ":03 0000 00 01 02 03 CC":
  LL=0x03, AAAA=0x0000, TT=0x00, DD=[0x01, 0x02, 0x03]
  sum = 0x03 + 0x00 + 0x00 + 0x00 + 0x01 + 0x02 + 0x03 = 0x09
  checksum = (0x100 - 0x09) % 0x100 = 0xF7

=== 8008 Address Space ===

The Intel 8008 has a 14-bit address space covering 16 384 bytes (16 KB).
The address range is 0x0000–0x3FFF:

  0x0000–0x1FFF   ROM: program code (8 KB)
  0x2000–0x3FFF   RAM: static variable data (8 KB)

The OCT01 spec calls for packaging the full 16 384-byte image in Intel HEX.
ROM bytes unused by the program are padded with 0xFF (erased flash state).
RAM bytes start as 0x00 (power-on RAM state).

We use 16 bytes per data record (standard "ihex16" format, compatible with
all EPROM programmers and the Intel HEX standard).

=== Key Differences from the 4004 Packager ===

  4004: 12-bit address space (4 KB), max image 4 096 bytes
  8008: 14-bit address space (16 KB), max image 16 384 bytes

The Intel HEX format itself is identical — only the address cap differs.
"""

from __future__ import annotations

_BYTES_PER_RECORD = 16  # Standard Intel HEX record size

# Record type codes
_RT_DATA = 0x00
_RT_EOF = 0x01

# The Intel 8008 has a 14-bit address space: 0x0000–0x3FFF (16 KB).
_MAX_IMAGE_SIZE = 0x4000  # 16 KB — full 8008 address space


def _checksum(data: list[int]) -> int:
    """Compute the Intel HEX checksum for a record.

    The checksum is the two's complement of the byte-sum of all fields
    (byte count, address high, address low, record type, and data bytes).
    It is designed so that summing all bytes in the record — including the
    checksum — yields zero modulo 256.

    Parameters
    ----------
    data:
        List of integers (0–255) representing the record bytes, in order:
        [byte_count, addr_high, addr_low, record_type, data0, data1, ...]

    Returns
    -------
    int:
        The checksum byte (0–255).

    Examples
    --------
    >>> _checksum([0x03, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03])
    247  # 0xF7
    """
    return (0x100 - (sum(data) % 256)) % 256


def _data_record(address: int, chunk: bytes) -> str:
    """Format a single Intel HEX data record.

    Parameters
    ----------
    address:
        16-bit ROM address where this chunk begins (0x0000–0x3FFF for 8008).
    chunk:
        Up to 16 bytes of data to encode.

    Returns
    -------
    str:
        A single line like ``:10000000AABBCCDD...XX\\n``.
    """
    n = len(chunk)
    addr_hi = (address >> 8) & 0xFF
    addr_lo = address & 0xFF
    fields = [n, addr_hi, addr_lo, _RT_DATA] + list(chunk)
    cs = _checksum(fields)
    data_hex = "".join(f"{b:02X}" for b in chunk)
    return f":{n:02X}{addr_hi:02X}{addr_lo:02X}{_RT_DATA:02X}{data_hex}{cs:02X}\n"


def _eof_record() -> str:
    """Format the Intel HEX end-of-file record.

    The EOF record always has zero data bytes, address 0x0000, type 0x01,
    and checksum 0xFF (= two's complement of 0x01).

    Returns
    -------
    str:
        The string ``:00000001FF\\n``.
    """
    return ":00000001FF\n"


def encode_hex(binary: bytes, origin: int = 0x000) -> str:
    """Convert raw binary bytes to an Intel HEX string.

    Splits the binary into records of up to 16 bytes each, starting at
    ``origin``, then appends the mandatory end-of-file record.

    Parameters
    ----------
    binary:
        The compiled ROM image bytes.  For the Intel 8008, up to 16 384
        bytes (0x0000–0x3FFF), covering both ROM code and RAM regions.
    origin:
        ROM load address for the first byte (default 0x000 = start of ROM).
        Must be 0–65535 and ``origin + len(binary)`` must not exceed 65536.

    Returns
    -------
    str:
        A complete Intel HEX file contents (multiple lines, ending with
        the ``:00000001FF`` EOF record).

    Raises
    ------
    ValueError:
        If ``binary`` is empty, ``origin`` is out of range, or the image
        overflows the 16-bit address space.

    Examples
    --------
    >>> print(encode_hex(bytes([0x06, 0x00, 0xFF])))
    :0300000006_00FF...
    :00000001FF
    <BLANKLINE>
    """
    if not binary:
        raise ValueError("binary must be non-empty")
    if not (0 <= origin <= 0xFFFF):
        raise ValueError(f"origin must be 0–65535, got {origin:#06x}")
    if origin + len(binary) > 0x10000:
        raise ValueError(
            f"image overflows 16-bit address space: "
            f"origin={origin:#06x}, size={len(binary)}"
        )

    lines: list[str] = []
    offset = 0
    while offset < len(binary):
        chunk = binary[offset: offset + _BYTES_PER_RECORD]
        lines.append(_data_record(origin + offset, chunk))
        offset += len(chunk)

    lines.append(_eof_record())
    return "".join(lines)


def decode_hex(hex_text: str) -> tuple[int, bytes]:
    """Parse an Intel HEX string back to (origin_address, binary_bytes).

    Useful for round-trip testing and for loading HEX files into the
    Intel 8008 simulator.  Only handles Type 00 (Data) and Type 01 (EOF)
    records — the subset we generate with ``encode_hex``.

    Parameters
    ----------
    hex_text:
        A complete Intel HEX file contents as a string.

    Returns
    -------
    tuple[int, bytes]:
        ``(origin, binary)`` where ``origin`` is the lowest address seen
        across all data records and ``binary`` is the concatenated payload
        in address order.

    Raises
    ------
    ValueError:
        On malformed records, bad checksums, unsupported record types, or
        images larger than the 8008's 16 KB address space.

    Examples
    --------
    >>> origin, data = decode_hex(encode_hex(bytes([0x06, 0x00, 0xFF])))
    >>> origin
    0
    >>> data
    b'\\x06\\x00\\xff'
    """
    segments: dict[int, bytes] = {}

    for line_num, raw_line in enumerate(hex_text.splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue
        if not line.startswith(":"):
            raise ValueError(f"line {line_num}: expected ':', got {line[:1]!r}")

        try:
            record_bytes = bytes.fromhex(line[1:])
        except ValueError as exc:
            raise ValueError(f"line {line_num}: invalid hex: {exc}") from exc

        if len(record_bytes) < 5:
            raise ValueError(
                f"line {line_num}: record too short ({len(record_bytes)} bytes)"
            )

        byte_count = record_bytes[0]
        address = (record_bytes[1] << 8) | record_bytes[2]
        rec_type = record_bytes[3]

        # Validate that the record is long enough to hold byte_count data bytes
        # plus the trailing checksum byte.  Without this check, a record that
        # claims byte_count=255 but only contains a few bytes would either raise
        # an IndexError on stored_cs or silently truncate data, causing a
        # bad-data condition that bypasses checksum verification.
        expected_len = 4 + byte_count + 1  # header(4) + data(byte_count) + checksum(1)
        if len(record_bytes) < expected_len:
            raise ValueError(
                f"line {line_num}: record claims {byte_count} data bytes "
                f"but only {len(record_bytes)} total bytes present "
                f"(need {expected_len})"
            )

        data = record_bytes[4: 4 + byte_count]
        stored_cs = record_bytes[4 + byte_count]

        # Verify checksum
        fields = list(record_bytes[: 4 + byte_count])
        computed_cs = _checksum(fields)
        if computed_cs != stored_cs:
            raise ValueError(
                f"line {line_num}: checksum mismatch "
                f"(expected {computed_cs:#04x}, got {stored_cs:#04x})"
            )

        if rec_type == _RT_EOF:
            break
        if rec_type == _RT_DATA:
            segments[address] = data
        else:
            raise ValueError(
                f"line {line_num}: unsupported record type {rec_type:#04x}"
            )

    if not segments:
        return 0, b""

    origin = min(segments)
    end = max(addr + len(d) for addr, d in segments.items())

    # Guard against adversarial inputs that claim widely-separated addresses
    # (e.g., one record at 0x0000 and one at 0xFFFF) which would cause a large
    # allocation even if almost no data is present.  The 8008 address space is
    # 16 KB (0x4000 bytes), so we cap at that size.
    if (end - origin) > _MAX_IMAGE_SIZE:
        raise ValueError(
            f"decoded image too large: {end - origin} bytes "
            f"(maximum {_MAX_IMAGE_SIZE} bytes for Intel 8008 address space)"
        )

    buf = bytearray(end - origin)
    for addr, d in segments.items():
        buf[addr - origin: addr - origin + len(d)] = d
    return origin, bytes(buf)
