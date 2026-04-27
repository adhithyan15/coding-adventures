"""url-parser — RFC 1738 URL parser with relative resolution and percent-encoding.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

= Overview =

A URL (Uniform Resource Locator) is a structured string that identifies a
resource on the internet. It was first specified in RFC 1738 (1994) by Tim
Berners-Lee, and later refined in RFC 3986. The anatomy of a URL is:

    scheme://userinfo@host:port/path?query#fragment
    └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
    protocol  auth   server port route search  anchor

Think of a URL like a postal address:
  - scheme   → the delivery service (HTTP, FTP, mailto)
  - host     → the building address
  - port     → the apartment number
  - path     → which room in the apartment
  - query    → special instructions for the recipient
  - fragment → a specific paragraph on the page

= Parsing Strategy =

We use a single-pass, left-to-right scanner. This is the same approach used
in production browsers: scan once, carve off pieces as we go. The order is
carefully chosen to avoid ambiguity:

    1. Scheme  (find "://" or "scheme:" for opaque URIs)
    2. Fragment (find "#" — rightmost delimiter, grab it early)
    3. Query   (find "?" — next rightmost)
    4. Authority vs Path (find first "/" in remainder)
    5. Userinfo (find "@" in authority)
    6. Host:Port (bracket-aware splitting)

This order matters! Fragment must be extracted before query, because a
fragment can contain "?" characters. Similarly, query must be extracted
before path splitting.
"""

from __future__ import annotations

import re
from typing import Final

__version__ = "0.1.0"

# ──────────────────────────────────────────────────────────────────────
# Section 1: Error Hierarchy
# ──────────────────────────────────────────────────────────────────────
#
# Each error maps to a specific failure mode in URL parsing. We use a
# hierarchy so callers can catch UrlError for "any URL problem" or a
# specific subclass for targeted handling.


class UrlError(Exception):
    """Base class for all URL parsing errors."""


class MissingScheme(UrlError):
    """The input has no recognizable scheme (e.g., missing '://' or ':')."""


class InvalidScheme(UrlError):
    """The scheme contains characters not allowed by RFC 3986.

    Valid schemes match: [a-z][a-z0-9+.-]*
    Examples of invalid schemes: "1http", "ht tp", "http!"
    """


class InvalidPort(UrlError):
    """The port number is not a valid integer or exceeds the 16-bit range (0-65535)."""


class InvalidPercentEncoding(UrlError):
    """A percent-encoded sequence is malformed (e.g., '%G9', '%2')."""


class EmptyHost(UrlError):
    """The authority section has an '@' or port but no host."""


class RelativeWithoutBase(UrlError):
    """A relative reference was provided but no base URL to resolve against."""


# ──────────────────────────────────────────────────────────────────────
# Section 2: Constants
# ──────────────────────────────────────────────────────────────────────

# Default port numbers for well-known schemes. These are the IANA-assigned
# ports that browsers use when no explicit port is given.
#
#   http://example.com   →  port 80  (the web, since 1991)
#   https://example.com  →  port 443 (TLS-encrypted web, since 1994)
#   ftp://example.com    →  port 21  (file transfer, since 1971!)

DEFAULT_PORTS: Final[dict[str, int]] = {
    "http": 80,
    "https": 443,
    "ftp": 21,
}

# The scheme regex from RFC 3986 §3.1:
#   scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
#
# In plain English: starts with a letter, then any mix of letters, digits,
# plus, hyphen, or dot. This prevents numeric prefixes from being confused
# with IPv6 addresses or port numbers.

_SCHEME_RE: Final[re.Pattern[str]] = re.compile(r"^[a-z][a-z0-9+.\-]*$")

# Unreserved characters that should NOT be percent-encoded.
# RFC 3986 §2.3 defines these as characters that carry no special meaning
# in any URL component, so encoding them is unnecessary (though harmless).
#
# We also include '/' and '~' as unreserved for practical compatibility:
#   - '/' is the path separator and encoding it breaks path semantics
#   - '~' is commonly used in Unix home-directory URLs (~user)

_UNRESERVED: Final[frozenset[str]] = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~/"
)


