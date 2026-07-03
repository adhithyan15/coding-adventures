"""IrcServer — the Python control surface for the Rust IRC engine.

``IrcServer`` is a thin facade over the ``irc_server_native`` C extension, which
in turn embeds the all-Rust ``irc-net-reactor`` engine.  Because every line of
IRC and TCP logic lives in Rust, this class only needs to *create*, *run*, and
*stop* the server — it never handles a message itself.

Typical usage::

    server = IrcServer(host="127.0.0.1", port=6667, server_name="irc.example")
    server.serve()          # blocks until another thread calls server.stop()

For tests, run ``serve()`` on a background thread and call ``stop()`` after
assertions; read the OS-assigned port with ``local_port()`` when binding to 0.
"""

from __future__ import annotations

from typing import Any


def _load_native() -> Any:
    """Import the compiled ``irc_server_native`` extension.

    Done lazily (not at module import time) so that ``import
    coding_adventures.irc_server_native`` succeeds on a machine without the
    compiled ``.so`` — the extension only has to exist by the time an
    :class:`IrcServer` is actually constructed (i.e. after the BUILD script ran).
    """
    try:
        from . import irc_server_native  # type: ignore[import]

        return irc_server_native
    except ImportError:  # pragma: no cover - env-specific fallback
        # Defensive: load the cdylib directly by path when the normal package
        # import misses it (its filename carries a platform/ABI tag).  Only one
        # of these two branches runs in any given environment, so this one is
        # excluded from coverage rather than chased with an unportable test.
        import importlib.util
        import os

        pkg_dir = os.path.dirname(__file__)
        candidate = next(
            (
                os.path.join(pkg_dir, f)
                for f in os.listdir(pkg_dir)
                if f.startswith("irc_server_native") and f.endswith(".so")
            ),
            None,
        )
        spec = importlib.util.spec_from_file_location(
            "irc_server_native", candidate or ""
        )
        if spec is None or spec.origin is None or spec.loader is None:
            raise ImportError(  # noqa: B904
                "irc_server_native extension not found. "
                "Run the BUILD script first to compile the Rust extension."
            )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module


class IrcServer:
    """A high-performance IRC server backed by the Rust ``irc-net-reactor`` engine.

    Parameters
    ----------
    host:
        Bind address. ``"127.0.0.1"`` is loopback; ``"0.0.0.0"`` listens on all
        interfaces (reachable from other machines — use deliberately).
    port:
        TCP port. ``0`` asks the OS for a free ephemeral port, then readable via
        :meth:`local_port`.
    server_name:
        Hostname advertised in the ``001`` welcome and as the message prefix.
    motd:
        Message of the Day lines. Defaults to ``["Welcome."]``.
    oper_password:
        Password for the ``OPER`` command. Empty string (default) disables OPER.
    max_connections:
        Maximum simultaneous connections.
    """

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 6667,
        server_name: str = "irc.local",
        motd: list[str] | None = None,
        oper_password: str = "",
        max_connections: int = 1024,
        *,
        _native: Any = None,
    ) -> None:
        # ``_native`` is an injection seam for tests: pass a fake module exposing
        # the same server_* functions to exercise this facade without the .so.
        native = _native if _native is not None else _load_native()
        motd_lines = list(motd) if motd else ["Welcome."]

        self._native = native
        self._capsule = native.server_new(
            str(host),
            int(port),
            str(server_name),
            [str(line) for line in motd_lines],
            str(oper_password),
            int(max_connections),
        )

    # ── Lifecycle ────────────────────────────────────────────────────────────

    def serve(self) -> None:
        """Run the event loop, blocking until :meth:`stop` is called.

        The Rust extension releases the GIL around the blocking loop, so another
        Python thread (or a signal handler) can call :meth:`stop`.
        """
        self._native.server_serve(self._capsule)

    def stop(self) -> None:
        """Signal the event loop to stop; a blocked :meth:`serve` returns.

        Safe to call from any thread, and idempotent.
        """
        self._native.server_stop(self._capsule)

    def dispose(self) -> None:
        """Release the server's listener now (the server must be stopped first).

        Disposing is optional — the engine is also freed when this object is
        garbage-collected — but it frees the bound port deterministically.
        """
        self._native.server_dispose(self._capsule)

    # ── Introspection ────────────────────────────────────────────────────────

    def running(self) -> bool:
        """Whether the event loop is currently running."""
        return bool(self._native.server_running(self._capsule))

    def local_host(self) -> str:
        """The bound IP address as a string."""
        return str(self._native.server_local_host(self._capsule))

    def local_port(self) -> int:
        """The bound TCP port (the OS-assigned port when constructed with ``port=0``).

        Read this after constructing with ``port=0`` to learn the chosen port.
        """
        return int(self._native.server_local_port(self._capsule))
