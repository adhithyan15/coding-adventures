"""
halt_exception — short-circuit signal for Conduit response helpers.

``HaltException`` is the Python equivalent of Ruby's ``HaltError``. It is
raised by every response helper (``ctx.json``, ``ctx.html``, ``ctx.text``,
``ctx.halt``, ``ctx.redirect``) to immediately exit a handler without
returning a value.

The exception is caught by ``NativeServer``'s dispatch methods *before*
returning to Rust. Rust never sees it — only the ``[status, headers, body]``
list that the dispatch method extracts from it.

Example lifetime:

    handler raises HaltException(200, '{"ok":true}',
                                 {"content-type": "application/json"})
        ↓
    NativeServer.native_dispatch_route catches it
        ↓
    converts to [200, [["content-type", "application/json"]], '{"ok":true}']
        ↓
    Rust receives that list and builds the HTTP response

Design note: inheriting from ``Exception`` (not ``BaseException``) means
``HaltException`` is caught by broad ``except Exception`` handlers in user
code. That is intentional — user code should generally not suppress halts,
but it is not catastrophic if it does (the ``finally`` block pattern still
works for cleanup).
"""

from __future__ import annotations


class HaltException(Exception):
    """Short-circuit a Conduit handler with an explicit HTTP response.

    Raised by every response helper (``ctx.json``, ``ctx.html``, etc.).
    Never cross the Rust boundary — only ``[status, headers, body]`` does.

    Attributes:
        status: HTTP status code (integer, e.g. 200, 404, 503).
        body: Response body as a plain string.
        halt_headers: List of ``[name, value]`` header pairs (both strings).
    """

    def __init__(
        self,
        status: int,
        body: str = "",
        headers: dict[str, str] | list[list[str]] | None = None,
    ) -> None:
        super().__init__(f"halt {status}")
        self.status: int = int(status)
        self.body: str = str(body)
        # Normalize headers to a list of [name, value] pairs — this is the
        # wire format that Rust expects when parsing the response list.
        self.halt_headers: list[list[str]] = _normalize_headers(headers)

    def to_response(self) -> list:
        """Convert to the [status, headers, body] list that Rust reads."""
        return [self.status, self.halt_headers, self.body]


def _normalize_headers(
    headers: dict[str, str] | list[list[str]] | None,
) -> list[list[str]]:
    """Convert any header representation to [[name, val], ...] form.

    Accepts:
    - ``None`` or empty dict/list → ``[]``
    - ``{"content-type": "..."}`` dict → ``[["content-type", "..."]]``
    - ``[["content-type", "..."]]`` list → unchanged

    Always returns a list of two-element string lists so Rust's
    ``parse_header_pairs`` has a consistent format to work with.
    """
    if not headers:
        return []
    if isinstance(headers, dict):
        pairs = [[str(k), str(v)] for k, v in headers.items()]
    else:
        # Already a list of pairs — coerce elements to strings for safety.
        pairs = [[str(pair[0]), str(pair[1])] for pair in headers]
    # Strip CRLF from header values to prevent HTTP response splitting (CWE-113).
    return [[k, v.replace("\r", "").replace("\n", "")] for k, v in pairs]
