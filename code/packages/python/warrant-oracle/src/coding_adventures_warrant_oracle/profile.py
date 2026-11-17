"""``corpus.profile``: the neutral line file of VER04r section 8.4.

The knobs of a v2 corpus live in one data file, read by the compiler and by this
oracle's independent reader and by nothing else::

    budget 16777216
    chunk_size 65536
    slim_stride auto
    max_ir_cost 32768
    fault multiply generator-shift
    accept_blind multiply generator-shift

``max_ir_cost`` is the **only** way to raise the per-expression IR cost bound above
its default of 2,000 (ceiling 65,536); no command-line flag or environment
variable can (section 4.2 rule 6: an override is a second place for the limit to
live, and the report could not show it).  The reader is strict: LF only, one
value per key, no duplicates, no unknown keys, ranges checked, and (section 16.4
item 7, closed by slice S4 part 2) no token that is empty or not printable ASCII, no repeated ``fault`` or
``slim_chunk`` line, no ``budget 0`` and no ``slim_stride 0``.

``accept_blind <op> <fault>`` (owner decision [D15], section 16.8) excuses a fault that
every candidate claim is blind to.  The reader checks that the line repeats a ``fault``
line of the same file and does not repeat itself; that the fault really is all-blind is
checked against the document (:func:`.faults.derive_controls`).
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from .errors import OracleError

DEFAULT_MAX_IR_COST = 2_000
MAX_IR_COST_CEILING = 65_536
MAX_PROFILE_BYTES = 64 * 1024
# A token is printable ASCII only: no space, tab, NBSP, NEL, control character or BOM can hide
# in a name, and the generator's reader applies the same byte rule.
_TOKEN = re.compile(r"[!-~]+")


@dataclass
class Profile:
    budget: int = 1 << 24
    chunk_size: int = 1 << 16
    slim_stride: str | int = "auto"
    max_ir_cost: int = DEFAULT_MAX_IR_COST
    slim_chunks: list = field(default_factory=list)  # [(claim, index)]
    faults: list = field(default_factory=list)  # [(op, fault)]
    accept_blind: list = field(default_factory=list)  # [(op, fault)]


def _int(text: str) -> int:
    if (
        not text.isascii()
        or not text.isdigit()
        or (len(text) > 1 and text[0] == "0")
        or len(text) > 20
    ):
        raise OracleError("corpus", "a profile value is a canonical decimal integer")
    value = int(text)
    if value >= 1 << 64:
        raise OracleError("corpus", "a profile value is at most 2^64 - 1")
    return value


def parse_profile(raw: bytes) -> Profile:
    """Parse a ``corpus.profile``; any irregularity is a ``corpus`` error."""
    if len(raw) > MAX_PROFILE_BYTES:
        raise OracleError("limit", "corpus.profile exceeds its size cap")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise OracleError("corpus", "corpus.profile is not UTF-8") from exc
    if "\r" in text:
        raise OracleError("corpus", "corpus.profile must use LF line endings")
    prof = Profile()
    seen: set[str] = set()
    for line in text.split("\n"):
        if not line:
            continue
        parts = line.split(" ")
        if not all(_TOKEN.fullmatch(token) for token in parts):
            raise OracleError("corpus", "a profile token is empty or not printable ASCII")
        key = parts[0]
        if key in ("budget", "chunk_size", "slim_stride", "max_ir_cost"):
            if key in seen or len(parts) != 2:
                raise OracleError("corpus", "a profile key takes one value and occurs once")
            seen.add(key)
            if key == "slim_stride":
                prof.slim_stride = "auto" if parts[1] == "auto" else _int(parts[1])
                if prof.slim_stride == 0:
                    raise OracleError("corpus", "slim_stride 0 is refused (a positive integer)")
            else:
                setattr(prof, key, _int(parts[1]))
                if key == "budget" and prof.budget == 0:
                    raise OracleError("corpus", "budget 0 is refused")
        elif key == "slim_chunk" and len(parts) == 3:
            row = (parts[1], _int(parts[2]))
            if row in prof.slim_chunks:
                raise OracleError("corpus", "a slim_chunk line is repeated")
            prof.slim_chunks.append(row)
        elif key == "fault" and len(parts) == 3:
            row = (parts[1], parts[2])
            if row in prof.faults:
                raise OracleError("corpus", "a fault line is repeated")
            prof.faults.append(row)
        elif key == "accept_blind" and len(parts) == 3:
            row = (parts[1], parts[2])
            if row in prof.accept_blind:
                raise OracleError("corpus", "an accept_blind line is repeated")
            prof.accept_blind.append(row)
        else:
            raise OracleError("corpus", "an unknown or malformed profile line")
    if any(row not in prof.faults for row in prof.accept_blind):
        raise OracleError("corpus", "an accept_blind line must repeat a fault line of this profile")
    if not DEFAULT_MAX_IR_COST <= prof.max_ir_cost <= MAX_IR_COST_CEILING:
        raise OracleError("corpus", "max_ir_cost is 2,000..65,536")
    cs = prof.chunk_size
    if cs & (cs - 1) or not (1 << 10) <= cs <= (1 << 16):
        raise OracleError("corpus", "chunk_size is a power of two in 2^10..2^16")
    return prof
