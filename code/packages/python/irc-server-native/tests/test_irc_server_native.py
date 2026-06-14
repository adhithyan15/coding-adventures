"""Tests for the Python IRC server binding.

Two layers:

* **Facade tests** drive ``IrcServer`` against a fake native module, exercising
  the Python wrapper logic (argument coercion, delegation) with no compiled
  extension required — so coverage holds even where the native smoke test skips.
* **End-to-end tests** start the real Rust engine on an ephemeral port and run
  two live IRC clients through registration and channel broadcast.  They skip
  cleanly if the compiled ``irc_server_native`` extension is not present.
"""

from __future__ import annotations

import socket
import threading
import time

import pytest

from coding_adventures.irc_server_native import IrcServer
from coding_adventures.irc_server_native.server import _load_native


def _native_available() -> bool:
    try:
        _load_native()
        return True
    except Exception:
        return False


NATIVE = _native_available()
requires_native = pytest.mark.skipif(
    not NATIVE, reason="irc_server_native extension not compiled"
)


# ───────────────────────────────────────────────────────────────────────────
# Facade tests — fake native module, no .so needed
# ───────────────────────────────────────────────────────────────────────────


class FakeNative:
    """Records calls so the facade's delegation and coercion can be asserted."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []
        self.capsule = object()
        self._running = False
        self.host = "127.0.0.1"
        self.port = 6667

    def server_new(self, host, port, server_name, motd, oper_password, max_connections):
        self.calls.append(
            ("new", host, port, server_name, motd, oper_password, max_connections)
        )
        return self.capsule

    def server_serve(self, capsule):
        self.calls.append(("serve", capsule))
        self._running = True

    def server_stop(self, capsule):
        self.calls.append(("stop", capsule))
        self._running = False

    def server_dispose(self, capsule):
        self.calls.append(("dispose", capsule))

    def server_running(self, capsule):
        self.calls.append(("running", capsule))
        return self._running

    def server_local_host(self, capsule):
        return self.host

    def server_local_port(self, capsule):
        return self.port


def test_new_coerces_arguments_and_defaults_motd():
    fake = FakeNative()
    IrcServer(host=0xC0A8, port="6667", server_name=123, _native=fake)  # type: ignore[arg-type]
    kind, host, port, name, motd, oper, maxc = fake.calls[0]
    assert kind == "new"
    assert host == str(0xC0A8)  # coerced to str
    assert port == 6667  # coerced to int
    assert name == "123"  # coerced to str
    assert motd == ["Welcome."]  # default applied
    assert oper == ""  # default empty
    assert maxc == 1024  # default


def test_new_passes_through_explicit_motd_and_oper():
    fake = FakeNative()
    IrcServer(
        motd=["one", "two"],
        oper_password="s3cret",
        max_connections=42,
        _native=fake,
    )
    _, _, _, _, motd, oper, maxc = fake.calls[0]
    assert motd == ["one", "two"]
    assert oper == "s3cret"
    assert maxc == 42


def test_lifecycle_methods_delegate_to_native():
    fake = FakeNative()
    server = IrcServer(_native=fake)
    assert server.running() is False
    server.serve()
    assert server.running() is True
    server.stop()
    assert server.running() is False
    server.dispose()
    assert ("serve", fake.capsule) in fake.calls
    assert ("stop", fake.capsule) in fake.calls
    assert ("dispose", fake.capsule) in fake.calls


def test_introspection_returns_native_values():
    fake = FakeNative()
    fake.host = "0.0.0.0"
    fake.port = 9999
    server = IrcServer(_native=fake)
    assert server.local_host() == "0.0.0.0"
    assert server.local_port() == 9999


# ───────────────────────────────────────────────────────────────────────────
# End-to-end tests — the real Rust engine over real sockets
# ───────────────────────────────────────────────────────────────────────────


def _recv_until(sock: socket.socket, needle: str, timeout: float = 5.0) -> str:
    sock.settimeout(0.3)
    deadline = time.time() + timeout
    buf = b""
    while time.time() < deadline:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            buf += chunk
            if needle.encode() in buf:
                break
        except TimeoutError:
            continue
    return buf.decode("utf-8", "replace")


def _connect(port: int) -> socket.socket:
    last: Exception | None = None
    for _ in range(40):
        try:
            return socket.create_connection(("127.0.0.1", port), timeout=1.0)
        except OSError as exc:  # noqa: PERF203
            last = exc
            time.sleep(0.01)
    raise AssertionError(f"could not connect to server: {last}")


def _register(sock: socket.socket, nick: str) -> None:
    sock.sendall(f"NICK {nick}\r\nUSER {nick} 0 * :{nick}\r\n".encode())
    welcome = _recv_until(sock, "001")
    assert "001" in welcome, f"expected 001 welcome for {nick}, got: {welcome!r}"


@pytest.fixture()
def running_server():
    server = IrcServer(host="127.0.0.1", port=0, server_name="irc.test")
    port = server.local_port()
    thread = threading.Thread(target=server.serve, daemon=True)
    thread.start()
    try:
        yield server, port
    finally:
        server.stop()
        thread.join(timeout=5)


@requires_native
def test_local_addr_reports_ephemeral_port():
    server = IrcServer(host="127.0.0.1", port=0)
    assert server.local_host() == "127.0.0.1"
    assert server.local_port() > 0
    server.dispose()


@requires_native
def test_registration_and_ping(running_server):
    _server, port = running_server
    alice = _connect(port)
    try:
        _register(alice, "alice")
        alice.sendall(b"PING :liveness\r\n")
        pong = _recv_until(alice, "PONG")
        assert "PONG" in pong, f"expected PONG, got: {pong!r}"
    finally:
        alice.close()


@requires_native
def test_privmsg_broadcasts_between_clients(running_server):
    _server, port = running_server
    alice = _connect(port)
    bob = _connect(port)
    try:
        _register(alice, "alice")
        _register(bob, "bob")
        alice.sendall(b"JOIN #test\r\n")
        bob.sendall(b"JOIN #test\r\n")
        _recv_until(alice, "JOIN")
        _recv_until(bob, "JOIN")

        # Alice speaks; Bob (a different connection) must receive it — this
        # exercises the Rust engine's in-process mailbox fan-out.
        alice.sendall(b"PRIVMSG #test :hello bob\r\n")
        received = _recv_until(bob, "hello bob")
        assert "PRIVMSG" in received and "hello bob" in received, (
            f"bob should receive alice's broadcast, got: {received!r}"
        )
    finally:
        alice.close()
        bob.close()


@requires_native
def test_running_flag_tracks_serve(running_server):
    server, _port = running_server
    # Give the background serve() a moment to flip the flag.
    deadline = time.time() + 5
    while not server.running() and time.time() < deadline:
        time.sleep(0.01)
    assert server.running() is True


@requires_native
def test_dispose_refused_while_running(running_server):
    # Disposing a live server must be refused — the native layer frees the engine
    # in dispose(), and allowing that mid-serve() would be a use-after-free.
    server, _port = running_server
    deadline = time.time() + 5
    while not server.running() and time.time() < deadline:
        time.sleep(0.01)
    assert server.running() is True
    with pytest.raises(RuntimeError):
        server.dispose()
