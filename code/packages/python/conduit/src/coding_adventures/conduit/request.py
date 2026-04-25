"""
request — HTTP request wrapper for Conduit handlers.

``Request`` wraps the ``env`` dict that Rust builds for every incoming HTTP
request. It provides a clean, typed interface to the request data: method,
path, query params, route params, headers, and body.

The ``env`` dict keys mirror what the Ruby Conduit uses (and what Rack uses),
so the same mental model applies:

    env["REQUEST_METHOD"]           → "GET", "POST", …
    env["PATH_INFO"]                → "/hello/alice"
    env["QUERY_STRING"]             → "page=1&sort=asc"
    env["conduit.route_params"]     → {"name": "alice"}
    env["conduit.query_params"]     → {"page": "1", "sort": "asc"}
    env["conduit.headers"]          → {"content-type": "application/json"}
    env["conduit.body"]             → '{"ping":"pong"}'
    env["conduit.content_type"]     → "application/json"  (may be absent)
    env["conduit.content_length"]   → 15  (may be absent)

This class is analogous to Ruby's ``CodingAdventures::Conduit::Request``.
"""

from __future__ import annotations

import json
from urllib.parse import parse_qs

from .halt_exception import HaltException

# 10 MB cap on JSON bodies — prevents DoS via deeply-nested or huge payloads.
# Deeply-nested JSON can exhaust Python's recursion stack; very large JSON
# bodies consume unbounded memory during parsing. 10 MB is generous for any
# legitimate API use-case while blocking trivial amplification attacks.
_MAX_JSON_BYTES = 10 * 1024 * 1024


class Request:
    """Read-only view of an incoming HTTP request.

    Constructed by ``NativeServer`` from the ``env`` dict that Rust passes
    to every Python dispatch method. All properties are memoized — parsing
    happens at most once per request.

    Note: ``HandlerContext`` delegates unknown attribute lookups here via
    ``__getattr__``, so ``ctx.path`` and ``ctx.params`` work directly without
    going through ``ctx.request.path``.
    """

    def __init__(self, env: dict) -> None:
        self._env = env
        self._json_cache: object = (
            _SENTINEL  # parsed JSON body; SENTINEL = not yet parsed
        )
        self._form_cache: dict[str, str] | None = None

    # ── Core HTTP attributes ────────────────────────────────────────────────

    @property
    def method(self) -> str:
        """HTTP method in upper case: ``"GET"``, ``"POST"``, ``"PUT"``, …"""
        return self._env["REQUEST_METHOD"]

    @property
    def path(self) -> str:
        """URL path without query string: ``"/hello/alice"``."""
        return self._env["PATH_INFO"]

    @property
    def query_string(self) -> str:
        """Raw query string, empty string if absent: ``"page=1&sort=asc"``."""
        return self._env.get("QUERY_STRING", "")

    # ── Parameter dicts ─────────────────────────────────────────────────────

    @property
    def params(self) -> dict[str, str]:
        """Route named parameters captured by the Rust router.

        For a route ``GET /hello/<name>`` matched by ``/hello/alice``,
        ``params`` is ``{"name": "alice"}``.
        """
        return self._env.get("conduit.route_params", {})

    @property
    def query_params(self) -> dict[str, str]:
        """Parsed query-string parameters.

        For ``/search?q=hello&page=2``, this returns
        ``{"q": "hello", "page": "2"}``. Only the *first* value per key is
        kept (multi-value keys are not supported in this interface).
        """
        return self._env.get("conduit.query_params", {})

    # ── Headers ─────────────────────────────────────────────────────────────

    @property
    def headers(self) -> dict[str, str]:
        """All request headers with lower-cased names.

        Example: ``{"content-type": "application/json", "accept": "*/*"}``.
        """
        return self._env.get("conduit.headers", {})

    def header(self, name: str) -> str | None:
        """Return a single header value by name (case-insensitive)."""
        return self.headers.get(name.lower())

    # ── Body ────────────────────────────────────────────────────────────────

    @property
    def body(self) -> str:
        """Raw request body as a string. Empty string if no body."""
        return self._env.get("conduit.body", "")

    def json(self) -> object:
        """Parse the request body as JSON.

        Returns the parsed Python value (dict, list, str, int, …).
        Memoized — parsing happens only once per request.

        Raises ``HaltException(400)`` on invalid JSON instead of propagating
        the raw ``json.JSONDecodeError``. This prevents internal parser
        details from leaking into HTTP error responses.

        Raises ``HaltException(413)`` if the body exceeds ``_MAX_JSON_BYTES``
        to prevent DoS via unbounded deeply-nested JSON payloads (CWE-400).
        """
        if self._json_cache is not _SENTINEL:
            return self._json_cache
        if len(self.body) > _MAX_JSON_BYTES:
            raise HaltException(
                413,
                "request body too large",
                {"content-type": "text/plain; charset=utf-8"},
            )
        try:
            self._json_cache = json.loads(self.body)
            return self._json_cache
        except json.JSONDecodeError as err:
            raise HaltException(
                400,
                "invalid JSON body",
                {"content-type": "text/plain; charset=utf-8"},
            ) from err

    def form(self) -> dict[str, str]:
        """Parse the request body as URL-encoded form data.

        For ``application/x-www-form-urlencoded`` bodies like
        ``"name=alice&age=30"``, this returns ``{"name": "alice", "age": "30"}``.

        Multi-value keys keep only the *first* value. Memoized.
        """
        if self._form_cache is None:
            raw = parse_qs(self.body, keep_blank_values=True)
            # parse_qs returns {key: [val1, val2, ...]}; we keep only the first.
            self._form_cache = {k: v[0] for k, v in raw.items()}
        return self._form_cache

    # ── Connection metadata ─────────────────────────────────────────────────

    @property
    def content_type(self) -> str | None:
        """Value of the Content-Type header, or ``None`` if absent."""
        return self._env.get("conduit.content_type")

    @property
    def content_length(self) -> int | None:
        """Value of Content-Length as an integer, or ``None`` if absent."""
        return self._env.get("conduit.content_length")

    @property
    def remote_addr(self) -> str:
        """Client IP address: ``"127.0.0.1"``."""
        return self._env.get("REMOTE_ADDR", "")

    def __getitem__(self, key: str) -> object:
        """Raw access to the env dict for advanced use."""
        return self._env[key]


# Sentinel used to distinguish "not yet parsed" from None in JSON cache.
_SENTINEL = object()
