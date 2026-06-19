#!/usr/bin/env python3
"""offline_guard.py — a network-egress tripwire that PROVES a block of code made no
online call (MYCIN-2026 board-exam, the zero-online-call invariant).

WHY THIS EXISTS
---------------
The board-exam thesis is that MYCIN answers every question with **no ONLINE model
call** — the only model permitted is a *local, in-memory* one that turns prose into
an ADJ program; the engine that ANSWERS is the native CPU reasoner over the grounded
knowledge graph. "We don't call out" is easy to claim and hard to prove. This module
makes it a hard, testable fact: run the answer path inside `no_network()` and ANY
attempt to open a socket (an HTTP call to an LLM API, a DNS lookup, anything) raises
`OnlineCallError` and fails the run. Proof by construction, not by assertion.

HOW IT WORKS (and its honest limits)
------------------------------------
A context manager monkeypatches the three places Python opens an outbound
connection — `socket.socket.connect`, `socket.socket.connect_ex`, and
`socket.create_connection` — to raise instead. That covers urllib / requests /
http.client / any MLX or HTTP client built on the standard socket layer (which is
all of them in practice). It deliberately does NOT block:

  * `AF_UNIX` sockets (local IPC — e.g. talking to a subprocess) and
  * loopback (127.0.0.1 / ::1 / localhost) connections,

because a local model server or the adj-lang-cli subprocess is exactly the kind of
*offline* machinery the invariant permits. The guard is about EGRESS to the network,
not about all I/O. It is a tripwire for the common case, not a kernel-level sandbox;
a determined `os.system("curl …")` would slip past it. For the board harness — pure
Python + a local subprocess + (optionally) an in-process MLX model — it is exactly
the right granularity, and it is generic enough to drop around any code path whose
"no online call" property you want to enforce.

USAGE
-----
    from offline_guard import no_network, OnlineCallError

    with no_network():
        answer = answer_offline(question)      # raises if anything dials out

    # or as a decorator
    @no_network()
    def scored(): ...
"""

from __future__ import annotations

import functools
import socket
from contextlib import ContextDecorator

# Loopback hosts a local model server / IPC may legitimately use while still being
# "offline" (no traffic leaves the machine). Everything else is egress and is blocked.
_LOOPBACK = {"127.0.0.1", "::1", "localhost", "0.0.0.0", ""}


class OnlineCallError(RuntimeError):
    """Raised when code inside `no_network()` tries to open an outbound connection —
    the violation of the zero-online-call invariant we are proving the absence of."""


def _is_loopback(address) -> bool:
    """A best-effort check that a connect target is local (loopback or a UNIX path).
    UNIX-domain targets are a bare string path; INET targets are an (host, port) tuple."""
    if isinstance(address, str):
        return True  # AF_UNIX path — local IPC, never network egress
    if isinstance(address, tuple) and address:
        host = address[0]
        return host in _LOOPBACK
    return False


class no_network(ContextDecorator):  # noqa: N801 (reads as a verb at the call site)
    """Block all non-loopback outbound socket connections for the duration of the
    block; record how many egress attempts were tripped (normally 0). Usable as a
    context manager or a decorator. Reentrant-safe via save/restore of the originals."""

    def __init__(self) -> None:
        self.attempts: list[object] = []
        self._saved: dict[str, object] = {}

    # -- the patched connect surface --------------------------------------------
    def _guard(self, target):
        self.attempts.append(target)
        raise OnlineCallError(
            f"online call blocked: attempted outbound connection to {target!r} inside "
            f"no_network() — the board answer path must make zero online calls"
        )

    def __enter__(self) -> "no_network":
        guard = self  # close over the live recorder

        self._saved = {
            "connect": socket.socket.connect,
            "connect_ex": socket.socket.connect_ex,
            "create_connection": socket.create_connection,
        }

        def connect(self, address):  # noqa: ANN001 (mirrors the stdlib signature)
            if _is_loopback(address):
                return guard._saved["connect"](self, address)
            return guard._guard(address)

        def connect_ex(self, address):  # noqa: ANN001
            if _is_loopback(address):
                return guard._saved["connect_ex"](self, address)
            return guard._guard(address)

        def create_connection(address, *args, **kwargs):  # noqa: ANN001
            if _is_loopback(address):
                return guard._saved["create_connection"](address, *args, **kwargs)
            return guard._guard(address)

        socket.socket.connect = connect
        socket.socket.connect_ex = connect_ex
        socket.create_connection = create_connection
        return self

    def __exit__(self, *exc) -> bool:
        socket.socket.connect = self._saved["connect"]
        socket.socket.connect_ex = self._saved["connect_ex"]
        socket.create_connection = self._saved["create_connection"]
        return False  # never suppress exceptions (incl. OnlineCallError)


def proves_offline(fn):
    """Decorator: assert the wrapped function makes zero online calls, returning its
    result. Sugar over `with no_network(): ...` for one-liners and tests."""

    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        with no_network():
            return fn(*args, **kwargs)

    return wrapper