# ──────────────────────────────────────────────────────────────────────
# Section 3: Percent-Encoding / Decoding
# ──────────────────────────────────────────────────────────────────────
#
# Percent-encoding is URL-land's escape mechanism. Any byte can be
# represented as %XX where XX is the uppercase hexadecimal value.
#
# Example:
#   "hello world" → "hello%20world"   (space = 0x20)
#   "café"        → "caf%C3%A9"       (UTF-8 encoding of 'é')
#
# The encoding process:
#   1. Convert the string to UTF-8 bytes
#   2. For each byte, if it's unreserved → keep as-is
#   3. Otherwise → replace with %XX
#
# The decoding process:
#   1. Scan for '%' characters
#   2. Read the next two hex digits
#   3. Convert back to the byte value
#   4. Reassemble bytes into a UTF-8 string


def percent_encode(input_str: str) -> str:
    """Encode a string using URL percent-encoding.

    Unreserved characters (A-Z, a-z, 0-9, -, _, ., ~, /) pass through
    unchanged. All other characters are encoded as %XX sequences using
    their UTF-8 byte representation.

    >>> percent_encode("hello world")
    'hello%20world'
    >>> percent_encode("a/b")
    'a/b'
    """
    # We work at the byte level because percent-encoding operates on
    # octets, not characters. A multi-byte character like '日' (U+65E5)
    # encodes to three separate %XX sequences (%E6%97%A5) because its
    # UTF-8 representation is three bytes: 0xE6, 0x97, 0xA5.
    result: list[str] = []
    for byte in input_str.encode("utf-8"):
        char = chr(byte)
        if char in _UNRESERVED:
            result.append(char)
        else:
            # Uppercase hex, zero-padded to 2 digits: 0x20 → "20"
            result.append(f"%{byte:02X}")
    return "".join(result)


def percent_decode(input_str: str) -> str:
    """Decode a percent-encoded string back to its original form.

    Each %XX sequence is converted to the corresponding byte, and the
    resulting byte sequence is decoded as UTF-8.

    >>> percent_decode("hello%20world")
    'hello world'
    >>> percent_decode("%E6%97%A5")
    '日'

    Raises InvalidPercentEncoding if a %XX sequence is truncated or
    contains non-hex characters.
    """
    # We accumulate raw bytes because a single character may span multiple
    # %XX sequences (UTF-8 multi-byte characters). Only at the end do we
    # decode the full byte sequence back to a string.
    result_bytes: list[int] = []
    i: int = 0
    while i < len(input_str):
        if input_str[i] == "%":
            # Need at least 2 more characters for the hex digits
            if i + 2 >= len(input_str):
                raise InvalidPercentEncoding(
                    f"Truncated percent-encoding at position {i}: "
                    f"'{input_str[i:]}'"
                )
            hex_digits = input_str[i + 1 : i + 3]
            # Validate that both characters are hexadecimal
            try:
                byte_val = int(hex_digits, 16)
            except ValueError:
                raise InvalidPercentEncoding(
                    f"Invalid hex digits in percent-encoding: '%{hex_digits}'"
                ) from None
            result_bytes.append(byte_val)
            i += 3
        else:
            # Regular ASCII character — convert directly to its byte value
            result_bytes.append(ord(input_str[i]))
            i += 1
    return bytes(result_bytes).decode("utf-8")


# ──────────────────────────────────────────────────────────────────────
# Section 4: Dot Segment Removal
# ──────────────────────────────────────────────────────────────────────
#
# When resolving relative URLs, we may end up with paths containing "."
# (current directory) and ".." (parent directory) segments. These must
# be normalized, just like a filesystem does with `cd`.
#
# The algorithm processes segments left-to-right, maintaining an output
# stack:
#
#   Input:  /a/b/../c/./d
#
#   Step 1: push "a"     → [a]
#   Step 2: push "b"     → [a, b]
#   Step 3: ".." → pop   → [a]
#   Step 4: push "c"     → [a, c]
#   Step 5: "." → skip   → [a, c]
#   Step 6: push "d"     → [a, c, d]
#
#   Output: /a/c/d
#
# Edge case: ".." at the root is silently ignored (you can't go above /).
# This matches browser behavior — /a/../../../b resolves to /b.


