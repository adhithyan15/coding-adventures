"""
RESP type definitions.

The RESP protocol represents values as a small set of types.  In Python we
map these to native types where possible:

  RESP type           Python type        Notes
  ─────────────────────────────────────────────────────────────
  Simple String       str                Short status messages like "OK"
  Error               RespError          Carries error_type and detail
  Integer             int                Signed 64-bit integer
  Bulk String         bytes              Binary-safe string (None = null)
  Null Bulk String    None               Key does not exist (GET miss)
  Array               list               Ordered elements, any RESP types
  Null Array          None               Some commands return null arrays

RespValue is a union type that covers all these possibilities.
"""

from __future__ import annotations


class RespError:
    """
    A RESP Error reply from the server.

    By Redis convention, the error message begins with an all-caps error
    type (e.g. "ERR", "WRONGTYPE", "NOSCRIPT") followed by a space and
    a human-readable description.

    Examples:
        RespError("ERR unknown command 'foo'")
            .error_type  →  "ERR"
            .detail      →  "unknown command 'foo'"
            .message     →  "ERR unknown command 'foo'"

        RespError("WRONGTYPE Operation against a key holding the wrong kind of value")
            .error_type  →  "WRONGTYPE"

    The error_type is the first whitespace-delimited word.  If the message
    has no space, the whole message is the error_type and detail is empty.
    """

    def __init__(self, message: str) -> None:
        self.message = message
        # Split once on the first space to separate type from detail.
        parts = message.split(" ", 1)
        self._error_type = parts[0]
        self._detail = parts[1] if len(parts) > 1 else ""

    @property
    def error_type(self) -> str:
        """The error class, e.g. 'ERR', 'WRONGTYPE', 'NOSCRIPT'."""
        return self._error_type

    @property
    def detail(self) -> str:
        """The human-readable description after the error type."""
        return self._detail

    def __repr__(self) -> str:
        return f"RespError({self.message!r})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, RespError):
            return self.message == other.message
        return NotImplemented

    def __hash__(self) -> int:
        return hash(("RespError", self.message))


# RespValue is the union of all types that can appear in a RESP stream.
# Using a type alias keeps function signatures readable.
#
#   str    → decoded from Simple String (+...)
#   RespError → decoded from Error (-...)
#   int    → decoded from Integer (:...)
#   bytes  → decoded from Bulk String ($...)
#   None   → decoded from Null Bulk String ($-1\r\n) or Null Array (*-1\r\n)
#   list   → decoded from Array (*...) where elements are themselves RespValue
type RespValue = str | RespError | int | bytes | None | list  # type: ignore[valid-type]