def _remove_dot_segments(path: str) -> str:
    """Remove '.' and '..' segments from a URL path.

    This implements RFC 3986 §5.2.4, the dot-segment removal algorithm.
    The path must be absolute (starting with '/') for correct results.

    >>> _remove_dot_segments("/a/b/../c")
    '/a/c'
    >>> _remove_dot_segments("/a/./b")
    '/a/b'
    >>> _remove_dot_segments("/a/../../../c")
    '/c'
    """
    # Split into segments, filtering out empty segments that arise from
    # leading '/' or consecutive '/' characters.
    segments = path.split("/")
    output: list[str] = []

    for segment in segments:
        if segment == ".":
            # Current directory — do nothing. Like a no-op in a filesystem.
            continue
        elif segment == "..":
            # Parent directory — pop the last segment if one exists.
            # We never pop past the root, so check before popping.
            if output:
                output.pop()
        else:
            output.append(segment)

    # Reconstruct the path. The first segment is always "" (from the
    # leading "/"), so joining with "/" restores the leading slash.
    result = "/".join(output)

    # Ensure the path starts with "/" for absolute paths
    if path.startswith("/") and not result.startswith("/"):
        result = "/" + result

    return result


# ──────────────────────────────────────────────────────────────────────
# Section 5: The Url Class
# ──────────────────────────────────────────────────────────────────────
#
# This is the main data structure. A parsed URL is decomposed into its
# seven components, each stored as a typed field. The class provides
# methods for serialization, relative resolution, and port lookup.


class Url:
    """A parsed URL with all its components.

    The URL is decomposed into:

        scheme://userinfo@host:port/path?query#fragment

    Each component is stored as a separate field. None means the component
    was not present in the original URL.

    The ``raw`` attribute preserves the original input string.
    """

    __slots__ = (
        "scheme",
        "userinfo",
        "host",
        "port",
        "path",
        "query",
        "fragment",
        "raw",
    )

    def __init__(
        self,
        *,
        scheme: str,
        userinfo: str | None,
        host: str | None,
        port: int | None,
        path: str,
        query: str | None,
        fragment: str | None,
        raw: str,
    ) -> None:
        self.scheme: str = scheme
        self.userinfo: str | None = userinfo
        self.host: str | None = host
        self.port: int | None = port
        self.path: str = path
        self.query: str | None = query
        self.fragment: str | None = fragment
        self.raw: str = raw

    # ── Parsing ──────────────────────────────────────────────────────

    @staticmethod
    def parse(input_str: str) -> Url:
        """Parse a URL string into its component parts.

        This is a single-pass, left-to-right parser that follows this order:

            ┌─────────────────────────────────────────────────┐
            │  1. Extract scheme (find "://" or "scheme:")     │
            │  2. Extract fragment (find "#")                  │
            │  3. Extract query (find "?")                     │
            │  4. Split authority from path (find first "/")   │
            │  5. Extract userinfo (find "@" in authority)     │
            │  6. Parse host:port (bracket-aware)              │
            └─────────────────────────────────────────────────┘

        Raises:
            MissingScheme: No scheme found in the input.
            InvalidScheme: Scheme contains invalid characters.
            InvalidPort: Port is not a valid 16-bit integer.
            EmptyHost: Authority has no host but has other components.
        """
        raw = input_str
        rest = input_str

        # ── Step 1: Extract the scheme ──────────────────────────────
        #
        # We look for "://" first (hierarchical URL like http://...).
        # If not found, we look for "scheme:path" form (opaque URI like
        # mailto:alice@example.com). The opaque form requires:
        #   - The part before ":" starts with an alpha character
        #   - The part before ":" contains no "/"

        scheme: str
        is_opaque: bool = False

        separator_pos = rest.find("://")
        if separator_pos >= 0:
            scheme = rest[:separator_pos].lower()
            rest = rest[separator_pos + 3:]
        else:
            # Try opaque URI form: scheme:path (e.g., mailto:user@host)
            colon_pos = rest.find(":")
            if colon_pos > 0:
                candidate = rest[:colon_pos]
                # Must start with alpha and contain no slashes
                if candidate[0].isalpha() and "/" not in candidate:
                    scheme = candidate.lower()
                    rest = rest[colon_pos + 1:]
                    is_opaque = True
                else:
                    raise MissingScheme(
                        f"No scheme found in URL: '{input_str}'"
                    )
            else:
                raise MissingScheme(
                    f"No scheme found in URL: '{input_str}'"
                )

        # Validate the scheme against RFC 3986 §3.1
        if not _SCHEME_RE.match(scheme):
            raise InvalidScheme(
                f"Invalid scheme '{scheme}': must match [a-z][a-z0-9+.-]*"
            )

        # ── Step 2: Extract the fragment ────────────────────────────
        #
        # The fragment is everything after the '#'. We extract it first
        # because fragments can contain '?' characters, and we don't
        # want those confused with query delimiters.

        fragment: str | None = None
        hash_pos = rest.find("#")
        if hash_pos >= 0:
            fragment = rest[hash_pos + 1:]
            rest = rest[:hash_pos]

        # ── Step 3: Extract the query ───────────────────────────────
        #
        # The query is everything after the '?' (and before the fragment,
        # which we already removed). Queries contain key=value pairs
        # separated by '&', but we store the raw string.

        query: str | None = None
        question_pos = rest.find("?")
        if question_pos >= 0:
            query = rest[question_pos + 1:]
            rest = rest[:question_pos]

        # ── For opaque URIs (like mailto:), the rest is the path ────
        if is_opaque:
            return Url(
                scheme=scheme,
                userinfo=None,
                host=None,
                port=None,
                path=rest,
                query=query,
                fragment=fragment,
                raw=raw,
            )

        # ── Step 4: Split authority from path ───────────────────────
        #
        # In a hierarchical URL, after the scheme://, the next part is
        # the authority (host, port, userinfo) followed by the path.
        # The first "/" separates them:
        #
        #   example.com:8080/path/to/resource
        #   └─────────────┘└───────────────┘
        #      authority         path

        authority: str
        path: str

        slash_pos = rest.find("/")
        if slash_pos >= 0:
            authority = rest[:slash_pos]
            path = rest[slash_pos:]
        else:
            # No path component — just authority (e.g., http://example.com)
            authority = rest
            path = "/"

        # ── Step 5: Extract userinfo ────────────────────────────────
        #
        # Userinfo appears before '@' in the authority:
        #   user:password@host:port
        #   └───────────┘
        #      userinfo
        #
        # Note: Putting passwords in URLs is deprecated (RFC 3986 §3.2.1)
        # but we still parse them for compatibility.

        userinfo: str | None = None
        at_pos = authority.find("@")
        if at_pos >= 0:
            userinfo = authority[:at_pos]
            authority = authority[at_pos + 1:]

        # ── Step 6: Parse host:port ─────────────────────────────────
        #
        # This is the trickiest part because of IPv6 addresses. IPv6
        # addresses are wrapped in brackets to distinguish colons in
        # the address from the port separator:
        #
        #   [::1]:8080     ← IPv6 localhost on port 8080
        #   [2001:db8::1]  ← IPv6 address, no port
        #   example.com:80 ← IPv4/hostname with port
        #
        # Decision tree:
        #   - Starts with '[' → IPv6: find ']', then optional ':port'
        #   - Contains ':'    → Split on LAST ':', check if port is digits
        #   - Otherwise       → Entire string is the host

        host: str | None = None
        port: int | None = None

        if authority.startswith("["):
            # IPv6 address: [address]:port
            bracket_end = authority.find("]")
            if bracket_end >= 0:
                host = authority[: bracket_end + 1]
                remaining = authority[bracket_end + 1:]
                if remaining.startswith(":"):
                    port_str = remaining[1:]
                    port = _parse_port(port_str)
        else:
            # IPv4 or hostname: check for port
            # We use the LAST colon because hostnames don't contain colons
            # (only IPv6 does, and that's handled above).
            colon_pos = authority.rfind(":")
            if colon_pos >= 0:
                potential_port = authority[colon_pos + 1:]
                # Only treat as port if all characters are digits
                if potential_port.isdigit():
                    host = authority[:colon_pos]
                    port = _parse_port(potential_port)
                else:
                    host = authority
            else:
                host = authority

        # Normalize: lowercase the host, treat empty host as None
        if host is not None:
            host = host.lower()
            if host == "":
                host = None

        return Url(
            scheme=scheme,
            userinfo=userinfo,
            host=host,
            port=port,
            path=path,
            query=query,
            fragment=fragment,
            raw=raw,
        )

    # ── Derived Properties ───────────────────────────────────────────

    def effective_port(self) -> int | None:
        """Return the effective port number for this URL.

        If an explicit port is specified, return that. Otherwise, look up
        the default port for the scheme. Returns None if neither is available.

        This mirrors browser behavior: http://example.com implicitly uses
        port 80 even though it's not written in the URL.

        >>> Url.parse("http://example.com").effective_port()
        80
        >>> Url.parse("http://example.com:9090").effective_port()
        9090
        """
        if self.port is not None:
            return self.port
        return DEFAULT_PORTS.get(self.scheme)

    def authority(self) -> str:
        """Reconstruct the authority component from its parts.

        The authority is: [userinfo@]host[:port]

        >>> Url.parse("http://user:pass@host:8080/path").authority()
        'user:pass@host:8080'
        >>> Url.parse("http://example.com/path").authority()
        'example.com'
        """
        result = ""
        if self.userinfo is not None:
            result += self.userinfo + "@"
        if self.host is not None:
            result += self.host
        if self.port is not None:
            result += f":{self.port}"
        return result

    # ── Serialization ────────────────────────────────────────────────

    def to_url_string(self) -> str:
        """Serialize the URL back to a string.

        Two forms exist depending on whether a host is present:

        With host (hierarchical):
            scheme://[userinfo@]host[:port]path[?query][#fragment]

        Without host (opaque):
            scheme:path[?query][#fragment]

        >>> Url.parse("http://example.com/path?q=1#frag").to_url_string()
        'http://example.com/path?q=1#frag'
        """
        parts: list[str] = []

        if self.host is not None:
            # Hierarchical URL with authority
            parts.append(self.scheme)
            parts.append("://")
            if self.userinfo is not None:
                parts.append(self.userinfo)
                parts.append("@")
            parts.append(self.host)
            if self.port is not None:
                parts.append(f":{self.port}")
            parts.append(self.path)
        else:
            # Opaque URI (like mailto:)
            parts.append(self.scheme)
            parts.append(":")
            parts.append(self.path)

        if self.query is not None:
            parts.append("?")
            parts.append(self.query)
        if self.fragment is not None:
            parts.append("#")
            parts.append(self.fragment)

        return "".join(parts)

    # ── Relative Resolution ──────────────────────────────────────────
    #
    # Relative resolution is how browsers turn a relative link on a page
    # into a full URL. Given a base URL and a relative reference, we
    # produce a new absolute URL.
    #
    # The algorithm comes from RFC 1808 (later formalized in RFC 3986 §5):
    #
    #   Reference         Base                    Result
    #   ─────────         ────                    ──────
    #   ""                http://a.com/b          http://a.com/b (no fragment)
    #   "#frag"           http://a.com/b          http://a.com/b#frag
    #   "http://new.com"  (any)                   http://new.com
    #   "//other.com/p"   http://a.com/x          http://other.com/p
    #   "/absolute/path"  http://a.com/x/y        http://a.com/absolute/path
    #   "relative/path"   http://a.com/x/y        http://a.com/x/relative/path
    #   "../up"           http://a.com/x/y/z      http://a.com/x/up

    def resolve(self, relative: str) -> Url:
        """Resolve a relative URL reference against this URL as the base.

        Implements RFC 3986 §5 (based on RFC 1808). The resolution rules,
        in order of priority:

        1. Empty reference → return base without fragment
        2. Fragment-only (#...) → update fragment, keep everything else
        3. Has scheme → already absolute, parse directly
        4. Starts with "//" → scheme-relative, inherit scheme
        5. Starts with "/" → absolute path, inherit scheme+authority
        6. Otherwise → relative path, merge and normalize

        >>> base = Url.parse("http://example.com/a/b/c")
        >>> base.resolve("../d").to_url_string()
        'http://example.com/a/d'
        """
        # Rule 1: Empty reference — return base without fragment
        # This is useful for <a href=""> which means "reload this page"
        if relative == "":
            return Url(
                scheme=self.scheme,
                userinfo=self.userinfo,
                host=self.host,
                port=self.port,
                path=self.path,
                query=self.query,
                fragment=None,
                raw=self.raw,
            )

        # Rule 2: Fragment-only reference
        # <a href="#section2"> keeps the page but jumps to an anchor
        if relative.startswith("#"):
            return Url(
                scheme=self.scheme,
                userinfo=self.userinfo,
                host=self.host,
                port=self.port,
                path=self.path,
                query=self.query,
                fragment=relative[1:],
                raw=relative,
            )

        # Rule 3: Has a scheme — it's already absolute
        # Check for "://" or "scheme:" pattern
        if "://" in relative:
            return Url.parse(relative)

        # Rule 4: Scheme-relative (starts with "//")
        # Inherits just the scheme from the base
        if relative.startswith("//"):
            return Url.parse(self.scheme + ":" + relative)

        # For rules 5 and 6, we need to extract query and fragment
        # from the relative reference first.
        rel_fragment: str | None = None
        rel_rest = relative
        hash_pos = rel_rest.find("#")
        if hash_pos >= 0:
            rel_fragment = rel_rest[hash_pos + 1:]
            rel_rest = rel_rest[:hash_pos]

        rel_query: str | None = None
        question_pos = rel_rest.find("?")
        if question_pos >= 0:
            rel_query = rel_rest[question_pos + 1:]
            rel_rest = rel_rest[:question_pos]

        # Rule 5: Absolute path (starts with "/")
        # Keeps the scheme and authority, replaces the path entirely
        if rel_rest.startswith("/"):
            new_path = _remove_dot_segments(rel_rest)
            return Url(
                scheme=self.scheme,
                userinfo=self.userinfo,
                host=self.host,
                port=self.port,
                path=new_path,
                query=rel_query,
                fragment=rel_fragment,
                raw=relative,
            )

        # Rule 6: Relative path — merge with base path
        #
        # "Merging" means: take the base path up to the last '/',
        # then append the relative reference. This mimics how a
        # filesystem resolves relative paths:
        #
        #   base:  /a/b/c     (last '/' is after 'b')
        #   rel:   d/e
        #   merge: /a/b/d/e
        #
        # Then we remove dot segments to normalize ".." and ".".

        base_path = self.path
        last_slash = base_path.rfind("/")
        if last_slash >= 0:
            merged = base_path[: last_slash + 1] + rel_rest
        else:
            merged = "/" + rel_rest

        new_path = _remove_dot_segments(merged)

        return Url(
            scheme=self.scheme,
            userinfo=self.userinfo,
            host=self.host,
            port=self.port,
            path=new_path,
            query=rel_query,
            fragment=rel_fragment,
            raw=relative,
        )

    def __repr__(self) -> str:
        """Developer-friendly representation for debugging."""
        return (
            f"Url(scheme={self.scheme!r}, host={self.host!r}, "
            f"port={self.port!r}, path={self.path!r})"
        )

    def __eq__(self, other: object) -> bool:
        """Two URLs are equal if all their components match."""
        if not isinstance(other, Url):
            return NotImplemented
        return (
            self.scheme == other.scheme
            and self.userinfo == other.userinfo
            and self.host == other.host
            and self.port == other.port
            and self.path == other.path
            and self.query == other.query
            and self.fragment == other.fragment
        )


# ──────────────────────────────────────────────────────────────────────
# Section 6: Helper Functions
# ──────────────────────────────────────────────────────────────────────


def _parse_port(port_str: str) -> int:
    """Parse and validate a port number string.

    Ports are 16-bit unsigned integers (0-65535). This matches the TCP/IP
    specification where port numbers fit in a 16-bit field.

    Raises InvalidPort if the string is not a valid integer or exceeds
    the range.
    """
    try:
        port = int(port_str)
    except ValueError:
        raise InvalidPort(f"Invalid port number: '{port_str}'") from None

    if port < 0 or port > 65535:
        raise InvalidPort(
            f"Port {port} out of range (must be 0-65535)"
        )

    return port
